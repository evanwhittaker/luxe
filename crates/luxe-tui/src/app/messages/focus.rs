//! Focus — which pane has the keyboard, and how it moves between them.
//!
//! `Tab` (or `Ctrl+T`) walks the composer, the FILES tree and the preview in a loop, whichever pane
//! has the keyboard. Three stops at most, so going back is one or two more presses; `Shift+Tab`
//! stays the mode switch everywhere rather than meaning "back" in some panes and not others.

use crate::app::{App, Msg};
use luxe_core::config::DockSectionName;

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
