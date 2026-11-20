//! Session preview — the session under the cursor, drawn as its transcript draws it.
//!
//! The pane beside the session list shows the turns the catalog kept, through the transcript's own
//! renderer ([`crate::view::excerpt_lines`]): a user's turn on its band, a reply's markdown styled.
//! With a query, it scrolls to the first turn that says it and lights it there. The catalog keeps
//! the turns of the same text the session search reads, but the search is fuzzy and joins a
//! message's blocks, so a session can still match on text no one turn holds; the pane says so when
//! the query is in the session's text but not its turns.

use super::SessionEntry;
use crate::app::ConvoBlock;
use crate::dashboard::{PreviewRole, PreviewTurn};
use crate::glyphs::Icon;
use crate::theme::styles::ThemeStyles;
use luxe_core::project_search::{QueryMatcher, SearchOptions};
use ratatui::text::{Line, Span};

/// The preview for `session`, `width` by `height`, scrolled to and lit for `query`.
pub(super) fn preview_lines(
    session: Option<&SessionEntry>,
    query: &str,
    width: u16,
    height: u16,
    theme: &ThemeStyles,
) -> Vec<Line<'static>> {
    let Some(session) = session else {
        return vec![Line::styled(" No session selected", theme.muted)];
    };
    let matcher = matcher(query);
    let body = body_lines(session, matcher.as_ref(), width, height, theme);
    let first =
        matcher.as_ref().and_then(|matcher| crate::match_light::first_match(&body, matcher));
    // No heading: a session's name is its first message, which the first turn already shows
    let mut lines = Vec::new();
    if first.is_none() && matched_later(session, matcher.as_ref()) {
        lines.extend(later_note(query, width, theme));
    }
    let window = usize::from(height).saturating_sub(lines.len());
    let shown = crate::match_light::window_on(body, first, window);
    lines.extend(match &matcher {
        Some(matcher) => crate::match_light::light_lines(shown, matcher, theme.match_bg()),
        None => shown,
    });
    lines
}

/// Whether the query is in the session's searched text, though not in its kept turns.
fn matched_later(session: &SessionEntry, matcher: Option<&QueryMatcher>) -> bool {
    matcher.is_some_and(|matcher| !matcher.find_in(&session.search_text).is_empty())
}

/// A literal, smart-case reading of the query, which lights what the fuzzy list search would
/// call a match whenever the reader typed words; `None` for no query.
fn matcher(query: &str) -> Option<QueryMatcher> {
    let query = query.trim();
    if query.is_empty() {
        return None;
    }
    QueryMatcher::new(query, SearchOptions::default()).ok()
}

fn later_note(query: &str, width: u16, theme: &ThemeStyles) -> Vec<Line<'static>> {
    let note =
        format!(" \"{}\" comes later in this session than the preview reaches", query.trim());
    let mut lines = crate::widgets::wrap_line(&Line::styled(note, theme.muted), usize::from(width));
    lines.push(Line::default());
    lines
}

/// The kept turns a `height` pane can show, wrapped to `width`: from the turn before the first that
/// says the query (or the start) to a pane past that turn. The session's flattened text when none
/// were kept.
///
/// A session keeps up to 16 KiB of text, too much to render as markdown on every frame for the
/// few lines the pane shows.
fn body_lines(
    session: &SessionEntry,
    matcher: Option<&QueryMatcher>,
    width: u16,
    height: u16,
    theme: &ThemeStyles,
) -> Vec<Line<'static>> {
    if session.preview.is_empty() {
        return plain_text(&session.search_text, width, theme);
    }
    let saying = matcher.and_then(|matcher| {
        session.preview.iter().position(|turn| !matcher.find_in(&turn.text).is_empty())
    });
    let start = saying.map_or(0, |at| at.saturating_sub(1));
    let through = saying.unwrap_or(0) - start;
    draw_turns(&session.preview[start..], through, height, width, theme)
}

/// `turns` drawn in order, blank-separated, until a `height` pane past turn `through`.
fn draw_turns(
    turns: &[PreviewTurn],
    through: usize,
    height: u16,
    width: u16,
    theme: &ThemeStyles,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut enough = usize::MAX;
    for (index, turn) in turns.iter().enumerate() {
        if lines.len() >= enough {
            break;
        }
        push_turn(&mut lines, turn_lines(turn, width, theme));
        if index == through {
            enough = lines.len() + usize::from(height);
        }
    }
    lines
}

/// Adds a turn's lines, a blank line from the one before.
fn push_turn(lines: &mut Vec<Line<'static>>, drawn: Vec<Line<'static>>) {
    if !drawn.is_empty() && !lines.is_empty() {
        lines.push(Line::default());
    }
    lines.extend(drawn);
}

/// One turn. A tool turn is the one line the catalog keeps of it, since its card needs the call's
/// full record.
fn turn_lines(turn: &PreviewTurn, width: u16, theme: &ThemeStyles) -> Vec<Line<'static>> {
    let block = match turn.role {
        PreviewRole::User => {
            ConvoBlock::User { text: turn.text.clone(), attached: Vec::new(), chips: Vec::new() }
        }
        PreviewRole::Assistant => ConvoBlock::Assistant(turn.text.clone()),
        PreviewRole::Tool => return vec![tool_line(&turn.text, width, theme)],
    };
    crate::view::excerpt_lines(&[block], width, theme)
}

fn tool_line(text: &str, width: u16, theme: &ThemeStyles) -> Line<'static> {
    // Blanks where a tier has no icon: the pane shares its rows with the list, and a line that
    // moves by the icon's width in one tier only would move against the list beside it
    let glyph = match Icon::ToolGeneric.prefix() {
        prefix if prefix.is_empty() => "  ".to_string(),
        prefix => prefix,
    };
    let room = usize::from(width)
        .saturating_sub(1 + unicode_width::UnicodeWidthStr::width(glyph.as_str()));
    Line::from(vec![
        Span::raw(" "),
        Span::styled(glyph, theme.warning),
        Span::styled(crate::widgets::truncate_end(text, room), theme.dim),
    ])
}

fn plain_text(text: &str, width: u16, theme: &ThemeStyles) -> Vec<Line<'static>> {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.is_empty() {
        return vec![Line::styled(" No message preview", theme.muted)];
    }
    crate::widgets::wrap_line(&Line::styled(flat, theme.muted), usize::from(width))
}

#[cfg(test)]
mod tests;
