//! Unified review diff — the selected file's rows one above the other, with folds, the active bar
//! and the feedback under each line.

use super::annotations::{
    comment_lines, findings_at, line_anchor_matches, thread_lines, threads_at,
};
use super::source::{
    active_bar_span, diff_line_background, emphasis_style, full_source_spans,
    highlighted_source_spans, hunk_header_line, intraline_emphasis, ACTIVE_BAR_WIDTH,
};
use super::split::render_split_file_diff;
use super::{append_end_of_reading, render_non_text_diff, shown_file};
use crate::app::App;
use crate::guided_review::diff::{clip_terminal_cells, gutter_width, visible_rows, UnifiedDiffRow};
use crate::guided_review::visible_groups;
use crate::view::diff_lines::{changed_spans, clip_styled_spans, rendered_marker, tint_row};
use crate::view::emphasis;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Cells a unified row spends between its numbers and its source: a space after each number, and
/// the diff sign.
pub(super) const UNIFIED_NUMBER_CHROME: usize = 3;

pub(in crate::view::guided_review) fn render_first_file_diff(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    review: &luxe_core::guided_review::GuidedReview,
) {
    let Some(file) = shown_file(app, review) else {
        return;
    };
    if !file.is_text() {
        render_non_text_diff(frame, area, app, file);
        return;
    }

    if app.guided_review_workspace.split_layout_active_for(file) {
        render_split_file_diff(frame, area, app, file, review);
        return;
    }

    let groups = visible_groups(file, &app.guided_review_workspace.fold);
    let cursor = app.guided_review_workspace.position.line_offset;
    let total_rows = groups.iter().map(crate::guided_review::VisibleGroup::rows).sum();
    let drawn = super::fit_window(app, (area.height, total_rows), |top| {
        let drawn = draw_unified(app, file, &groups, top, area);
        let cursor_row = drawn.row_ys.iter().find(|(.., row)| *row == cursor);
        let cursor_row = cursor_row.map(|(y, height, _)| (*y, *height));
        (drawn, cursor_row)
    });
    publish_diff_rows(app, area, &drawn.row_ys, &drawn.rows, file);
    let drawn_end = drawn.row_ys.last().map_or(0, |(.., row)| row + 1);
    let mut lines = drawn.lines;
    append_end_of_reading(&mut lines, app, review, (&groups, drawn_end), area.height);
    frame.render_widget(Paragraph::new(lines), area);
}

/// A unified window laid out from one top row, before anything is published or drawn.
struct UnifiedDrawn<'a> {
    lines: Vec<Line<'static>>,
    /// (first visual Y, visual height, absolute row index) per row, so click regions map to the
    /// right source row despite blank inter-hunk spacers and wrapped continuation lines
    row_ys: Vec<(u16, u16, usize)>,
    rows: Vec<UnifiedDiffRow<'a>>,
}

/// Lays out the rows that fit `area` from row `top`, the cursor's row banded and marked.
fn draw_unified<'a>(
    app: &App,
    file: &'a luxe_core::guided_review::ReviewFile,
    groups: &[crate::guided_review::VisibleGroup<'a>],
    top: usize,
    area: Rect,
) -> UnifiedDrawn<'a> {
    let rows = visible_rows(groups, top, usize::from(area.height));
    let emphasized = intraline_emphasis(&rows);
    let context = unified_context(app, (file, groups), (top, area), &emphasized);
    let mut drawn = UnifiedDrawn { lines: Vec::new(), row_ys: Vec::new(), rows: Vec::new() };
    for (index, row) in rows.iter().enumerate() {
        // Separate stacked blocks with a blank row so each reads as a distinct one.
        if matches!(row, UnifiedDiffRow::HunkHeader { .. } | UnifiedDiffRow::Folded(_))
            && !drawn.lines.is_empty()
        {
            drawn.lines.push(Line::default());
        }
        let first_y = drawn.lines.len() as u16;
        drawn.lines.extend(unified_row_lines(&context, row, index));
        let visual_height = (drawn.lines.len() as u16).saturating_sub(first_y).max(1);
        drawn.row_ys.push((first_y, visual_height, top + index));
    }
    drawn.rows = rows;
    drawn
}

/// What every row of a unified window from `top` is drawn against.
fn unified_context<'a>(
    app: &'a App,
    (file, groups): (
        &'a luxe_core::guided_review::ReviewFile,
        &[crate::guided_review::VisibleGroup<'_>],
    ),
    (top, area): (usize, Rect),
    emphasized: &'a crate::view::diff_lines::IntralineEmphasis,
) -> UnifiedContext<'a> {
    let width = gutter_width(&file.hunks);
    let position = &app.guided_review_workspace.position;
    UnifiedContext {
        app,
        file,
        width,
        // One cell at the margin for the bar down the hunk being read, so `Space` is a shape
        // moving rather than a redraw of the same frame with different code in it
        source_width: usize::from(area.width)
            .saturating_sub(width.saturating_mul(2) + ACTIVE_BAR_WIDTH + UNIFIED_NUMBER_CHROME),
        area_width: area.width,
        emphasized,
        cursor: position.line_offset.checked_sub(top),
        active_rows: active_window_rows(app, groups, top),
    }
}

/// The window rows the hunk under the cursor owns, so its bar runs the whole height of it even
/// when the window starts below its header.
fn active_window_rows(
    app: &App,
    groups: &[crate::guided_review::VisibleGroup<'_>],
    top: usize,
) -> Option<std::ops::Range<usize>> {
    let hunk_id = app.guided_review_workspace.position.hunk_id.as_ref()?;
    let span = crate::guided_review::hunk_row_span(groups, hunk_id)?;
    Some(span.start.saturating_sub(top)..span.end.saturating_sub(top))
}

/// One row of the window as the lines it draws, the feedback under a line included.
fn unified_row_lines(
    ctx: &UnifiedContext<'_>,
    row: &UnifiedDiffRow<'_>,
    index: usize,
) -> Vec<Line<'static>> {
    let app = ctx.app;
    let on_active_hunk = ctx.active_rows.as_ref().is_some_and(|span| span.contains(&index));
    let on_cursor = ctx.cursor == Some(index);
    match *row {
        UnifiedDiffRow::Folded(run) => {
            vec![folded_run_line(app, run, on_folded(app, on_cursor), false)]
        }
        UnifiedDiffRow::HunkHeader { hunk_id, old_range, new_range, heading } => {
            let header = (hunk_id, old_range, new_range, heading);
            let lines = hunk_header_rows(app, header, (on_active_hunk, on_cursor), ctx.area_width);
            band_first_line(lines, on_cursor.then(|| app.theme_styles.cursor_line_bg()), ctx)
        }
        UnifiedDiffRow::Source(line) => {
            let mut lines = unified_source_rows(ctx, line, index, on_active_hunk);
            let width = usize::from(ctx.area_width);
            append_unified_annotations(&mut lines, app, (ctx.file, line), (width, on_cursor));
            lines
        }
    }
}

/// A folded run lights only while the cursor sits on it outside any hunk.
pub(super) fn on_folded(app: &App, on_cursor: bool) -> bool {
    on_cursor && app.guided_review_workspace.position.hunk_id.is_none()
}

/// Lays the cursor's `band` across the first line, the header itself rather than the cards under it.
fn band_first_line(
    mut lines: Vec<Line<'static>>,
    band: Option<ratatui::style::Style>,
    ctx: &UnifiedContext<'_>,
) -> Vec<Line<'static>> {
    let (Some(band), Some(first)) = (band, lines.first_mut()) else { return lines };
    *first = Line::from(tint_row(std::mem::take(&mut first.spans), band, ctx.area_width));
    lines
}

/// A hunk's header row, with the comment cards anchored to the whole hunk under it.
fn hunk_header_rows(
    app: &App,
    (hunk_id, old_range, new_range, heading): (
        &luxe_core::guided_review::HunkId,
        luxe_core::guided_review::LineRange,
        luxe_core::guided_review::LineRange,
        Option<&str>,
    ),
    (on_active_hunk, on_cursor): (bool, bool),
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines =
        vec![hunk_header_line(app, hunk_id, old_range, new_range, heading, on_active_hunk)];
    let on_hunk = |anchor: &_| crate::guided_review::hunk_anchor_matches(anchor, hunk_id);
    lines.extend(comment_lines(app, &findings_at(app, on_hunk), usize::from(width), on_cursor));
    lines
}

/// What every unified source row is drawn against.
struct UnifiedContext<'a> {
    app: &'a App,
    file: &'a luxe_core::guided_review::ReviewFile,
    /// Digits each line number takes
    width: usize,
    source_width: usize,
    area_width: u16,
    emphasized: &'a crate::view::diff_lines::IntralineEmphasis,
    /// Where the cursor's row sits in the window, when the window holds it
    cursor: Option<usize>,
    /// Where the active hunk's rows sit in the window
    active_rows: Option<std::ops::Range<usize>>,
}

/// One source line as its visual rows: the active bar, both numbers and the sign, then the source
/// wrapped under that gutter or clipped at the diff's column offset.
fn unified_source_rows(
    ctx: &UnifiedContext<'_>,
    line: &luxe_core::guided_review::ReviewLine,
    index: usize,
    on_active_hunk: bool,
) -> Vec<Line<'static>> {
    let (app, file, width) = (ctx.app, ctx.file, ctx.width);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let old = line.old_line.map_or_else(|| " ".repeat(width), |number| format!("{number:>width$}"));
    let new = line.new_line.map_or_else(|| " ".repeat(width), |number| format!("{number:>width$}"));
    let style = match line.kind {
        luxe_core::guided_review::ReviewLineKind::Addition => app.theme_styles.success,
        luxe_core::guided_review::ReviewLineKind::Deletion => app.theme_styles.error,
        _ => app.theme_styles.text,
    };
    // The cursor's row lights its numbers and marks them, as the Changes pane marks its line, so
    // the reader can see which line a comment or a question will land on
    let on_cursor = ctx.cursor == Some(index);
    let (mark, numbers) = match on_cursor {
        true => ("›", app.theme_styles.accent.add_modifier(ratatui::style::Modifier::BOLD)),
        false => (" ", app.theme_styles.muted),
    };
    // The fixed gutter (active bar + old/new numbers + diff marker) prefixing the
    // first visual line; wrapped continuations align their source under it.
    let gutter = || {
        vec![
            active_bar_span(app, on_active_hunk),
            Span::styled(format!("{old} {new}{mark}"), numbers),
            Span::styled(rendered_marker(line.kind), style),
        ]
    };
    let source = crate::tabstop::display_text(&line.text);
    let row_bg = unified_row_band(app, line.kind, &source, on_cursor);
    let finish = |mut spans: Vec<Span<'static>>| match row_bg {
        Some(background) => Line::from(tint_row(spans, background, ctx.area_width)),
        None => {
            spans.shrink_to_fit();
            Line::from(spans)
        }
    };
    if app.guided_review_workspace.word_wrap {
        // Soft-wrap the full source under the gutter with an aligned continuation glyph,
        // instead of horizontal scrolling (`w` toggles this).
        let full = full_source_spans(line, changed_spans(ctx.emphasized, index), style, file, app);
        let continuation_pad = ACTIVE_BAR_WIDTH + width * 2 + 2;
        for (segment_index, segment) in
            crate::widgets::wrap_cells(full, ctx.source_width).into_iter().enumerate()
        {
            let mut spans = if segment_index == 0 {
                gutter()
            } else {
                vec![Span::styled(
                    format!(
                        "{}{}",
                        " ".repeat(continuation_pad),
                        crate::glyphs::Mark::Continuation
                    ),
                    app.theme_styles.dim,
                )]
            };
            spans.extend(segment);
            lines.push(finish(spans));
        }
        // Guarantee at least one visual line (e.g. an empty added line).
        if lines.is_empty() {
            lines.push(finish(gutter()));
        }
        return lines;
    }
    let column_offset = app.guided_review_workspace.diff_column_offset;
    let source_spans = match line.kind {
        luxe_core::guided_review::ReviewLineKind::Context => {
            let clipped = clip_terminal_cells(&source, column_offset, ctx.source_width);
            highlighted_source_spans(&clipped, file.language.as_deref(), app)
        }
        // Mark only the spans that changed; everything either side keeps the line's
        // own color.
        _ => crate::view::diff_lines::owned_spans(clip_styled_spans(
            &emphasis::parts(
                &source,
                changed_spans(ctx.emphasized, index),
                style,
                emphasis_style(app, style, line),
            ),
            column_offset,
            ctx.source_width,
        )),
    };
    let mut spans = gutter();
    spans.extend(source_spans);
    lines.push(match row_bg {
        Some(background) => Line::from(tint_row(spans, background, ctx.area_width)),
        None => Line::from(spans),
    });
    lines
}

/// The band a unified source row wears: the add or delete tint, a search hit's band over it so
/// the located line stays obvious, and the cursor's lift over either.
fn unified_row_band(
    app: &App,
    kind: luxe_core::guided_review::ReviewLineKind,
    source: &str,
    on_cursor: bool,
) -> Option<ratatui::style::Style> {
    let band = match app.guided_review_workspace.line_matches_search(source) {
        true => Some(app.theme_styles.selected_bg),
        false => diff_line_background(kind, app),
    };
    match on_cursor {
        true => Some(app.theme_styles.cursor_line_over(band)),
        false => band,
    }
}

/// The comments and earlier threads anchored to a source line, under it.
///
/// The comment sits under the line it is about, the way every review tool shows one: a bare `R1` in
/// a gutter, with the words in a drawer over the far corner, makes "where did my comment go" a
/// question at all.
fn append_unified_annotations(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    (file, line): (&luxe_core::guided_review::ReviewFile, &luxe_core::guided_review::ReviewLine),
    (width, open): (usize, bool),
) {
    lines.extend(comment_lines(
        app,
        &findings_at(app, |anchor| line_anchor_matches(anchor, &file.id, line)),
        width,
        open,
    ));
    lines.extend(thread_lines(app, &threads_at(app, &file.id, line), width));
}

/// Publish a click region per rendered source/header row → its absolute index (spanning its
/// wrapped visual lines), so a click moves the cursor/anchor there. A collapsed run is the one
/// row whose click means "open this" rather than "put the cursor here".
fn publish_diff_rows(
    app: &App,
    area: Rect,
    row_ys: &[(u16, u16, usize)],
    rows: &[UnifiedDiffRow<'_>],
    file: &luxe_core::guided_review::ReviewFile,
) {
    let mut regions = app.hit.borrow_mut();
    for ((y, height, absolute), row) in row_ys.iter().zip(rows) {
        let row_y = area.y.saturating_add(*y);
        if row_y >= area.bottom() {
            continue;
        }
        let region_height = (*height).min(area.bottom().saturating_sub(row_y));
        let target = if matches!(row, UnifiedDiffRow::Folded(_)) {
            crate::mouse::MouseTarget::ReviewFoldedRun(file.id.clone())
        } else {
            crate::mouse::MouseTarget::ReviewDiffRow(*absolute)
        };
        regions.push(Rect::new(area.x, row_y, area.width, region_height), target);
    }
}

/// The row that stands in for a run of collapsed mechanical hunks.
///
/// It is a row, not an absence: whatever the classifier decided, the reviewer can see that
/// something is there, what kind of thing it is, how much of it there is, and how to open it.
pub(super) fn folded_run_line(
    app: &App,
    run: crate::guided_review::FoldedRun,
    selected: bool,
    full_width: bool,
) -> Line<'static> {
    let style = if selected {
        app.theme_styles.muted.patch(app.theme_styles.selected_bg)
    } else {
        app.theme_styles.muted
    };
    let marker =
        format!("{}{} ", if selected { "› " } else { "  " }, crate::glyphs::Mark::FoldClosed);
    let mut spans = vec![
        Span::styled(marker, style),
        Span::styled(run.summary(), style),
        Span::styled("  ·  enter opens", app.theme_styles.dim),
    ];
    if full_width {
        spans.push(Span::styled(String::new(), style));
    }
    Line::from(spans)
}
