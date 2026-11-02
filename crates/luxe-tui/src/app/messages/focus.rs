//! Focus — which pane has the keyboard, and the two ways it moves between them.
//!
//! `Alt+←` and `Alt+→` step by where the panes are drawn: the FILES tree in the dock, the preview
//! beside it, then the conversation, mirrored when the dock is on the right. `Ctrl+T` walks the
//! same three in a loop, for a reader who'd rather not think about sides.

use crate::app::{App, Msg};
use luxe_core::config::{DockPosition, DockSectionName};

/// Somewhere the keyboard can be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pane {
    Files,
    Preview,
    Composer,
}

/// Reduces one focus message.
pub(super) fn reduce(app: &mut App, msg: Msg) {
    match msg {
        Msg::FocusPeek => focus_preview(app),
        Msg::WalkFocus => walk(app),
        Msg::StepFocus(rightward) => step(app, rightward),
        _ => {}
    }
}

fn focus_preview(app: &mut App) {
    app.files.focused = false;
    if let Some(view) = app.changes.as_mut() {
        view.focus();
    }
}

fn holding(app: &App) -> Pane {
    if app.changes.as_ref().is_some_and(|view| view.focused) {
        return Pane::Preview;
    }
    if app.files.focused {
        return Pane::Files;
    }
    Pane::Composer
}

/// The panes the keyboard can go to, left to right as they're drawn.
fn screen_order(app: &App) -> Vec<Pane> {
    let files = app.config.dock.shows(&DockSectionName::Files).then_some(Pane::Files);
    let preview = app.changes.is_some().then_some(Pane::Preview);
    let mut panes: Vec<Pane> =
        [files, preview, Some(Pane::Composer)].into_iter().flatten().collect();
    if app.dock.position == DockPosition::Right {
        panes.reverse();
    }
    panes
}

/// The keyboard to the pane beside it, or nowhere at the edge: a step that wrapped around would
/// land it on the far side of the screen from where the arrow pointed.
fn step(app: &mut App, rightward: bool) {
    let order = screen_order(app);
    let Some(at) = order.iter().position(|pane| *pane == holding(app)) else { return };
    let next = if rightward { at.checked_add(1) } else { at.checked_sub(1) };
    if let Some(&pane) = next.and_then(|next| order.get(next)) {
        focus(app, pane);
    }
}

/// The composer, then the tree, then the preview when one is open, then the composer again. A tree
/// the config hides is passed over for an open preview, and otherwise says why it can't be had.
fn walk(app: &mut App) {
    let files_shown = app.config.dock.shows(&DockSectionName::Files);
    let next = match holding(app) {
        Pane::Composer if !files_shown && app.changes.is_some() => Pane::Preview,
        Pane::Composer => Pane::Files,
        Pane::Files if app.changes.is_some() => Pane::Preview,
        Pane::Files | Pane::Preview => Pane::Composer,
    };
    focus(app, next);
}

fn focus(app: &mut App, pane: Pane) {
    match pane {
        Pane::Files => app.update(Msg::FocusFiles),
        Pane::Preview => focus_preview(app),
        Pane::Composer => {
            app.files.focused = false;
            if let Some(view) = app.changes.as_mut() {
                view.focused = false;
            }
        }
    }
}

#[cfg(test)]
mod tests;
