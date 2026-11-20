//! Find preview — what Find's pane shows for the row under the cursor.
//!
//! A text hit shows the file around its line: numbered, syntax colored as the File view colors
//! it, the hit's line banded and every match of the rows' query lit, so the reader judges the hit
//! in its context without opening it. A file row shows the top of its file. The lines are cut to
//! the pane rather than wrapped, so code keeps its shape; a hit far along a long line shifts the
//! view across to it.

use super::{FindState, FindTab, PreviewText};
use crate::theme::styles::ThemeStyles;
use luxe_core::project_search::QueryMatcher;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

/// How far down the pane a hit's line sits, as a share of its height: low enough that what leads
/// up to it shows, high enough that most of the pane is what follows.
const HIT_AT: usize = 3;

/// The pane's lines for the row under `state`'s cursor, `width` by `height` cells.
pub(super) fn preview_lines(
    state: &FindState,
    width: u16,
    height: u16,
    theme: &ThemeStyles,
) -> Vec<Line<'static>> {
    if state.tab() == super::FindTab::Conversation {
        let Some(hit) = state.messages().get(state.selected()) else { return Vec::new() };
        return super::messages::message_preview(hit, state.query(), width, height, theme);
    }
    let Some(target) = state.target() else { return Vec::new() };
    let mut lines = vec![heading(&target.path, target.line, theme)];
    lines.extend(body(state, &target, (width, height.saturating_sub(1)), theme));
    lines
}

/// The file under the heading, or what the pane has to say about it instead.
fn body(
    state: &FindState,
    target: &super::FindTarget,
    (width, rows): (u16, u16),
    theme: &ThemeStyles,
) -> Vec<Line<'static>> {
    match state.previews().get(&target.path) {
        Some(PreviewText::Ready(file)) if file.binary => {
            note("Binary file, no text to show", theme)
        }
        Some(PreviewText::Ready(file)) => {
            let view = CodeView::new(state, &target.path, target.line, width);
            view.lines(&file.text, usize::from(rows), theme)
        }
        Some(PreviewText::Failed(why)) => note(&format!("Couldn't read the file: {why}"), theme),
        Some(PreviewText::Reading) | None => note("Reading…", theme),
    }
}

/// The file's path, its folder quiet, and the line the row points at.
fn heading(path: &str, line: Option<u32>, theme: &ThemeStyles) -> Line<'static> {
    let (folder, name) = path.rsplit_once('/').map_or(("", path), |(folder, name)| (folder, name));
    let mut spans = Vec::new();
    if !folder.is_empty() {
        spans.push(Span::styled(format!("{folder}/"), theme.muted));
    }
    spans.push(Span::styled(name.to_owned(), theme.label_on(false)));
    if let Some(line) = line {
        spans.push(Span::styled(format!(":{line}"), theme.accent));
    }
    Line::from(spans)
}

fn note(text: &str, theme: &ThemeStyles) -> Vec<Line<'static>> {
    vec![Line::default(), Line::from(Span::styled(text.to_owned(), theme.muted))]
}

/// How the file's lines are drawn: which line is the hit, what lights, and how wide the code is.
struct CodeView {
    language: Option<String>,
    hit: Option<usize>,
    matcher: Option<QueryMatcher>,
    width: usize,
}

impl CodeView {
    fn new(state: &FindState, path: &str, line: Option<u32>, width: u16) -> Self {
        let matcher = match state.tab() {
            FindTab::Text => state
                .rows_query()
                .and_then(|(query, options)| QueryMatcher::new(query, options).ok()),
            _ => None,
        };
        Self {
            language: path.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase()),
            hit: line.map(|line| (line as usize).saturating_sub(1)),
            matcher,
            width: usize::from(width),
        }
    }

    /// `rows` of `text`'s lines around the hit, or from the top for a file with none.
    fn lines(&self, text: &str, rows: usize, theme: &ThemeStyles) -> Vec<Line<'static>> {
        let all: Vec<&str> = text.lines().collect();
        let first = self.first_line(all.len(), rows);
        let shown = &all[first.min(all.len())..(first + rows).min(all.len())];
        let gutter = (first + shown.len()).max(1).to_string().len();
        let shift = self.shift(&all, gutter);
        shown
            .iter()
            .enumerate()
            .map(|(offset, line)| self.line(first + offset, line, (gutter, shift), theme))
            .collect()
    }

    /// The first line shown: the hit a third of the way down, never leaving rows empty at the end
    /// that earlier lines could fill.
    fn first_line(&self, total: usize, rows: usize) -> usize {
        let Some(hit) = self.hit else { return 0 };
        hit.saturating_sub(rows / HIT_AT).min(total.saturating_sub(rows))
    }

    /// Columns to skip on every line so the hit's first match shows, when it lies past the edge.
    fn shift(&self, all: &[&str], gutter: usize) -> usize {
        let code = self.width.saturating_sub(gutter + 1);
        let (Some(hit), Some(matcher)) = (self.hit, &self.matcher) else { return 0 };
        let Some(line) = all.get(hit) else { return 0 };
        let source = crate::tabstop::display_text(line);
        let Some(found) = matcher.find_in(&source).into_iter().next() else { return 0 };
        let end = UnicodeWidthStr::width(&source[..found.end]);
        match end > code {
            true => UnicodeWidthStr::width(&source[..found.start]).saturating_sub(code / 4),
            false => 0,
        }
    }

    /// One numbered line, colored, its matches lit, cut to the pane, and banded if it's the hit.
    fn line(
        &self,
        index: usize,
        text: &str,
        (gutter, shift): (usize, usize),
        theme: &ThemeStyles,
    ) -> Line<'static> {
        let is_hit = self.hit == Some(index);
        let number_style = if is_hit { theme.accent } else { theme.dim };
        let mut spans = vec![Span::styled(format!("{:>gutter$} ", index + 1), number_style)];
        spans.extend(self.code(text, shift, self.width.saturating_sub(gutter + 1), theme));
        match is_hit {
            true => banded(spans, self.width, theme.cursor_line_bg()),
            false => Line::from(spans),
        }
    }

    fn code(
        &self,
        text: &str,
        shift: usize,
        width: usize,
        theme: &ThemeStyles,
    ) -> Vec<Span<'static>> {
        let source = crate::tabstop::display_text(text);
        let mut spans =
            crate::view::highlighted_source_spans(&source, self.language.as_deref(), theme);
        if let Some(matcher) = &self.matcher {
            spans = crate::match_light::light_ranges(
                spans,
                &matcher.find_in(&source),
                theme.match_bg(),
            );
        }
        clipped(&spans, shift, width)
    }
}

/// `spans` from `shift` columns in, cut to `width`.
fn clipped(spans: &[Span<'static>], shift: usize, width: usize) -> Vec<Span<'static>> {
    let parts: Vec<(&str, ratatui::style::Style)> =
        spans.iter().map(|span| (span.content.as_ref(), span.style)).collect();
    crate::view::clip_styled_spans(&parts, shift, width)
        .into_iter()
        .map(|span| Span::styled(span.content.into_owned(), span.style))
        .collect()
}

/// `spans` across `width` cells on `band`, keeping any background a span brings, as a match does.
fn banded(spans: Vec<Span<'static>>, width: usize, band: ratatui::style::Style) -> Line<'static> {
    let used: usize = spans.iter().map(|span| span.width()).sum();
    let mut spans: Vec<Span<'static>> = spans
        .into_iter()
        .map(|span| match span.style.bg {
            Some(_) => span,
            None => Span::styled(span.content, span.style.patch(band)),
        })
        .collect();
    spans.push(Span::styled(" ".repeat(width.saturating_sub(used)), band));
    Line::from(spans)
}

#[cfg(test)]
mod tests;
