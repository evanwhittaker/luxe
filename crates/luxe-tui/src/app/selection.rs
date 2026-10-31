//! Text-selection handling — mouse drags over the transcript, the composer and the Changes pane.
//!
//! Mouse capture takes the terminal's own click-drag selection away, so luxe provides it: a press
//! anchors, a drag or Shift and an arrow extends, and the selection stays on screen until a copy
//! key or the right-click menu copies it, or Esc drops it. A press that lands twice on the same
//! cell escalates to the word under it and then to the whole line, the other two gestures capture
//! costs. The composer keeps its selection as buffer offsets inside [`crate::editor`]; the
//! transcript's lives on [`App`] in rendered-line coordinates and is served by the render pass,
//! which is the only place that knows the laid-out text. The Changes pane holds its own in source
//! rows ([`crate::changes::CellPick`]), because it has the source to cut a copy from.

use super::{App, ConversationSelection};
use crate::mouse::{TextHit, TextSurface};
use crate::selection::{GridSelection, SelectionSpan, TextPoint};

/// Starts a selection at `hit`, dropping any previous one.
///
/// `span` is how much of the text under the pointer the gesture claims. A word on the transcript is
/// the one thing this cannot resolve on its own — where a word ends is a fact about the rendered
/// line, which only the render pass holds — so it asks the next frame for it.
pub(super) fn begin(app: &mut App, hit: TextHit, span: SelectionSpan) {
    clear(app);
    app.pointer.selection_drag = Some(hit.surface);
    let (line, column) = (hit.point.line, hit.point.column);
    match hit.surface {
        TextSurface::Composer => match span {
            SelectionSpan::Caret => app.editor.begin_selection_at_visual(line, column),
            SelectionSpan::Word => app.editor.select_word_at_visual(line, column),
            SelectionSpan::Line => app.editor.select_visual_line(line),
        },
        // No pin means no way to follow the line through the next reflow, so there is nothing safe
        // to select. Unreachable in practice: every transcript row publishes one.
        TextSurface::Conversation => {
            let Some(pin) = hit.pin else { return };
            let grid = match span {
                SelectionSpan::Line => GridSelection::whole_line(line),
                _ => GridSelection::collapsed(TextPoint::new(line, column)),
            };
            app.pointer.expand_word = span == SelectionSpan::Word;
            app.pointer.selection = Some(ConversationSelection { grid, pins: (pin.clone(), pin) });
        }
        // A press in the pane is the reader turning to it, so the keyboard follows the pointer
        TextSurface::Peek(side) => {
            let Some(view) = app.changes.as_mut() else { return };
            view.begin_cells(hit.point, side, span);
            view.focus_on_row(hit.point.line);
            app.files.focused = false;
        }
    }
}

/// Extends the in-flight selection to `hit`. Ignores a drag that wandered onto a different surface
/// than the one the press started on.
pub(super) fn extend(app: &mut App, hit: TextHit) {
    if app.pointer.selection_drag != Some(hit.surface) {
        return;
    }
    let (line, column) = (hit.point.line, hit.point.column);
    match hit.surface {
        TextSurface::Composer => app.editor.extend_selection_to_visual(line, column),
        TextSurface::Conversation => {
            // A word or line grab that turns into a drag becomes an ordinary cell selection.
            app.pointer.expand_word = false;
            let Some(pin) = hit.pin else { return };
            if let Some(selection) = app.pointer.selection.as_mut() {
                selection.grid.cursor = TextPoint::new(line, column);
                selection.pins.1 = pin;
            }
        }
        TextSurface::Peek(_) => {
            if let Some(view) = app.changes.as_mut() {
                view.extend_cells(hit.point);
            }
        }
    }
}

/// Copies whichever surface holds a selection.
pub(super) fn copy(app: &mut App) {
    if app.editor.selection().is_some() {
        copy_composer(app);
    } else if covers_cells(app) {
        copy_conversation(app);
    } else if let Some(text) = app.changes.as_ref().and_then(|view| view.cells_text()) {
        if !text.is_empty() {
            app.record_copy(text);
        }
    }
}

/// Drops every selection (Esc, or a press that starts a fresh one).
pub(super) fn clear(app: &mut App) {
    app.pointer.selection = None;
    app.pointer.expand_word = false;
    app.editor.clear_selection();
    if let Some(view) = app.changes.as_mut() {
        view.cells = None;
    }
}

/// Whether anything is selected on any surface.
pub(super) fn is_active(app: &App) -> bool {
    app.editor.selection().is_some()
        || covers_cells(app)
        || app.changes.as_ref().is_some_and(crate::changes::ChangesView::cells_picked)
}

/// Whether the transcript selection covers cells — i.e. a drag happened, not a bare click.
///
/// A pending word grab counts: the cells are real, the frame that can measure them just hasn't run.
pub(super) fn covers_cells(app: &App) -> bool {
    app.pointer.expand_word
        || app.pointer.selection.as_ref().is_some_and(|selection| !selection.grid.is_empty())
}

/// Whether a live text selection took `key`: Esc drops it before anything else claims the key, and
/// Shift and an arrow grow a transcript or Changes pane selection rather than the composer's.
/// Without a selection both fall through, Esc to the composer, where it clears the draft, and
/// Shift and an arrow to the editor, which answers them over its own selection.
pub(super) fn key_took(app: &mut App, key: &crossterm::event::KeyEvent) -> bool {
    let esc = key.code == crossterm::event::KeyCode::Esc && key.modifiers.is_empty();
    if esc && is_active(app) {
        app.update(super::Msg::ClearTextSelection);
        return true;
    }
    match step_for(key) {
        Some(step) if covers_cells(app) => {
            app.pointer.selection_steps.push(step);
            true
        }
        _ => peek_step_took(app, key),
    }
}

/// Whether Shift and an arrow, Home or End grew the Changes pane's character selection. The pane
/// asks first while it has the keyboard, where the same keys would otherwise grow its line pick.
pub(super) fn peek_step_took(app: &mut App, key: &crossterm::event::KeyEvent) -> bool {
    let Some(step) = step_for(key) else { return false };
    app.changes.as_mut().is_some_and(|view| view.step_cells(step))
}

/// The step Shift and an arrow, Home or End asks of a selection's free end; any other key, or one
/// with more held than Shift, asks none.
fn step_for(key: &crossterm::event::KeyEvent) -> Option<crate::selection::SelectionStep> {
    use crate::selection::SelectionStep;
    use crossterm::event::{KeyCode, KeyModifiers};
    if key.modifiers != KeyModifiers::SHIFT {
        return None;
    }
    match key.code {
        KeyCode::Left => Some(SelectionStep::Left),
        KeyCode::Right => Some(SelectionStep::Right),
        KeyCode::Up => Some(SelectionStep::Up),
        KeyCode::Down => Some(SelectionStep::Down),
        KeyCode::Home => Some(SelectionStep::LineStart),
        KeyCode::End => Some(SelectionStep::LineEnd),
        _ => None,
    }
}

fn copy_composer(app: &mut App) {
    if let Some(text) = app.editor.selected_text().filter(|text| !text.is_empty()) {
        app.record_copy(text);
    }
}

/// Asks the render pass to extract the covered cells — the transcript's plain text only exists
/// there, in the frame that laid it out.
fn copy_conversation(app: &mut App) {
    app.pointer.copy_requested = true;
}
