//! Selection — keeps a transcript selection on its text as rows reflow, and resolves its copy.

use super::RenderedConversationLine;
use crate::app::App;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

/// The plain text of a rendered line, styling dropped — what a copy puts on the clipboard.
pub(in crate::view) fn line_text(line: &Line<'static>) -> String {
    line.spans.iter().map(|span| span.content.as_ref()).collect()
}

/// Follows the selection to where its text went, and resolves the word, step and copy requests the
/// pointer left for this frame. True when a step scrolled the transcript to keep its moving end on
/// screen, so the caller's window is stale.
pub(in crate::view) fn resolve_selection_copy(
    app: &mut App,
    rendered_lines: &[RenderedConversationLine],
) -> bool {
    // A transcript selection's line indices belong to the frame that made it, and the transcript
    // moves under it constantly — output streams in, a card opens. Follow each end's pin to where it
    // is now instead of throwing the selection away.
    remap_selection(app, rendered_lines);
    // A double-click asks for the word under it, which is a fact about the laid-out line rather
    // than about the press — so the gesture leaves the request here and this frame resolves it.
    if std::mem::take(&mut app.pointer.expand_word) {
        expand_selection_to_word(app, rendered_lines);
    }
    let scrolled = step_selection(app, rendered_lines);
    // A copy key or the menu's Copy asks for the selected text; the plain text only exists here,
    // in the frame that laid it out.
    let copy_requested = std::mem::take(&mut app.pointer.copy_requested);
    if let Some(selection) = app.pointer.selection.as_ref().filter(|_| copy_requested) {
        let plain =
            rendered_lines.iter().map(|rendered| line_text(&rendered.line)).collect::<Vec<_>>();
        let text = selection.grid.extract(&plain);
        app.record_copy(text);
    }
    scrolled
}

/// Re-derives a transcript selection's line indices from its pins.
///
/// The pin is the selection's real address: a rendered line's index changes whenever anything above
/// it reflows, but its (item, line-within-item) identity does not. An end whose line no longer
/// exists at all — a card that collapsed under the selection — takes the whole selection with it,
/// because there is nothing honest left to highlight.
fn remap_selection(app: &mut App, rendered_lines: &[RenderedConversationLine]) {
    let Some(selection) = app.pointer.selection.as_mut() else {
        return;
    };
    let line_of = |pin: &crate::mouse::TextPin| {
        rendered_lines.iter().position(|rendered| {
            *rendered.item_id == pin.item
                && rendered.visual_line_within_item == pin.visual_line_within_item
        })
    };
    match (line_of(&selection.pins.0), line_of(&selection.pins.1)) {
        (Some(anchor), Some(cursor)) => {
            selection.grid.anchor.line = anchor;
            selection.grid.cursor.line = cursor;
        }
        _ => app.pointer.selection = None,
    }
}

/// Grows a collapsed transcript selection to the word its cursor sits on.
///
/// A press only knows a cell; where a word starts and ends is a property of the rendered line, and
/// this is the one place that holds it. A cell with no word in it (whitespace, padding) leaves the
/// selection collapsed, so a double-click on empty space selects nothing rather than guessing.
fn expand_selection_to_word(app: &mut App, rendered_lines: &[RenderedConversationLine]) {
    let Some(selection) = app.pointer.selection.as_mut() else {
        return;
    };
    let at = selection.grid.cursor;
    let Some(rendered) = rendered_lines.get(at.line) else {
        return;
    };
    let Some(word) = crate::selection::word_columns(&line_text(&rendered.line), at.column) else {
        return;
    };
    selection.grid = crate::selection::GridSelection {
        anchor: crate::selection::TextPoint::new(at.line, word.start),
        cursor: crate::selection::TextPoint::new(at.line, word.end),
    };
}

/// Moves the selection's free end for each Shift and arrow press since the last frame, and pins it
/// to the line it lands on so it follows that line through the next reflow.
fn step_selection(app: &mut App, rendered_lines: &[RenderedConversationLine]) -> bool {
    let steps = std::mem::take(&mut app.pointer.selection_steps);
    let Some(selection) = app.pointer.selection.as_mut().filter(|_| !steps.is_empty()) else {
        return false;
    };
    let plain: Vec<String> = rendered_lines.iter().map(|it| line_text(&it.line)).collect();
    for step in steps {
        selection.grid.cursor = crate::selection::step_point(&plain, selection.grid.cursor, step);
    }
    repin_moving_end(selection, rendered_lines);
    // The moving end is where the reader is looking, so stepping it off an edge scrolls with it
    let before = app.scroll_state.offset;
    let line = selection.grid.cursor.line;
    app.scroll_state.reveal(line);
    app.scroll_state.offset != before
}

/// Pins the selection's moving end to the row it stepped onto, so a reflow keeps it on that text.
fn repin_moving_end(
    selection: &mut crate::app::ConversationSelection,
    rendered_lines: &[RenderedConversationLine],
) {
    let Some(rendered) = rendered_lines.get(selection.grid.cursor.line) else {
        return;
    };
    selection.pins.1 = crate::mouse::TextPin {
        item: (*rendered.item_id).clone(),
        visual_line_within_item: rendered.visual_line_within_item,
    };
}

/// Repaints the `[start, end)` cell range of a line with `style`, splitting spans at the range's
/// edges so a selection can cover part of a word without disturbing the rest of the line.
pub(in crate::view) fn highlight_columns(
    line: &Line<'static>,
    columns: std::ops::Range<usize>,
    style: Style,
) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(line.spans.len() + 2);
    let mut column = 0usize;
    for span in &line.spans {
        let width = crate::selection::cell_width(&span.content);
        let span_range = column..column + width;
        column += width;
        // Wholly outside or wholly inside: no split needed.
        if span_range.end <= columns.start || span_range.start >= columns.end {
            spans.push(span.clone());
            continue;
        }
        if span_range.start >= columns.start && span_range.end <= columns.end {
            spans.push(Span::styled(span.content.clone(), span.style.patch(style)));
            continue;
        }
        for (piece, selected) in [
            (span_range.start..columns.start.max(span_range.start), false),
            (columns.start.max(span_range.start)..columns.end.min(span_range.end), true),
            (columns.end.min(span_range.end)..span_range.end, false),
        ] {
            if piece.is_empty() {
                continue;
            }
            let local = piece.start - span_range.start..piece.end - span_range.start;
            let text = crate::selection::slice_columns(&span.content, local);
            if text.is_empty() {
                continue;
            }
            let piece_style = if selected { span.style.patch(style) } else { span.style };
            spans.push(Span::styled(text, piece_style));
        }
    }
    Line::from(spans)
}
