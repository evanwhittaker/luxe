//! Find over a Guided Review: drawn there, and a hit on a line the review shows goes to it.

use super::*;
use crate::app::ViewMode;
use crate::fixture::{seeded_session, FixtureScenario};

fn review_app() -> (App, Receiver<Action>) {
    let (mut app, rx) =
        seeded_session("nord", FixtureScenario::GuidedReviewReady).expect("fixture");
    app.update(Msg::TerminalResized { width: 140, height: 40 });
    assert_eq!(app.current_view, ViewMode::GuidedReview);
    (app, rx)
}

/// The last new-side line the review shows, as a path, a line, and the hunk holding it: away from
/// where the review opens, so going there is a move.
fn shown_line(app: &App) -> (String, usize, luxe_core::guided_review::HunkId) {
    let review = app.guided_review.as_deref().expect("a review");
    review
        .files
        .iter()
        .filter_map(|file| Some((file.new_path.as_ref()?, file)))
        .flat_map(|(path, file)| file.hunks.iter().map(move |hunk| (path, hunk)))
        .rev()
        .find_map(|(path, hunk)| {
            let line = hunk.lines.iter().find_map(|source| source.new_line)?;
            Some((luxe_core::paths::display(path), line as usize, hunk.id.clone()))
        })
        .expect("a line the review shows")
}

fn ctrl_f(app: &mut App) {
    press(app, KeyCode::Char('f'), KeyModifiers::CONTROL);
}

/// Opens Find in the review and gives it one text hit at `path:line`.
fn hit_in_review(app: &mut App, rx: &mut Receiver<Action>, path: &str, line: usize) {
    ctrl_f(app);
    type_text(app, "x");
    let (request_id, _) = last_search(rx);
    app.update(Msg::Engine(answered(request_id, path, line)));
}

fn screen(app: &mut App) -> String {
    let backend = ratatui::backend::TestBackend::new(140, 40);
    let mut terminal = ratatui::Terminal::new(backend).expect("terminal");
    terminal.draw(|frame| crate::view::render(frame, app)).expect("a frame");
    let buffer = terminal.backend().buffer();
    buffer.content.iter().map(ratatui::buffer::Cell::symbol).collect()
}

/// The break this catches: `ctrl+f` in a review taken for the review's `f` (the noise filter), or
/// opening a Find the review never draws, which then swallows every key unseen.
#[test]
fn ctrl_f_in_a_review_opens_find_over_it() {
    let (mut app, _rx) = review_app();
    let folds = format!("{:?}", app.guided_review_workspace.fold);
    ctrl_f(&mut app);
    assert!(app.overlays.find.is_some(), "Find opens");
    let after = format!("{:?}", app.guided_review_workspace.fold);
    assert_eq!(after, folds, "the noise filter is untouched");
    assert!(screen(&mut app).contains("Search:"), "and is drawn over the review");
}

/// The break this catches: a hit on a line the review shows sending the reader out of the review
/// to the File view, or staying in the review without moving to the line.
#[test]
fn a_hit_on_a_line_the_review_shows_goes_to_it() {
    let (mut app, mut rx) = review_app();
    let (path, line, hunk) = shown_line(&app);
    let start = app.guided_review_workspace.position.hunk_id.clone();
    assert_ne!(start, Some(hunk.clone()), "the review opens elsewhere");
    hit_in_review(&mut app, &mut rx, &path, line);
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.overlays.find.is_none());
    assert_eq!(app.current_view, ViewMode::GuidedReview, "still reviewing");
    assert_eq!(app.guided_review_workspace.position.hunk_id, Some(hunk), "on the hit's hunk");
}

/// The break this catches: a hit the review doesn't show going nowhere, or opening a File view the
/// review screen hides.
#[test]
fn a_hit_outside_the_diff_opens_the_file_view() {
    let (mut app, mut rx) = review_app();
    let (path, _, _) = shown_line(&app);
    hit_in_review(&mut app, &mut rx, &path, 999_999);
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(app.current_view, ViewMode::Chat, "out of the review");
    assert!(app.changes.is_some(), "into the File view");
}

/// The break this catches: a mention typed into a draft the review screen hides.
#[test]
fn a_mention_from_a_review_lands_in_a_draft_on_screen() {
    let (mut app, mut rx) = review_app();
    let (path, line, _) = shown_line(&app);
    hit_in_review(&mut app, &mut rx, &path, line);
    press(&mut app, KeyCode::Char('m'), KeyModifiers::ALT);
    assert_eq!(app.current_view, ViewMode::Chat);
    assert!(app.editor.current_input().contains(&path), "{}", app.editor.current_input());
}

/// The break this catches: `ctrl+f`, or `/find` from the start screen's command list, opening Find
/// where nothing draws it, so the keys after it vanish.
#[test]
fn find_on_the_start_screen_is_drawn_or_not_open() {
    let (mut app, _rx) = seeded_session("nord", FixtureScenario::Dashboard).expect("fixture");
    app.update(Msg::TerminalResized { width: 140, height: 40 });
    let key = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL);
    match crate::main_loop::route_key(&mut app, &key) {
        crate::main_loop::RouteAction::Msg(message) => app.update(*message),
        crate::main_loop::RouteAction::Dashboard(outcome) => app.handle_dashboard_outcome(outcome),
        _ => {}
    }
    let open = app.overlays.find.is_some();
    assert_eq!(open, screen(&mut app).contains("Search:"), "ctrl+f");
    app.dispatch_action(luxe_core::keymap::UiAction::OpenFind);
    let open = app.overlays.find.is_some();
    assert_eq!(open, screen(&mut app).contains("Search:"), "the command");
}
