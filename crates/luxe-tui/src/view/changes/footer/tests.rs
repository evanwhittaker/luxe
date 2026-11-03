use super::shortcuts;
use crate::fixture::{seed_file_view, seeded_app, FixtureScenario};
use std::collections::HashMap;

fn navigate_key(app: &crate::app::App) -> String {
    let view = app.changes.as_ref().expect("a file showing");
    let found =
        shortcuts(app, view, false).into_iter().find(|shortcut| shortcut.label == "navigate");
    found.expect("the navigate shortcut").chord
}

/// The break this catches: the footer naming a key that no longer walks the panes, by default or
/// once the walk is rebound.
#[test]
fn the_footer_names_the_key_that_walks_the_panes() {
    let mut app = seeded_app("nord", FixtureScenario::ChatIdle).expect("chat fixture");
    seed_file_view(&mut app, "src/lib.rs", "one\n", None);
    assert_eq!(navigate_key(&app), "tab");
    let rebound =
        HashMap::from([("tui.workspace.focus_files".to_string(), vec!["ctrl+t".to_string()])]);
    app.keymap = app.keymap.clone().with_overrides(rebound);
    assert_eq!(navigate_key(&app), "ctrl+t");
}
