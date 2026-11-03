use super::{holding, Pane};
use crate::app::{App, Msg, ViewMode};
use crate::fixture::{seed_file_view, seeded_app, FixtureScenario};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use luxe_core::Action;
use tokio::sync::mpsc;

fn chatting() -> App {
    seeded_app("nord", FixtureScenario::ChatIdle).expect("chat fixture")
}

fn previewing() -> App {
    let mut app = chatting();
    seed_file_view(&mut app, "src/lib.rs", "one\ntwo\n", None);
    app
}

/// A previewing app whose actions can be read, for the keys that reach the engine.
fn previewing_with_actions() -> (App, mpsc::Receiver<Action>) {
    let (tx, rx) = mpsc::channel(16);
    let keymap = luxe_core::keymap::KeyMap::defaults();
    let mut app = App::new(tx, keymap, luxe_core::config::Config::default());
    app.dashboard = None;
    app.current_view = ViewMode::Chat;
    seed_file_view(&mut app, "src/lib.rs", "one\ntwo\n", None);
    (app, rx)
}

fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    app.update(Msg::Key(KeyEvent::new(code, modifiers)));
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

/// The break this catches: Tab doing nothing from the composer, the tree sending it straight back
/// to the composer past an open preview, the preview handing it back to the tree, or Tab landing
/// in the draft.
#[test]
fn tab_walks_every_pane_and_types_nothing() {
    let mut app = previewing();
    app.editor.insert_text("half a thought");
    let mut seen = Vec::new();
    for _ in 0..3 {
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        seen.push(holding(&app));
    }
    assert_eq!(seen, [Pane::Files, Pane::Preview, Pane::Composer]);
    assert_eq!(app.editor.current_input(), "half a thought");
}

/// The break this catches: shift+Tab still trading the keyboard between the tree and the preview,
/// or handing it to the composer, instead of switching the mode where the keyboard is.
#[test]
fn shift_tab_switches_the_mode_from_the_tree_and_the_preview() {
    let (mut app, mut rx) = previewing_with_actions();
    for pane in [Pane::Files, Pane::Preview] {
        app.update(Msg::WalkFocus);
        assert_eq!(holding(&app), pane);
        press(&mut app, KeyCode::BackTab, KeyModifiers::SHIFT);
        assert_eq!(holding(&app), pane, "the keyboard stayed put");
        assert!(matches!(rx.try_recv(), Ok(Action::SetMode(_))), "from {pane:?}");
    }
}

/// The break this catches: a review losing Alt+Left, its way back to chat.
#[test]
fn alt_left_still_leaves_a_review() {
    let mut app = seeded_app("nord", FixtureScenario::GuidedReviewReady).expect("review");
    assert_eq!(app.current_view, ViewMode::GuidedReview);
    press(&mut app, KeyCode::Left, KeyModifiers::ALT);
    assert_eq!(app.current_view, ViewMode::Chat);
}
