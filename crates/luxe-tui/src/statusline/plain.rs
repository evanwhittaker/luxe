//! Plain — the status line's facts as words, for a surface that isn't a terminal.
//!
//! The phone app shows what the bar shows, so it takes the bar's own wording from here rather than
//! keeping a copy that could drift: the context reading and its ramp, the cost slot's choice
//! between a plan's allowance and the session's spend, the thinking level's short name, the pull
//! request and plan on the left (`left`), and the surface each module sits on, so the app can cut
//! the same slabs. Fitting and painting them is the app's; glyphs are the Nerd Font marks, which
//! the app's font always has.

pub mod dock;
mod left;

pub use left::{left_segments, TodoProgress};

use super::segments::{context_text, cost_text, tokens_text};
use super::{context_percent, thinking_label, SegmentKind};
use crate::glyphs::{GlyphTier, Icon, Mark};
use crate::theme::styles::{ContextFill, Depth};
use luxe_core::github::PullRequestStatus;
use luxe_core::provider::{PlanUsage, ThinkingLevel};
use luxe_core::usage::SessionUsage;
use luxe_core::ModelRef;

/// What a session last reported, each part absent until its event arrives.
#[derive(Debug, Default, Clone, Copy)]
pub struct Facts<'a> {
    /// The model answering, from `state_changed`
    pub model: Option<&'a ModelRef>,
    /// Its thinking level, from `state_changed`
    pub thinking: Option<ThinkingLevel>,
    /// Tokens in the window and the window's size, from `context_usage`
    pub context: Option<(u32, u32)>,
    /// A subscription's allowance, from `plan_usage`
    pub plan: Option<&'a PlanUsage>,
    /// The session's spend, from `session_usage`
    pub session: Option<&'a SessionUsage>,
    /// The running turn's tokens, summed from its `turn_usage` events
    pub turn: Option<TurnTokens>,
    /// The branch's pull request, from `workspace_updated`, when it has one
    pub pull_request: Option<&'a PullRequestStatus>,
    /// How far the plan has got, from `workspace_updated`
    pub todos: Option<TodoProgress<'a>>,
    /// Files changed in the worktree, from `workspace_updated`
    pub changed_files: usize,
}

/// Tokens one turn has used so far, which the bar shows rather than the session's.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TurnTokens {
    /// Input sent
    pub prompt: u64,
    /// Output received
    pub completion: u64,
    /// Input served from the provider's cache
    pub cache_read: u64,
}

/// One fact as the bar words it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// Which fact this is
    pub kind: Kind,
    /// The mark in front of it, if the bar draws one
    pub glyph: Option<&'static str>,
    /// The words
    pub label: String,
    /// How loudly to color it
    pub tone: Tone,
    /// What it sits on, as the bar's slab style draws it
    pub surface: Surface,
}

/// The facts, in the bar's order, left to right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The branch's pull request; a warning tone is its mark's, as the bar tints only the mark
    PullRequest,
    /// How far the plan has got
    Todo,
    /// How many files have changed
    Changes,
    /// How full the context window is
    Context,
    /// A plan's allowance or the session's spend
    Cost,
    /// The running turn's tokens
    Tokens,
    /// The thinking level
    Thinking,
    /// The model
    Model,
}

/// What a module sits on: straight on the bar, or a slab receding from the accent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// No fill, as the context reading has, whose color is its datum
    Bare,
    /// The accent itself, inked in the canvas
    Hero,
    /// One step back
    Raised,
    /// Two steps back, for telemetry
    Recessed,
}

/// The surface the bar gives `kind`: the TUI's own assignment, so the two never disagree.
fn surface(kind: Kind) -> Surface {
    let kind = match kind {
        Kind::PullRequest => SegmentKind::PullRequest,
        Kind::Todo => SegmentKind::Todo,
        Kind::Changes => SegmentKind::Changes,
        Kind::Context => SegmentKind::Context,
        Kind::Cost => SegmentKind::Cost,
        Kind::Tokens => SegmentKind::Tokens,
        Kind::Thinking => SegmentKind::Thinking,
        Kind::Model => SegmentKind::Model,
    };
    match kind.surface() {
        super::Surface::Bare => Surface::Bare,
        super::Surface::Slab(Depth::Hero) => Surface::Hero,
        super::Surface::Slab(Depth::Raised) => Surface::Raised,
        super::Surface::Slab(Depth::Recessed) => Surface::Recessed,
    }
}

/// How loudly a segment is colored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Body text
    Plain,
    /// The theme's warning color
    Warning,
    /// The theme's error color
    Error,
}

/// The facts the session has reported, worded and ordered as the bar's right-hand side.
pub fn segments(facts: &Facts<'_>) -> Vec<Segment> {
    [context(facts), cost(facts), tokens(facts), thinking(facts), model(facts)]
        .into_iter()
        .flatten()
        .collect()
}

fn segment(kind: Kind, glyph: Option<Icon>, label: String, tone: Tone) -> Segment {
    let glyph = glyph.map(|icon| icon.at(GlyphTier::Nerd));
    Segment { kind, glyph, label, tone, surface: surface(kind) }
}

fn context(facts: &Facts<'_>) -> Option<Segment> {
    let (used, window) = facts.context.filter(|(_, window)| *window > 0)?;
    let percent = context_percent(used, window);
    let tone = match ContextFill::at(percent) {
        ContextFill::Roomy => Tone::Plain,
        ContextFill::Filling => Tone::Warning,
        ContextFill::Full => Tone::Error,
    };
    Some(segment(Kind::Context, Some(Icon::Context), context_text(percent, window), tone))
}

fn cost(facts: &Facts<'_>) -> Option<Segment> {
    let label = cost_text(facts.plan, facts.session.unwrap_or(&SessionUsage::default()))?;
    Some(segment(Kind::Cost, None, label, Tone::Plain))
}

/// Only once the turn has used some, as on the bar.
fn tokens(facts: &Facts<'_>) -> Option<Segment> {
    let turn = facts.turn.filter(|turn| turn.prompt > 0 || turn.completion > 0)?;
    let reuse = Mark::CacheReuse.at(GlyphTier::Nerd);
    let label = tokens_text((turn.prompt, turn.completion), turn.cache_read, reuse);
    Some(segment(Kind::Tokens, None, label, Tone::Plain))
}

fn thinking(facts: &Facts<'_>) -> Option<Segment> {
    let level = facts.thinking?;
    facts.model.filter(|model| !model.model.is_empty())?;
    Some(segment(Kind::Thinking, Some(Icon::Thinking), thinking_label(&level).into(), Tone::Plain))
}

// Always there, as on the bar: with no model, saying so explains a refused turn
fn model(facts: &Facts<'_>) -> Option<Segment> {
    let (label, tone) = match facts.model.filter(|model| !model.model.is_empty()) {
        Some(model) => (model.display().to_string(), Tone::Plain),
        None => ("no model".to_string(), Tone::Warning),
    };
    Some(segment(Kind::Model, Some(Icon::Model), label, tone))
}

#[cfg(test)]
mod tests;
