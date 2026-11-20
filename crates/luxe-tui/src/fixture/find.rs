//! Find fixture — the Find overlay with a text search answered and its first hit's file read.
//!
//! What a frame pin needs to show the preview pane drawing real code: rows to list, a query to
//! light, and the file around the first hit, without an engine to search or read.

use crate::app::App;
use crate::find::FindTab;
use luxe_core::project_files::{FileReadRequestId, ProjectFile};
use luxe_core::project_search::{
    MatchRange, SearchAnswer, SearchHit, SearchOptions, SearchRequestId, SearchResults,
};

/// The file the first hit is in, as the pane shows it, a line at a time so the indents survive.
const SOURCE: &[&str] = &[
    "use crate::theme::styles::ThemeStyles;",
    "",
    "/// Draws the overlay over `area`.",
    "pub(crate) fn render_overlay(frame: &mut Frame, area: Rect, theme: &ThemeStyles) {",
    "    let block = titled_panel(\"Find\", theme);",
    "    let inner = block.inner(area);",
    "    frame.render_widget(block, area);",
    "    render_rows(frame, inner, theme);",
    "}",
    "",
    "fn render_rows(frame: &mut Frame, area: Rect, theme: &ThemeStyles) {",
    "    for (offset, row) in rows().enumerate() {",
    "        frame.render_widget(row, line_at(area, offset));",
    "    }",
    "}",
];

/// Opens Find on the Text tab, searched for `render`, with three hits and the first one's file
/// read for the pane.
pub fn seed_find(app: &mut App) {
    crate::app::find::open(app);
    let Some(find) = app.overlays.find.as_mut() else { return };
    if find.tab() != FindTab::Text {
        find.show_tab(0);
    }
    find.paste("render");
    let request = SearchRequestId::new();
    find.searching(request.clone(), ("render".into(), SearchOptions::default()));
    find.accept(&SearchAnswer { request_id: request, results: results() });
    let read = FileReadRequestId::new();
    find.previews_mut().reading(read.clone(), "src/overlay.rs");
    find.previews_mut().accept(&read, &file());
}

fn results() -> SearchResults {
    let hit = |path: &str, line: usize, text: &str, start: usize| SearchHit {
        path: path.into(),
        line,
        text: text.into(),
        clipped: false,
        matches: vec![MatchRange { start, end: start + 6 }],
        scope: None,
    };
    let scoped = |hit: SearchHit, scope: &str| SearchHit { scope: Some(scope.into()), ..hit };
    let hits = vec![
        hit("src/overlay.rs", 4, "pub(crate) fn render_overlay(frame: &mut Frame) {", 14),
        scoped(
            hit("src/overlay.rs", 8, "    render_rows(frame, inner, theme);", 4),
            "render_overlay",
        ),
        scoped(hit("src/view/panel.rs", 31, "    frame.render_widget(block, area);", 10), "draw"),
    ];
    SearchResults { query: "render".into(), total: hits.len(), hits, ..SearchResults::default() }
}

fn file() -> ProjectFile {
    let text = SOURCE.join("\n");
    ProjectFile {
        path: "src/overlay.rs".into(),
        bytes: text.len() as u64,
        text,
        truncated: false,
        binary: false,
        image: None,
    }
}
