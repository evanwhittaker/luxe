//! Preview keys — the File view and the diff, while the keyboard is in them.
//!
//! `Ctrl+T` walks the keyboard from the composer to the FILES tree to here and back, and `Tab`
//! trades it between the tree and the pane. In the File view a cursor moves line by line, the way
//! the review's diff does, so a pick can be made from the keyboard as well as with a drag: `v` or
//! `V` is vim's visual-line mode, where every move grows the pick until `y`, `Enter` or `Esc`. In
//! the diff the cursor moves row by row and picks nothing. `Ctrl+D` and `Ctrl+U` move half a page
//! in either, the only chords the pane keeps; every other chord is the app's. A key the pane does
//! not know hands the keyboard back and goes to the composer, the same as the tree.

use crate::app::{App, Msg};
use crate::changes::PeekView;
use crate::mouse::PeekSelectionAction;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Lines past which a jump is the whole file, whichever way.
const EVERYTHING: i64 = i64::MAX / 4;

/// Answers whether the pane took `key`.
pub(super) fn peek_took(app: &mut App, key: KeyEvent) -> bool {
    if !app.changes.as_ref().is_some_and(|view| view.focused) {
        return false;
    }
    // Text dragged over grows by the character, ahead of the line pick the same keys make
    if crate::app::selection::peek_step_took(app, &key) {
        return true;
    }
    let Some(view) = app.changes.as_mut() else { return false };
    let page = i64::try_from(view.viewport_rows.get().saturating_sub(1).max(1)).unwrap_or(1);
    if key.modifiers == KeyModifiers::CONTROL {
        // vim's half-page moves, the one pair of chords the pane keeps for itself
        let half = (page / 2).max(1);
        let delta = match key.code {
            KeyCode::Char('d') => half,
            KeyCode::Char('u') => -half,
            _ => 0,
        };
        if delta != 0 {
            match view.view {
                PeekView::File => view.move_cursor(delta, false),
                PeekView::Diff => view.move_diff_cursor(delta),
            }
            return true;
        }
    }
    if key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
        // A chord is the app's, not the pane's; it passes through and the keyboard stays here
        return false;
    }
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    match key.code {
        KeyCode::Tab | KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
            view.focused = false;
            app.update(Msg::FocusFiles);
            return true;
        }
        KeyCode::Esc if view.visual || view.selection.is_some() => {
            view.visual = false;
            view.selection = None;
        }
        KeyCode::Esc if view.cells_picked() => view.cells = None,
        KeyCode::Esc => view.focused = false,
        KeyCode::Char('o') if view.visual => view.swap_pick_ends(),
        KeyCode::Char('o') => {
            app.update(Msg::DispatchUiAction(luxe_core::keymap::UiAction::ChangesOpenInEditor));
        }
        _ if view.view == PeekView::Diff => return diff_key(app, key, page),
        KeyCode::Down | KeyCode::Char('j') => view.move_cursor(1, shift),
        KeyCode::Up | KeyCode::Char('k') => view.move_cursor(-1, shift),
        KeyCode::Char('J') => view.move_cursor(1, true),
        KeyCode::Char('K') => view.move_cursor(-1, true),
        KeyCode::PageDown | KeyCode::Char(' ') => view.move_cursor(page, shift),
        KeyCode::PageUp => view.move_cursor(-page, shift),
        KeyCode::Home | KeyCode::Char('g') => view.move_cursor(-EVERYTHING, shift),
        KeyCode::End | KeyCode::Char('G') => view.move_cursor(EVERYTHING, shift),
        KeyCode::Char('v' | 'V') => view.toggle_visual(),
        KeyCode::Enter | KeyCode::Char('@') => {
            pick_cursor_line(app);
            app.update(Msg::PeekSelectionAction(PeekSelectionAction::Ask));
            // The question about those lines is typed next, and that happens in the composer
            if let Some(view) = app.changes.as_mut() {
                view.focused = false;
                view.visual = false;
            }
        }
        KeyCode::Char('y') => {
            let picked = app.changes.as_ref().is_some_and(|view| view.selection.is_some());
            pick_cursor_line(app);
            app.update(Msg::PeekSelectionAction(PeekSelectionAction::Copy));
            // A yank ends visual mode, as it does in vim, and leaves a plain pick where it was
            if let Some(view) = app.changes.as_mut() {
                if view.visual || !picked {
                    view.selection = None;
                }
                view.visual = false;
            }
        }
        // Anything else is the composer's, and so is the keyboard from here
        _ => {
            view.focused = false;
            return false;
        }
    }
    true
}

/// The diff's keys: its cursor moves through the rows, and the view follows it.
fn diff_key(app: &mut App, key: KeyEvent, page: i64) -> bool {
    let Some(view) = app.changes.as_mut() else { return false };
    let delta = match key.code {
        KeyCode::Down | KeyCode::Char('j') => 1,
        KeyCode::Up | KeyCode::Char('k') => -1,
        KeyCode::PageDown | KeyCode::Char(' ') => page,
        KeyCode::PageUp => -page,
        KeyCode::Home | KeyCode::Char('g') => -EVERYTHING,
        KeyCode::End | KeyCode::Char('G') => EVERYTHING,
        _ => {
            view.focused = false;
            return false;
        }
    };
    view.move_diff_cursor(delta);
    true
}

/// Makes the cursor's line the pick when nothing is picked, so a key that acts on a pick acts on
/// the line the reader is standing on.
fn pick_cursor_line(app: &mut App) {
    let Some(view) = app.changes.as_mut() else { return };
    if view.selection.is_none() {
        let at = view.cursor.unwrap_or_else(|| view.top_line());
        view.selection = Some((at, at));
    }
}

#[cfg(test)]
mod tests;
