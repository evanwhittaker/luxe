//! Esc interrupt — a bare Esc stops a running turn, but only after everything else has had it.

use super::modes::mode_app;
use super::*;

fn esc(app: &mut App) {
    app.update(Msg::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
}

fn cancelled(rx: &mut mpsc::Receiver<Action>) -> bool {
    std::iter::from_fn(|| rx.try_recv().ok()).any(|action| matches!(action, Action::Cancel))
}

/// With a turn running and nothing else open, Esc is the interrupt.
///
/// The break this catches: the default binding stays `ctrl+esc` alone, which Windows never
/// delivers to a terminal.
#[tokio::test]
async fn esc_interrupts_a_running_turn() {
    let (mut app, mut rx) = mode_app();
    app.status.state = AppState::Streaming;
    esc(&mut app);
    assert!(cancelled(&mut rx), "Esc sent a cancel");
    assert!(matches!(app.status.state, AppState::Idle));
}

/// A draft is the composer's to clear first, so one Esc never both drops the draft and stops
/// the turn; the next press interrupts.
#[tokio::test]
async fn esc_clears_a_draft_before_it_interrupts() {
    let (mut app, mut rx) = mode_app();
    app.status.state = AppState::Streaming;
    app.editor.insert_text("half a steering message");
    esc(&mut app);
    assert_eq!(app.editor.current_input(), "");
    assert!(!cancelled(&mut rx), "the first press only cleared the draft");
    assert!(matches!(app.status.state, AppState::Streaming));
    esc(&mut app);
    assert!(cancelled(&mut rx), "the second press interrupts");
}

/// Closing the Changes pane takes the press, so dismissing a preview never stops the turn.
#[tokio::test]
async fn esc_that_closes_the_changes_pane_does_not_interrupt() {
    let (mut app, mut rx) = mode_app();
    app.status.state = AppState::Streaming;
    crate::fixture::seed_file_view(&mut app, "src/lib.rs", "line 1\nline 2\n", None);
    assert!(app.changes.is_some(), "the pane is open");
    esc(&mut app);
    assert!(app.changes.is_none(), "the pane closed");
    assert!(!cancelled(&mut rx), "and the turn kept running");
    assert!(matches!(app.status.state, AppState::Streaming));
}

/// With nothing running there is nothing to interrupt, and Esc keeps clearing the draft.
#[tokio::test]
async fn esc_when_idle_sends_no_cancel() {
    let (mut app, mut rx) = mode_app();
    app.editor.insert_text("a draft");
    esc(&mut app);
    assert_eq!(app.editor.current_input(), "");
    assert!(!cancelled(&mut rx), "an idle Esc is not an interrupt");
}

/// Ctrl+Esc still interrupts wherever a terminal delivers it, even over a draft.
#[tokio::test]
async fn ctrl_esc_still_interrupts() {
    let (mut app, mut rx) = mode_app();
    app.status.state = AppState::Streaming;
    app.editor.insert_text("draft");
    app.update(Msg::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::CONTROL)));
    assert!(cancelled(&mut rx));
}
