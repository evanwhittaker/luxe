//! `e`, `x` and `r` from the diff: they act on the comment drawn under the cursor's line.

use crate::app::{App, Msg};
use crate::fixture::{seeded_app, FixtureScenario};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use luxe_core::engine::{Action, ReviewEdit};
use luxe_core::guided_review::FindingId;
use tokio::sync::mpsc;

fn press(app: &mut App, code: KeyCode) {
    app.update(Msg::Key(KeyEvent::new(code, KeyModifiers::NONE)));
}

/// The findings review with its window shut, the cursor on the first file's first line, and the
/// unsent draft anchored there. The window's selection is left on a comment somewhere else.
fn draft_under_cursor() -> (App, mpsc::Receiver<Action>, FindingId) {
    let mut app = seeded_app("nord", FixtureScenario::GuidedReviewFindings).expect("fixture app");
    let (engine_tx, engine_rx) = mpsc::channel(16);
    app.engine_tx = engine_tx;
    let review = app.guided_review.as_ref().expect("review");
    let (file, hunk) = (&review.files[0], &review.files[0].hunks[0]);
    let workspace = &mut app.guided_review_workspace;
    workspace.findings_visible = false;
    workspace.position.file_id = Some(file.id.clone());
    workspace.position.hunk_id = Some(hunk.id.clone());
    workspace.position.line_offset = 1;
    let anchor = workspace.selected_finding_anchor(&review.files);
    let session = app.review.session_mut().expect("session");
    session.findings[2].anchor = anchor;
    let draft = session.findings[2].id.clone();
    app.guided_review_workspace.selected_finding_id = Some(session.findings[0].id.clone());
    (app, engine_rx, draft)
}

/// The break this catches: `x` deleting the feedback window's last selection, a comment the reader
/// cannot see from the diff, instead of the one under the line they are on.
#[test]
fn x_in_the_diff_deletes_the_comment_on_the_cursor_line() {
    let (mut app, mut engine_rx, draft) = draft_under_cursor();
    press(&mut app, KeyCode::Char('x'));
    let Ok(Action::EditReview { edit, .. }) = engine_rx.try_recv() else {
        panic!("x asks the desk to delete a comment");
    };
    assert_eq!(edit, ReviewEdit::DeleteComment { finding_id: draft });
}

/// The break this catches: a line with no comment falling back to whatever the window selected
/// last, and deleting that.
#[test]
fn x_on_a_line_with_no_comment_deletes_nothing() {
    let (mut app, mut engine_rx, _draft) = draft_under_cursor();
    app.guided_review_workspace.position.line_offset = 0;
    press(&mut app, KeyCode::Char('x'));
    assert!(engine_rx.try_recv().is_err(), "nothing is deleted");
    let toast = app.status.toast.as_ref().map(|(text, _)| text.as_str());
    assert_eq!(toast, Some("No comment on this line \u{b7} alt+c lists them all"));
}

/// Right-clicks a diff row as the event loop would, the row's click region standing where the
/// frame would have drawn it.
fn right_click_row(app: &mut App, row: usize) {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let region = ratatui::layout::Rect::new(0, 5, 80, 1);
    app.hit.borrow_mut().push(region, crate::mouse::MouseTarget::ReviewDiffRow(row));
    let kind = MouseEventKind::Down(MouseButton::Right);
    let click = MouseEvent { kind, column: 10, row: 5, modifiers: KeyModifiers::NONE };
    crate::main_loop::route_mouse(app, &click);
}

fn menu_labels(app: &App) -> Vec<String> {
    let items = app.overlays.context_menu.as_ref().map(|menu| menu.items.clone());
    items.unwrap_or_default().into_iter().map(|item| item.label).collect()
}

/// The break this catches: a right-click on a comment's line offering nothing, or offering to
/// delete a different comment than the one drawn there.
#[test]
fn right_clicking_a_comment_line_offers_to_delete_that_comment() {
    let (mut app, mut engine_rx, draft) = draft_under_cursor();
    app.guided_review_workspace.position.line_offset = 0;
    right_click_row(&mut app, 1);
    assert_eq!(app.guided_review_workspace.position.line_offset, 1, "the cursor moves there");
    let labels = menu_labels(&app);
    let expected = ["Edit the comment", "Delete the comment", "Show in the feedback window"];
    assert_eq!(labels[..3], expected, "{labels:?}");
    let delete = labels.iter().position(|label| label == "Delete the comment");
    app.update(Msg::ChooseMenuItem(delete.expect("a delete item")));
    let Ok(Action::EditReview { edit, .. }) = engine_rx.try_recv() else {
        panic!("choosing it asks the desk to delete a comment");
    };
    assert_eq!(edit, ReviewEdit::DeleteComment { finding_id: draft });
}

/// The break this catches: a line with no comment offering comment actions it cannot carry out.
#[test]
fn right_clicking_a_bare_line_offers_to_comment_or_ask() {
    let (mut app, _engine_rx, _draft) = draft_under_cursor();
    right_click_row(&mut app, 0);
    assert_eq!(menu_labels(&app), ["Comment here", "Ask the agent about this"]);
}

/// The break this catches: `e` opening the window's selection for editing rather than the comment
/// under the cursor.
#[test]
fn e_in_the_diff_edits_the_comment_on_the_cursor_line() {
    let (mut app, _engine_rx, draft) = draft_under_cursor();
    press(&mut app, KeyCode::Char('e'));
    assert_eq!(app.guided_review_workspace.editing_finding_id, Some(draft));
}
