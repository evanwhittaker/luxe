//! Changes split — the diff side by side: old on the left, new on the right.
//!
//! The same row space as the unified list, so toggling the layout keeps the reader where they were,
//! and each column's code published on its own side so a selection stays in the file it began in.

use super::cells::{code_pieces, CodeCell};
use super::diff::{hunk_header_line, DrawnRows};
use crate::changes::ChangesView;
use crate::guided_review::diff::SplitDiffCell;
use crate::mouse::PeekSide;
use crate::view::diff_lines::{SplitLayout, SplitSide};
use luxe_core::guided_review::ReviewFile;
use ratatui::text::Line;

/// The side-by-side view from `offset`, `viewport` rows of it.
pub(super) fn split_lines(
    file: &ReviewFile,
    layout: &SplitLayout<'_>,
    view: &ChangesView,
    offset: usize,
    viewport: usize,
) -> DrawnRows {
    use crate::guided_review::diff::{visible_split_rows, SplitRow};
    use crate::guided_review::VisibleGroup;

    let groups: Vec<VisibleGroup<'_>> = file.hunks.iter().map(VisibleGroup::Hunk).collect();
    let rows = visible_split_rows(&groups, offset, viewport);
    let cursor = view.diff_cursor.filter(|_| view.focused);
    let mut drawn: DrawnRows = (Vec::with_capacity(viewport), Vec::with_capacity(viewport));
    for row in &rows {
        if drawn.0.len() >= viewport {
            break;
        }
        match row {
            // A collapsed run cannot happen here: the pane folds nothing.
            SplitRow::Folded { .. } => {}
            SplitRow::Header { old_range, new_range, heading, unified_index, .. } => {
                let header =
                    hunk_header_line(*old_range, *new_range, *heading, layout.styles, None);
                let standing = cursor == Some(*unified_index);
                push_header(&mut drawn, header, standing, layout);
            }
            SplitRow::Pair { old, new, unified_index, last_index, .. } => {
                // A pair's first row is its old line and its last the new one; a context line, or
                // a line with nothing beside it, is one row and both
                let rows = (*unified_index, *last_index);
                // Either half standing under the cursor makes the whole pair its row
                let standing = cursor.is_some_and(|row| row == rows.0 || row == rows.1);
                let pair = SplitPair { old: *old, new: *new, rows, wrap: view.word_wrap, standing };
                push_pair(&mut drawn, layout, &pair);
            }
        }
    }
    drawn.0.truncate(viewport);
    drawn.1.truncate(viewport);
    drawn
}

/// One row of a side-by-side diff: the old line and the new one, either possibly missing.
struct SplitPair<'a> {
    old: SplitDiffCell<'a>,
    new: SplitDiffCell<'a>,
    /// The old and the new line's rows in the pane's row space
    rows: (usize, usize),
    wrap: bool,
    /// Whether the keyboard's cursor is on it
    standing: bool,
}

/// A hunk header under a blank when it is not the first, across both columns in the cursor's band
/// when the cursor stands on it.
fn push_header(
    drawn: &mut DrawnRows,
    header: Line<'static>,
    standing: bool,
    layout: &SplitLayout<'_>,
) {
    if !drawn.0.is_empty() {
        drawn.0.push(Line::default());
        drawn.1.push(Vec::new());
    }
    let width = u16::try_from(layout.side * 2 + crate::view::diff_lines::SPLIT_DIVIDER_CELLS)
        .unwrap_or(u16::MAX);
    let band = layout.styles.cursor_line_bg();
    drawn.0.push(match standing {
        true => Line::from(crate::view::diff_lines::tint_row(header.spans, band, width)),
        false => header,
    });
    drawn.1.push(Vec::new());
}

/// How one side of a pair is drawn: plainly, or in the cursor's band over its own tint.
fn side_style(
    cell: SplitDiffCell<'_>,
    standing: bool,
    layout: &SplitLayout<'_>,
) -> crate::view::diff_lines::SplitCellStyle {
    let plain = crate::view::diff_lines::SplitCellStyle::default();
    let SplitDiffCell::Source(line) = cell else { return plain };
    if !standing {
        return plain;
    }
    let tint = crate::view::diff_lines::diff_line_background(line.kind, layout.styles);
    crate::view::diff_lines::SplitCellStyle {
        background: Some(layout.styles.cursor_line_over(tint)),
        ..plain
    }
}

/// Draws one side-by-side pair, wrapped or clipped, with where each side's code sits.
fn push_pair(drawn: &mut DrawnRows, layout: &SplitLayout<'_>, pair: &SplitPair<'_>) {
    let old_style = side_style(pair.old, pair.standing, layout);
    let new_style = side_style(pair.new, pair.standing, layout);
    // Pairing is exact here, so the marks are too — no run-matching heuristic.
    let changed = match (pair.old, pair.new) {
        (SplitDiffCell::Source(old), SplitDiffCell::Source(new)) if old.kind != new.kind => {
            luxe_core::intraline::intraline_change(
                &crate::tabstop::display_text(&old.text),
                &crate::tabstop::display_text(&new.text),
            )
        }
        _ => luxe_core::intraline::IntralineChange::default(),
    };
    let old = side_pieces(pair.old, (SplitSide::Old, pair.rows.0), layout, pair.wrap);
    let new = side_pieces(pair.new, (SplitSide::New, pair.rows.1), layout, pair.wrap);
    let lines = match pair.wrap {
        true => layout.wrapped_pair(
            (pair.old, &changed.removed, &old_style),
            (pair.new, &changed.added, &new_style),
        ),
        false => {
            let mut spans =
                layout.cell_spans(SplitSide::Old, pair.old, &changed.removed, &old_style);
            spans.push(layout.divider());
            spans.extend(layout.cell_spans(SplitSide::New, pair.new, &changed.added, &new_style));
            vec![Line::from(spans)]
        }
    };
    for (index, line) in lines.into_iter().enumerate() {
        drawn.0.push(line);
        drawn.1.push(old.get(index).into_iter().chain(new.get(index)).copied().collect());
    }
}

/// Where the code of one side of a pair sits on each row it wraps to.
fn side_pieces(
    cell: SplitDiffCell<'_>,
    (side, row): (SplitSide, usize),
    layout: &SplitLayout<'_>,
    wrap: bool,
) -> Vec<CodeCell> {
    let SplitDiffCell::Source(line) = cell else { return Vec::new() };
    let (x, peek_side) = match side {
        SplitSide::Old => (layout.continuation(), PeekSide::Old),
        SplitSide::New => (
            layout.side + crate::view::diff_lines::SPLIT_DIVIDER_CELLS + layout.continuation(),
            PeekSide::New,
        ),
    };
    code_pieces(&line.text, row, peek_side, (x, layout.source.max(1)), wrap)
}
