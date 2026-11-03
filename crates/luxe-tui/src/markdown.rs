//! markdown — render model markdown into ratatui styled lines.
//!
//! Pure function: `render_markdown(src, theme) -> Vec<Line>` — no App state, no engine.
//! Uses pulldown-cmark to walk events; maintains a style stack for inline formatting.
//! Streaming-safe: tolerates unterminated fences/emphasis (partial input is fine).
//!
//! # Voices
//!
//! The same walk serves two readers. An answer takes the theme's markdown tokens as-is; model
//! reasoning keeps its muted italic voice and spends structure rather than color — see
//! `MarkdownVoice`.

mod fence;
pub mod html;
mod layout;
mod line;
mod table;
mod voice;

use ratatui::text::Line;

use crate::theme::styles::ThemeStyles;
pub(crate) use layout::layout_markdown_in;
pub(crate) use line::{FenceRule, ImageRow, MarkdownBlockKind, MarkdownLine, QuoteMark};
pub(crate) use voice::MarkdownVoice;

/// Render markdown source into styled `Line`s — one per block.
///
/// Supports: paragraphs, headings (h1-h6), bold (`**`/`__`), italic (`*`/`_`),
/// inline code (`` ` ``), fenced code blocks (with syntect highlighting via `highlight_code`),
/// blockquotes, lists (ordered and unordered), horizontal rules, and task list markers.
pub fn render_markdown(src: &str, theme: &ThemeStyles) -> Vec<Line<'static>> {
    layout_markdown(src, theme).into_iter().flat_map(unwrapped_lines).collect()
}

/// A laid-out line as lines to be drawn at a width nothing here knows: a table at its natural
/// width, and a quoted line behind its rules but off the plate, which only a known width can fill.
fn unwrapped_lines(layout: MarkdownLine) -> Vec<Line<'static>> {
    let lines = match layout.table {
        Some(table) => table.lines_at(usize::MAX),
        None => vec![layout.line],
    };
    let Some(quote) = layout.quote else { return lines };
    lines
        .into_iter()
        .map(|line| {
            let rules = quote.rule_spans(ratatui::style::Style::default());
            Line::from(rules.chain(line.spans).collect::<Vec<_>>()).style(line.style)
        })
        .collect()
}

pub(crate) fn layout_markdown(src: &str, theme: &ThemeStyles) -> Vec<MarkdownLine> {
    layout_markdown_in(src, theme, MarkdownVoice::Answer)
}

#[cfg(test)]
mod mixed_tests;
#[cfg(test)]
mod tests;
