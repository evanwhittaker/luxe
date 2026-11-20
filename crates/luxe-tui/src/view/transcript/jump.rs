//! Message jump — scrolls the transcript to a message Find went to, on its first match.
//!
//! Find knows the message, not where it is drawn. The search that Enter opened carries the
//! message's id ([`crate::search::SearchState::jump_to`]), and the first frame laid out after it
//! turns that into a line: the message's first line the search matches, which becomes the
//! current match so `n` and `N` go on from it.

use super::RenderedConversationLine;
use crate::app::{App, ConversationItemId};

/// How far down the view the line lands, in parts of its height, so the lines before it show.
const LANDS_AT: usize = 3;

/// Scrolls to the message the search is waiting to go to, if it is waiting.
pub(super) fn resolve_jump(app: &mut App, rendered_lines: &[RenderedConversationLine]) {
    let Some(item) = app.search_state.jump_to.take() else { return };
    app.search_state.search(rendered_lines.iter().map(|rendered| rendered.line.as_ref()));
    let Some(line) = jump_line(app, rendered_lines, &item) else { return };
    let viewport = app.scroll_state.viewport;
    app.scroll_state.follow = false;
    app.scroll_state.offset =
        line.saturating_sub(viewport / LANDS_AT).min(app.scroll_state.max_offset());
}

/// The message's first matching line, made the current match; its first line when the search
/// finds nothing drawn in it, as when the match sits in a folded part.
fn jump_line(
    app: &mut App,
    rendered_lines: &[RenderedConversationLine],
    item: &ConversationItemId,
) -> Option<usize> {
    let in_item =
        |line: usize| rendered_lines.get(line).is_some_and(|rendered| *rendered.item_id == *item);
    let search = &mut app.search_state;
    if let Some(at) = search.matches.iter().position(|line| in_item(*line)) {
        search.current_match = Some(at);
        return Some(search.matches[at]);
    }
    rendered_lines.iter().position(|rendered| *rendered.item_id == *item)
}
