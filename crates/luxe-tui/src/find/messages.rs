//! Conversation hits — the messages Find's Conversation tab lists, and the pane that shows one.
//!
//! What was said, by the reader or the assistant, is what this tab finds: tool output and system
//! notes are left out, as they have their own cards to look through. The query is plain text,
//! any case, so the conversation's own search that Enter hands it to finds the same lines.

use crate::app::{ConversationItem, ConversationItemId, ConvoBlock};
use crate::theme::styles::ThemeStyles;
use luxe_core::project_search::{MatchRange, QueryMatcher, SearchOptions};
use ratatui::text::Line;

/// The most messages the tab lists; a query common enough to pass it wants more words.
pub(crate) const MAX_MESSAGE_HITS: usize = 200;

/// A message that says the query.
#[derive(Debug, Clone)]
pub(crate) struct MessageHit {
    /// Where it is in the conversation, to go to
    pub(crate) item: ConversationItemId,
    /// The message, as the preview draws it
    pub(crate) block: ConvoBlock,
    /// Who said it: `You` or `Assistant`
    pub(crate) role: &'static str,
    /// The first of its lines that says the query, indent dropped
    pub(crate) line: String,
    /// Characters of `line` the query matched, by char index
    pub(crate) matched: Vec<usize>,
}

/// The messages in `items` that say `query`, newest first.
pub(crate) fn message_hits(items: &[ConversationItem], query: &str) -> Vec<MessageHit> {
    let Some(matcher) = conversation_matcher(query) else { return Vec::new() };
    items
        .iter()
        .rev()
        .filter_map(|item| message_hit(item, &matcher))
        .take(MAX_MESSAGE_HITS)
        .collect()
}

/// `query` read as the conversation's search reads it: plain text, any case. `None` for nothing
/// to look for.
pub(crate) fn conversation_matcher(query: &str) -> Option<QueryMatcher> {
    if query.trim().is_empty() {
        return None;
    }
    let options = SearchOptions { case_sensitive: Some(false), ..SearchOptions::default() };
    QueryMatcher::new(query, options).ok()
}

fn message_hit(item: &ConversationItem, matcher: &QueryMatcher) -> Option<MessageHit> {
    let (role, text) = match item.block() {
        ConvoBlock::User { text, .. } => ("You", text.as_str()),
        ConvoBlock::Assistant(text) => ("Assistant", text.as_str()),
        _ => return None,
    };
    let (line, ranges) = text
        .lines()
        .map(|line| (line, matcher.find_in(line)))
        .find(|(_, ranges)| !ranges.is_empty())?;
    let (line, matched) = trimmed_with_matches(line, &ranges);
    Some(MessageHit { item: item.id.clone(), block: item.block().clone(), role, line, matched })
}

/// `line` without its indent, and the characters of what's left that `ranges` cover.
fn trimmed_with_matches(line: &str, ranges: &[MatchRange]) -> (String, Vec<usize>) {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    let inside = |byte: usize| {
        ranges.iter().any(|range| (range.start..range.end).contains(&(indent + byte)))
    };
    let matched = trimmed
        .char_indices()
        .enumerate()
        .filter(|(_, (byte, _))| inside(*byte))
        .map(|(index, _)| index)
        .collect();
    (trimmed.replace('\t', " "), matched)
}

/// The message `hit` drawn as the transcript draws it, `width` by `height`, scrolled to where it
/// says `query` and lit there.
pub(crate) fn message_preview(
    hit: &MessageHit,
    query: &str,
    width: u16,
    height: u16,
    theme: &ThemeStyles,
) -> Vec<Line<'static>> {
    let lines = crate::view::excerpt_lines(std::slice::from_ref(&hit.block), width, theme);
    let Some(matcher) = conversation_matcher(query) else {
        return crate::match_light::window_on(lines, None, usize::from(height));
    };
    let first = crate::match_light::first_match(&lines, &matcher);
    let shown = crate::match_light::window_on(lines, first, usize::from(height));
    crate::match_light::light_lines(shown, &matcher, theme.match_bg())
}

#[cfg(test)]
mod tests;
