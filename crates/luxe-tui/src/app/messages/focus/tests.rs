use super::{holding, Pane};
use crate::app::{App, Msg};
use crate::fixture::{seed_file_view, seeded_app, FixtureScenario};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use luxe_core::config::DockPosition;

fn chatting() -> App {
    seeded_app("nord", FixtureScenario::ChatIdle).expect("chat fixture")
}

fn previewing() -> App {
    let mut app = chatting();
    seed_file_view(&mut app, "src/lib.rs", "one\ntwo\n", None);
    app
}

fn alt(app: &mut App, code: KeyCode) {
    app.update(Msg::Key(KeyEvent::new(code, KeyModifiers::ALT)));
}

/// The break this catches: a step that skips the preview, wraps around at the edge, or goes the
/// way the arrow doesn't point.
#[test]
fn steps_follow_the_panes_left_to_right_and_stop_at_the_edges() {
    let mut app = previewing();
    app.update(Msg::StepFocus(true));
    assert_eq!(holding(&app), Pane::Composer, "nothing right of the conversation");
    app.update(Msg::StepFocus(false));
    assert_eq!(holding(&app), Pane::Preview);
    app.update(Msg::StepFocus(false));
    assert_eq!(holding(&app), Pane::Files);
    app.update(Msg::StepFocus(false));
    assert_eq!(holding(&app), Pane::Files, "nothing left of the dock");
    app.update(Msg::StepFocus(true));
    assert_eq!(holding(&app), Pane::Preview);
    app.update(Msg::StepFocus(true));
    assert_eq!(holding(&app), Pane::Composer);
}

/// The break this catches: the arrows keeping the left-dock order with the dock on the right, so
/// Alt+Left from the conversation lands on a pane to its right.
#[test]
fn a_dock_on_the_right_mirrors_the_steps() {
    let mut app = previewing();
    app.dock.position = DockPosition::Right;
    app.update(Msg::StepFocus(false));
    assert_eq!(holding(&app), Pane::Composer, "nothing left of the conversation");
    app.update(Msg::StepFocus(true));
    assert_eq!(holding(&app), Pane::Preview);
    app.update(Msg::StepFocus(true));
    assert_eq!(holding(&app), Pane::Files);
}

/// The break this catches: a step into a preview that isn't open, which strands the keyboard on
/// nothing.
#[test]
fn without_a_preview_the_step_goes_straight_to_the_tree() {
    let mut app = chatting();
    app.update(Msg::StepFocus(false));
    assert_eq!(holding(&app), Pane::Files);
}

/// The break this catches: a step into a tree the config hides.
#[test]
fn a_hidden_tree_is_no_stop() {
    let mut app = previewing();
    app.config.dock.sections = Some(vec!["plan".to_string()]);
    app.update(Msg::StepFocus(false));
    app.update(Msg::StepFocus(false));
    assert_eq!(holding(&app), Pane::Preview, "the preview is the leftmost stop");
}

/// The break this catches: Ctrl+T's walk looping between the tree and the preview with no way
/// back to the composer, or visiting a preview that isn't open.
#[test]
fn the_walk_comes_back_to_the_composer() {
    let mut app = previewing();
    let mut seen = Vec::new();
    for _ in 0..3 {
        app.update(Msg::WalkFocus);
        seen.push(holding(&app));
    }
    assert_eq!(seen, [Pane::Files, Pane::Preview, Pane::Composer]);

    let mut app = chatting();
    app.update(Msg::WalkFocus);
    app.update(Msg::WalkFocus);
    assert_eq!(holding(&app), Pane::Composer, "no preview to visit");
}

/// The break this catches: a walk stuck on a tree the config hides, the preview beyond it never
/// reached.
#[test]
fn the_walk_passes_a_hidden_tree_for_the_preview() {
    let mut app = previewing();
    app.config.dock.sections = Some(vec!["plan".to_string()]);
    app.update(Msg::WalkFocus);
    assert_eq!(holding(&app), Pane::Preview);
}

/// The break this catches: the pane move taking Alt+Left in a review too, which strands a reader
/// who learned it as the way back to chat.
#[test]
fn alt_left_still_leaves_a_review() {
    let mut app = seeded_app("nord", FixtureScenario::GuidedReviewReady).expect("review");
    assert_eq!(app.current_view, crate::app::ViewMode::GuidedReview);
    alt(&mut app, KeyCode::Left);
    assert_eq!(app.current_view, crate::app::ViewMode::Chat);
}

/// The break this catches: Alt+Left still jumping a word in the composer, the review's way back
/// to chat winning the chord in chat, or the composer losing Ctrl+Left's word jump.
#[test]
fn alt_arrows_move_the_keyboard_in_chat_and_ctrl_arrows_still_jump_words() {
    let mut app = previewing();
    app.editor.insert_text("one two");
    alt(&mut app, KeyCode::Left);
    assert_eq!(holding(&app), Pane::Preview);
    assert_eq!(app.editor.cursor(), "one two".len(), "the draft's cursor stayed put");
    alt(&mut app, KeyCode::Left);
    assert_eq!(holding(&app), Pane::Files, "the preview passes the key on");
    alt(&mut app, KeyCode::Right);
    assert_eq!(holding(&app), Pane::Preview, "so does the tree");
    alt(&mut app, KeyCode::Right);
    assert_eq!(holding(&app), Pane::Composer);
    app.update(Msg::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL)));
    assert_eq!(app.editor.cursor(), "one ".len());
}
