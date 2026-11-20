//! Find's glue for what goes wrong, and for a click on its tabs.

use super::*;
use luxe_core::project_files::ProjectFileFailure;
use luxe_core::project_search::{SearchFailed, SearchFailure};

/// Opens Find on a search for `x` that answered one hit in `src/lib.rs`, and returns the read its
/// preview pane sent for that file.
fn previewing(
    app: &mut App,
    rx: &mut Receiver<Action>,
) -> luxe_core::project_files::FileReadRequestId {
    open(app);
    type_text(app, "x");
    let (request, _) = last_search(rx);
    app.update(Msg::Engine(answered(request, "src/lib.rs", 12)));
    reads(rx).pop().expect("the pane asked for the file").0
}

/// The break this catches: a failed search dropped, so Find sits on a spinner instead of saying the
/// regex is bad.
#[test]
fn a_failed_search_says_why() {
    let (mut app, mut rx) = app();
    open(&mut app);
    type_text(&mut app, "(");
    let (request_id, query) = last_search(&mut rx);
    let error = SearchFailure::InvalidPattern("unclosed group".into());
    app.update(Msg::Engine(Event::ProjectSearchFailed(SearchFailed { request_id, query, error })));
    let said = find(&app).error().unwrap_or_default();
    assert!(said.contains("not a valid regex"), "{said:?}");
}

/// The break this catches: a failed read for Find's pane passed on to the File view, so the pane
/// waits forever instead of saying the file is gone.
#[test]
fn a_failed_preview_read_shows_in_the_pane() {
    let (mut app, mut rx) = app();
    let request_id = previewing(&mut app, &mut rx);
    let path = "src/lib.rs".to_owned();
    let error = ProjectFileFailure::NotFound;
    app.update(Msg::Engine(Event::ProjectFileReadFailed { request_id, path, error }));
    let held = find(&app).previews().get("src/lib.rs");
    assert!(matches!(held, Some(crate::find::PreviewText::Failed(_))), "{held:?}");
}

/// The break this catches: a click on one of Find's tabs that changes nothing.
#[test]
fn a_click_on_a_tab_shows_it() {
    let (mut app, _rx) = app();
    open(&mut app);
    assert_eq!(find(&app).tab(), FindTab::Text);
    let strip = crate::mouse::TabStrip::Picker(crate::mouse::OverlayList::Find);
    app.update(Msg::OpenTab { strip, index: 1 });
    assert_eq!(find(&app).tab(), FindTab::Files);
}
