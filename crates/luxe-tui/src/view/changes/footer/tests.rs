use super::toward_the_preview;
use crate::fixture::{seeded_app, FixtureScenario};
use luxe_core::config::DockPosition;

fn arrow(position: DockPosition, tree_focused: bool) -> String {
    let mut app = seeded_app("nord", FixtureScenario::ChatIdle).expect("chat fixture");
    app.dock.position = position;
    app.files.focused = tree_focused;
    toward_the_preview(&app)
}

/// The break this catches: the footer naming an arrow that steps the keyboard away from the
/// preview, from the tree or the conversation, on either side of the screen.
#[test]
fn the_footer_names_the_arrow_that_reaches_the_preview() {
    assert_eq!(arrow(DockPosition::Left, false), "alt+left", "from the conversation");
    assert_eq!(arrow(DockPosition::Left, true), "alt+right", "from a tree on the left");
    assert_eq!(arrow(DockPosition::Right, true), "alt+left", "from a tree on the right");
    assert_eq!(arrow(DockPosition::Unknown, true), "alt+right", "unknown draws on the left");
}
