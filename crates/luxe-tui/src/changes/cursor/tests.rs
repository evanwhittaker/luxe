use super::super::{ChangesLoad, ChangesView, PeekView};
use luxe_core::guided_review::{
    FileId, HunkId, LineRange, NoiseKind, ReviewFile, ReviewFileStatus, ReviewHunk, ReviewLine,
    ReviewLineKind,
};

/// A hunk of three added lines, keyed by `seed` so two hunks differ.
fn hunk(id: &FileId, seed: &str) -> ReviewHunk {
    let line = |n| ReviewLine {
        old_line: None,
        new_line: Some(n),
        kind: ReviewLineKind::Addition,
        text: format!("line {n}"),
    };
    ReviewHunk {
        id: HunkId::from_content(id, seed, seed),
        old_range: LineRange::new(1, 0),
        new_range: LineRange::new(1, 3),
        heading: None,
        symbol: None,
        lines: (1..=3).map(line).collect(),
        noise: NoiseKind::Semantic,
    }
}

/// A pane on a two-hunk diff: rows 0 and 4 are headers, the rest lines. Four rows fit.
fn diffing() -> ChangesView {
    let id = FileId::from_path_lineage(None, Some(std::path::Path::new("a.rs")));
    let file = ReviewFile {
        id: id.clone(),
        old_path: Some("a.rs".into()),
        new_path: Some("a.rs".into()),
        status: ReviewFileStatus::Modified,
        language: None,
        additions: Some(6),
        deletions: Some(0),
        hunks: vec![hunk(&id, "first"), hunk(&id, "second")],
        binary: false,
        generated: false,
        non_text: None,
    };
    let mut view = ChangesView::new("a.rs".into(), Vec::new());
    view.load = ChangesLoad::Ready(Box::new(file));
    view.viewport_rows.set(4);
    view
}

/// The cursor starts where the view does and stops at the first and last rows.
#[test]
fn the_diff_cursor_stops_at_both_ends() {
    let mut view = diffing();
    view.scroll = 2;
    view.move_diff_cursor(1);
    assert_eq!(view.diff_cursor, Some(3), "one on from the top row showing");
    view.move_diff_cursor(100);
    assert_eq!(view.diff_cursor, Some(7), "the last row");
    view.move_diff_cursor(-100);
    assert_eq!((view.diff_cursor, view.scroll), (Some(0), 0), "the first row, scrolled to it");
}

/// Moving down past the bottom scrolls the least that shows the whole row, counting the lines each
/// row really took: the second hunk's header carries a blank above it.
#[test]
fn the_diff_cursor_scrolls_by_the_lines_rows_really_take() {
    let mut view = diffing();
    view.diff_row_heights.replace(vec![1, 1, 1, 1, 2, 1, 1, 1]);
    view.move_diff_cursor(3);
    assert_eq!(view.scroll, 0, "row 3 is the fourth line, still on screen");
    view.move_diff_cursor(1);
    assert_eq!(view.scroll, 2, "rows 2 to 4 fill the four lines, the header taking two");
}

/// A row no frame has measured counts as one line, so a cursor moved before the first draw still
/// lands on screen.
#[test]
fn an_unmeasured_row_counts_as_one_line() {
    let mut view = diffing();
    view.move_diff_cursor(5);
    assert_eq!(view.scroll, 2);
}

/// Focus puts a cursor on the top row showing on each face, and keeps one already placed.
#[test]
fn focus_starts_each_cursor_on_the_top_row_and_keeps_a_placed_one() {
    let mut view = diffing();
    view.scroll = 3;
    view.focus();
    assert!(view.focused);
    assert_eq!((view.cursor, view.diff_cursor), (Some(4), Some(3)));

    view.diff_cursor = Some(6);
    view.focus();
    assert_eq!(view.diff_cursor, Some(6), "a cursor already placed stays");
}

/// A press puts the cursor of the face showing on the row it landed on, and leaves the other.
#[test]
fn focus_on_row_moves_only_the_showing_faces_cursor() {
    let mut view = diffing();
    view.focus_on_row(5);
    assert_eq!((view.cursor, view.diff_cursor), (None, Some(5)));
    view.view = PeekView::File;
    view.focus_on_row(5);
    assert_eq!((view.cursor, view.diff_cursor), (Some(6), Some(5)), "the File view is 1-based");
}
