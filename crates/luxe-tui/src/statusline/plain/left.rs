//! Left — the bar's left-hand facts as words: the branch's pull request, the plan's progress and
//! the change count.
//!
//! The project and branch lead the bar's left side, but a surface that heads its screen with them
//! has no use for them twice, so these are what's left. The change count drops the bar's key for
//! opening the change: a key is no use on a phone.

use super::{segment, Facts, Kind, Segment, Tone};
use crate::glyphs::Icon;
use crate::statusline::segments::{files_text, plan_text};

/// How far a session's checklist has got: the counts, and the todo it says is in hand.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TodoProgress<'a> {
    /// Todos done
    pub completed: usize,
    /// Todos in all
    pub total: usize,
    /// The subject of the one in hand, if any
    pub current: Option<&'a str>,
}

/// The facts the session has reported, worded and ordered as the bar's left-hand side after the
/// project and branch.
pub fn left_segments(facts: &Facts<'_>) -> Vec<Segment> {
    [pull_request(facts), todo(facts), changes(facts)].into_iter().flatten().collect()
}

fn pull_request(facts: &Facts<'_>) -> Option<Segment> {
    let pr = facts.pull_request?;
    let tone = if crate::pull_request::wants_attention(pr) { Tone::Warning } else { Tone::Plain };
    Some(segment(Kind::PullRequest, Some(Icon::PullRequest), crate::pull_request::label(pr), tone))
}

fn todo(facts: &Facts<'_>) -> Option<Segment> {
    let todos = facts.todos?;
    let label = plan_text(todos.completed, todos.total, todos.current)?;
    Some(segment(Kind::Todo, Some(Icon::Plan), label, Tone::Plain))
}

/// Nothing while the worktree is clean, as on the bar.
fn changes(facts: &Facts<'_>) -> Option<Segment> {
    let files = Some(facts.changed_files).filter(|files| *files > 0)?;
    Some(segment(Kind::Changes, Some(Icon::Changes), files_text(files), Tone::Plain))
}

#[cfg(test)]
mod tests;
