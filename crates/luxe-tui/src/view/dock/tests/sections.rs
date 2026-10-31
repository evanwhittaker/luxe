//! Sections — `dock.sections` hides and orders the dock's sections within their regions.

use super::plan::checklist;
use super::*;
use luxe_core::workspace::{ProjectIdentity, WorkspaceScopeId, WorkspaceSnapshot};
use luxe_core::TodoSummaryStatus::Pending;

pub(super) fn app_with(sections: &[&str]) -> App {
    let mut app = app();
    app.config.dock.sections = Some(sections.iter().map(ToString::to_string).collect());
    app.status.context_window = 200_000;
    app
}

/// A project with a checklist, so PLAN and FILES both have something to draw.
fn model() -> DockSectionModel {
    let snapshot = WorkspaceSnapshot::empty(
        WorkspaceScopeId::new("dock-sections"),
        ProjectIdentity {
            root: "/work/luxe".into(),
            display_name: "luxe".into(),
            display_path: "~/work/luxe".into(),
            git_branch: Some("main".into()),
            git_head: None,
            git_dirty: false,
        },
    );
    let mut model = DockSectionModel::from_snapshot(&snapshot);
    model.plan = Some(checklist(&[Pending, Pending]));
    model
}

/// Each region's lines, as text.
pub(super) fn regions(app: &App) -> (Vec<String>, Vec<String>, Vec<String>) {
    regions_as(app, true, DockPresentation::SideBySide)
}

/// The regions of a dock over the conversation, with the statusline showing beneath it.
pub(super) fn overlay_regions(app: &App) -> (Vec<String>, Vec<String>, Vec<String>) {
    regions_as(app, false, DockPresentation::Overlay)
}

fn regions_as(
    app: &App,
    show_telemetry: bool,
    presentation: DockPresentation,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let (header, middle, footer, _) = region_lines(&model(), app, 60, show_telemetry, presentation);
    let texts = |lines: Vec<Line<'static>>| lines.iter().map(text).collect::<Vec<_>>();
    (texts(header), texts(middle), texts(footer))
}

pub(super) fn position(rows: &[String], heading: &str) -> Option<usize> {
    rows.iter().position(|row| row.contains(heading))
}

/// Whether `first` and `second` are both drawn, `first` above.
pub(super) fn above(rows: &[String], first: &str, second: &str) -> bool {
    matches!((position(rows, first), position(rows, second)), (Some(a), Some(b)) if a < b)
}

#[test]
fn a_section_left_out_is_not_drawn() {
    let app = app_with(&["project", "plan", "model", "session"]);
    let (header, middle, footer) = regions(&app);
    assert!(position(&header, "PROJECT").is_some());
    assert_eq!(position(&middle, "FILES"), None, "{middle:?}");
    assert!(position(&middle, "PLAN").is_some(), "{middle:?}");
    assert!(position(&footer, "SESSION").is_some(), "{footer:?}");
}

#[test]
fn leaving_out_project_empties_the_header() {
    let (header, ..) = regions(&app_with(&["files", "model"]));
    assert!(header.is_empty(), "{header:?}");
}

#[test]
fn the_middle_follows_the_list_order() {
    let (_, middle, _) = regions(&app_with(&["files", "plan"]));
    assert!(above(&middle, "FILES", "PLAN"), "{middle:?}");
    let (_, middle, _) = regions(&app_with(&["plan", "files"]));
    assert!(above(&middle, "PLAN", "FILES"), "{middle:?}");
}

#[test]
fn the_footer_follows_the_list_order() {
    let (.., footer) = regions(&app_with(&["session", "model"]));
    assert!(above(&footer, "SESSION", "MODEL"), "{footer:?}");
}

/// The provider, model and mode rows stay clickable wherever MODEL lands in the footer.
#[test]
fn the_model_rows_stay_clickable_after_reordering() {
    let app = app_with(&["session", "model"]);
    let (footer, rows) = footer_lines(&app, true, DockPresentation::SideBySide);
    let targets: Vec<String> = rows.iter().map(|(row, _)| text(&footer[*row])).collect();
    assert!(targets[0].starts_with("provider"), "{targets:?}");
    assert!(targets[1].starts_with("model"), "{targets:?}");
    assert!(targets[2].starts_with("mode"), "{targets:?}");
}

#[test]
fn hiding_model_leaves_nothing_to_click() {
    let app = app_with(&["session"]);
    let (footer, rows) = footer_lines(&app, true, DockPresentation::SideBySide);
    assert!(rows.is_empty(), "{rows:?}");
    assert!(!footer.iter().map(text).any(|row| row.contains("MODEL")));
}

/// A session approving its own escalations can't drop out of sight through a config list.
#[test]
fn hiding_model_keeps_auto_approve_on_screen() {
    let mut app = app_with(&["session"]);
    app.permission_auto_approve = true;
    let rows: Vec<String> = telemetry_lines(&app).iter().map(text).collect();
    let approvals = rows.iter().position(|row| row.starts_with("approvals"));
    assert!(above(&rows, "MODEL", "approvals"), "under a MODEL heading: {rows:?}");
    assert!(approvals.is_some(), "{rows:?}");
    assert!(!rows.iter().any(|row| row.starts_with("provider")), "the rest stays hidden");
}

/// `ctrl+t` into a tree the config hides would put the keyboard somewhere nobody can see.
#[test]
fn the_tree_key_says_files_is_hidden_instead_of_focusing_it() {
    let mut app = app_with(&["project", "model"]);
    app.update(crate::app::Msg::FocusFiles);
    assert!(!app.files.focused, "the keyboard stays put");
    assert_eq!(app.status.toast(), Some("FILES is hidden by dock.sections"));

    let mut shown = app_with(&["files"]);
    shown.update(crate::app::Msg::FocusFiles);
    assert!(shown.files.focused, "a shown tree takes the keyboard");
}

/// A name the dock has no section for is skipped, and the rest still draw.
#[test]
fn an_unknown_name_is_skipped() {
    let (_, middle, footer) = regions(&app_with(&["files", "quota", "model"]));
    assert!(position(&middle, "FILES").is_some(), "{middle:?}");
    assert!(position(&footer, "MODEL").is_some(), "{footer:?}");
}
