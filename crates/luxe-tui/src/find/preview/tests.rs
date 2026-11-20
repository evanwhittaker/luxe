use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use luxe_core::project_files::{FileReadRequestId, ProjectFile, ProjectFileFailure};
use luxe_core::project_search::{SearchAnswer, SearchHit, SearchRequestId, SearchResults};

fn theme() -> ThemeStyles {
    let resolved = luxe_core::theme::ThemeRegistry::new().resolve("nord").expect("nord");
    ThemeStyles::from_resolved(&resolved, "nord")
}

/// Forty numbered lines, `total` on line 20.
fn source() -> String {
    (1..=40)
        .map(|n| if n == 20 { "let total = 1;".to_owned() } else { format!("line {n}") })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A Text tab that found `total` on `line` of `path`, with `file` read for the pane.
fn found_at(path: &str, line: usize, file: Option<&str>) -> FindState {
    let mut state = FindState::open(FindTab::Text);
    for c in "total".chars() {
        state.on_key(&KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    let request = SearchRequestId::new();
    let asked = state.next_search().expect("a query");
    state.searching(request.clone(), asked);
    let text = String::new();
    let hit =
        SearchHit { path: path.into(), line, text, clipped: false, matches: vec![], scope: None };
    let results =
        SearchResults { query: "total".into(), hits: vec![hit], total: 1, ..Default::default() };
    state.accept(&SearchAnswer { request_id: request, results });
    if let Some(text) = file {
        read(&mut state, path, text);
    }
    state
}

fn read(state: &mut FindState, path: &str, text: &str) {
    let request = FileReadRequestId::new();
    state.previews_mut().reading(request.clone(), path);
    let file = ProjectFile {
        path: path.into(),
        text: text.into(),
        bytes: text.len() as u64,
        truncated: false,
        binary: false,
        image: None,
    };
    state.previews_mut().accept(&request, &file);
}

fn text(line: &Line<'static>) -> String {
    line.spans.iter().map(|span| span.content.as_ref()).collect()
}

fn lit(line: &Line<'static>, theme: &ThemeStyles) -> Vec<String> {
    let band = theme.match_bg().bg;
    line.spans
        .iter()
        .filter(|span| span.style.bg == band)
        .map(|span| span.content.to_string())
        .collect()
}

/// The break this catches: the pane opening the file at its top with the hit off screen, or
/// with the hit on the first row and nothing that leads up to it.
#[test]
fn a_hit_shows_a_third_of_the_way_down_with_its_line_numbered() {
    let theme = theme();
    let lines = preview_lines(&found_at("src/lib.rs", 20, Some(&source())), 60, 13, &theme);
    assert_eq!(text(&lines[0]), "src/lib.rs:20");
    let hit_row = lines.iter().position(|line| text(line).contains("let total")).expect("the hit");
    assert_eq!(hit_row, 1 + 12 / HIT_AT, "under the heading, a third of the way down");
    assert!(text(&lines[hit_row]).starts_with("20 "), "numbered");
    assert_eq!(lines.len(), 13, "the pane's height, no more");
}

/// The break this catches: the match unlit in the pane, or the hit's line not set apart from the
/// lines around it.
#[test]
fn the_hit_line_is_banded_and_its_match_lit() {
    let theme = theme();
    let lines = preview_lines(&found_at("src/lib.rs", 20, Some(&source())), 60, 13, &theme);
    let hit = lines.iter().find(|line| text(line).contains("let total")).expect("the hit");
    assert_eq!(lit(hit, &theme), ["total"]);
    let band = theme.cursor_line_bg().bg;
    assert!(hit.spans.iter().any(|span| span.style.bg == band), "banded");
    assert_eq!(hit.width(), 60, "the band runs the pane's width");
    let other = lines.iter().find(|line| text(line).contains("line 19")).expect("a neighbor");
    assert!(other.spans.iter().all(|span| span.style.bg != band), "only the hit is banded");
}

/// The break this catches: a hit near the end leaving the pane's lower rows empty when lines
/// above could fill them.
#[test]
fn a_hit_near_the_end_fills_the_pane_from_above() {
    let theme = theme();
    let lines = preview_lines(&found_at("src/lib.rs", 39, Some(&source())), 60, 13, &theme);
    assert_eq!(lines.len(), 13);
    assert!(text(&lines[12]).starts_with("40 "), "the last line is the file's last");
}

/// The break this catches: a match far along a long line cut off past the pane's edge, so the
/// line the reader is judging shows everything but what they searched for.
#[test]
fn a_match_past_the_edge_shifts_the_view_to_it() {
    let theme = theme();
    let long = format!("{}total", "x".repeat(200));
    let lines = preview_lines(&found_at("a.txt", 1, Some(&long)), 40, 5, &theme);
    assert_eq!(lit(&lines[1], &theme), ["total"]);
}

/// The break this catches: the pane blank while the file is read, or after a read fails, with
/// nothing to say why.
#[test]
fn the_pane_says_when_it_is_reading_or_could_not_read() {
    let theme = theme();
    let lines = preview_lines(&found_at("src/lib.rs", 20, None), 60, 13, &theme);
    assert!(lines.iter().any(|line| text(line) == "Reading…"));
    let mut state = found_at("src/lib.rs", 20, None);
    let request = FileReadRequestId::new();
    state.previews_mut().reading(request.clone(), "src/lib.rs");
    state.previews_mut().fail(&request, &ProjectFileFailure::NotFound);
    let lines = preview_lines(&state, 60, 13, &theme);
    assert!(lines.iter().any(|line| text(line).starts_with("Couldn't read the file")));
}

/// The break this catches: the Files tab lighting the file name it was typed to find inside the
/// file, or opening on some line other than the first.
#[test]
fn a_file_row_shows_the_top_of_its_file_with_nothing_lit() {
    let theme = theme();
    let mut state = FindState::open(FindTab::Files);
    let candidate = crate::ref_provider::Candidate {
        label: "total.rs".into(),
        insert: "total.rs".into(),
        path: true,
        ..Default::default()
    };
    state.set_files(vec![candidate]);
    read(&mut state, "total.rs", "let total = 1;\nmore\n");
    let lines = preview_lines(&state, 60, 13, &theme);
    assert_eq!(text(&lines[0]), "total.rs");
    assert!(text(&lines[1]).starts_with("1 let total"));
    assert!(lines.iter().all(|line| lit(line, &theme).is_empty()));
}
