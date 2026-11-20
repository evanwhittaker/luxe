//! Match light — paints what a search found inside text that is already styled.
//!
//! Find's preview, the File view a hit opens in, and the conversation and session previews all
//! draw text someone searched, each styled its own way first (syntax colors, markdown, transcript
//! bands). This lays the theme's match band over the found ranges afterward, splitting spans at
//! the ranges' edges, so the styling underneath keeps its colors and only the match changes.

use luxe_core::project_search::{MatchRange, QueryMatcher};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// `spans` with `style` patched over every byte range in `ranges`, which are offsets into the
/// spans' text joined end to end. A range that doesn't fall on character boundaries is skipped,
/// since it was found in some other text and can't be trusted here.
pub(crate) fn light_ranges(
    spans: Vec<Span<'static>>,
    ranges: &[MatchRange],
    style: Style,
) -> Vec<Span<'static>> {
    let text: String = spans.iter().map(|span| span.content.as_ref()).collect();
    let ranges = usable_ranges(&text, ranges);
    if ranges.is_empty() {
        return spans;
    }
    let mut out = Vec::with_capacity(spans.len() + ranges.len() * 2);
    let mut offset = 0usize;
    for span in spans {
        out.extend(split_span(&span, offset, &ranges, style));
        offset += span.content.len();
    }
    out
}

/// The ranges that are non-empty and on `text`'s character boundaries, which also keeps them
/// inside it.
fn usable_ranges<'a>(text: &str, ranges: &'a [MatchRange]) -> Vec<&'a MatchRange> {
    ranges
        .iter()
        .filter(|range| range.start < range.end)
        .filter(|range| text.is_char_boundary(range.start) && text.is_char_boundary(range.end))
        .collect()
}

/// One span cut at every range edge inside it, the pieces inside a range wearing `style`.
fn split_span(
    span: &Span<'static>,
    offset: usize,
    ranges: &[&MatchRange],
    style: Style,
) -> Vec<Span<'static>> {
    let content = span.content.as_ref();
    let cuts = cuts_inside(content.len(), offset, ranges);
    // the cuts are sorted and distinct, so no piece is empty
    cuts.windows(2)
        .map(|pair| {
            let at = offset + pair[0];
            let inside = ranges.iter().any(|range| range.start <= at && at < range.end);
            let piece_style = if inside { span.style.patch(style) } else { span.style };
            Span::styled(content[pair[0]..pair[1]].to_owned(), piece_style)
        })
        .collect()
}

/// Where a span `len` bytes long starting at `offset` is cut: its ends, and every range edge
/// inside it, in order, relative to the span. An edge outside the span lands on one of its ends.
fn cuts_inside(len: usize, offset: usize, ranges: &[&MatchRange]) -> Vec<usize> {
    let mut cuts = vec![0, len];
    for range in ranges {
        cuts.push(range.start.saturating_sub(offset).min(len));
        cuts.push(range.end.saturating_sub(offset).min(len));
    }
    cuts.sort_unstable();
    cuts.dedup();
    cuts
}

/// Each line with what `matcher` finds in its text lit, a line at a time, as a search reads.
pub(crate) fn light_lines(
    lines: Vec<Line<'static>>,
    matcher: &QueryMatcher,
    style: Style,
) -> Vec<Line<'static>> {
    lines.into_iter().map(|line| light_line(line, matcher, style)).collect()
}

/// How far down a pane the first match sits once scrolled to, in parts of its height, so what
/// leads up to it shows too.
const MATCH_AT: usize = 3;

/// The first of `lines` `matcher` finds something in.
pub(crate) fn first_match(lines: &[Line<'static>], matcher: &QueryMatcher) -> Option<usize> {
    lines.iter().position(|line| {
        let text: String = line.spans.iter().map(|span| span.content.as_ref()).collect();
        !matcher.find_in(&text).is_empty()
    })
}

/// The `height` of `lines` a pane shows: from the top, or from a little above line `first` when
/// it would sit past the bottom.
pub(crate) fn window_on(
    lines: Vec<Line<'static>>,
    first: Option<usize>,
    height: usize,
) -> Vec<Line<'static>> {
    let start = first.filter(|at| *at >= height).map_or(0, |at| at - height / MATCH_AT);
    lines.into_iter().skip(start).take(height).collect()
}

/// One line with its matches lit, keeping the line's own style and alignment.
fn light_line(line: Line<'static>, matcher: &QueryMatcher, style: Style) -> Line<'static> {
    let text: String = line.spans.iter().map(|span| span.content.as_ref()).collect();
    let ranges = matcher.find_in(&text);
    if ranges.is_empty() {
        return line;
    }
    let (line_style, alignment) = (line.style, line.alignment);
    let mut lit = Line::from(light_ranges(line.spans, &ranges, style));
    lit.style = line_style;
    lit.alignment = alignment;
    lit
}

#[cfg(test)]
mod tests;
