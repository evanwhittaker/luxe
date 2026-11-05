//! Dock — the rail's MODEL and SESSION rows as words, for a surface that isn't a terminal.
//!
//! The phone app's sidebar shows the sections the TUI's rail does, so it takes the rail's labels,
//! marks and readings from here. The rail draws them from the app's live state; this draws them
//! from what the session reported over the bridge, which has no auto-approval or guests to show.

use super::{tokens_text, Facts, Tone};
use crate::glyphs::{GlyphTier, Icon, Mark};
use crate::statusline::{compact_number, context_percent, thinking_label};
use crate::theme::styles::ContextFill;
use luxe_core::usage::UsageTotals;

/// Whether the app's font draws the brand marks only the newest Nerd Fonts carry. It's built from
/// one that has them (`scripts/remote-fonts.sh`), whatever font this terminal has.
const RECENT_BRANDS: bool = true;

/// One `label  value` row, as the rail lays it out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The muted word in the label column
    pub label: &'static str,
    /// The mark in front of the value, if the rail draws one
    pub glyph: Option<&'static str>,
    /// The value
    pub value: String,
    /// How to color the value
    pub tone: RowTone,
}

/// How a row's value is colored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowTone {
    /// One of the bar's tones
    Bar(Tone),
    /// Dimmed, for a value that stands in for one not reported yet
    Dim,
    /// The color of the mode the value names
    Mode,
}

/// The MODEL rows: who serves it, the model, its thinking level and the mode.
pub fn model_rows(facts: &Facts<'_>, mode: &str) -> Vec<Row> {
    let mut rows = vec![provider_row(facts), model_row(facts)];
    rows.extend(facts.thinking.map(|level| Row {
        label: "thinking",
        glyph: Some(Icon::Thinking.at(GlyphTier::Nerd)),
        value: thinking_label(&level).to_string(),
        tone: RowTone::Bar(Tone::Plain),
    }));
    if !mode.is_empty() {
        let glyph = Some(crate::modes::icon(mode).at(GlyphTier::Nerd));
        rows.push(Row { label: "mode", glyph, value: mode.to_string(), tone: RowTone::Mode });
    }
    rows
}

/// Always drawn, as on the rail, so the rows under it keep their place: `none`, dimmed, until a
/// provider is named.
fn provider_row(facts: &Facts<'_>) -> Row {
    let provider = facts.model.map_or("", |model| model.provider.as_str());
    let icon = crate::widgets::provider_icon_with(provider, RECENT_BRANDS);
    let (value, tone) = match provider.is_empty() {
        true => ("none".to_string(), RowTone::Dim),
        false => (provider.to_string(), RowTone::Bar(Tone::Plain)),
    };
    Row { label: "provider", glyph: Some(icon.at(GlyphTier::Nerd)), value, tone }
}

/// The model under its maker's mark, or `no model` as a warning, which explains a refused turn.
fn model_row(facts: &Facts<'_>) -> Row {
    let model = facts.model.filter(|model| !model.model.is_empty());
    let icon = crate::widgets::model_icon_with(model.map_or("", |m| &m.model), RECENT_BRANDS);
    let (value, tone) = match model {
        Some(model) => (model.display().to_string(), Tone::Plain),
        None => ("no model".to_string(), Tone::Warning),
    };
    Row { label: "model", glyph: Some(icon.at(GlyphTier::Nerd)), value, tone: RowTone::Bar(tone) }
}

/// The SESSION rows: the window, the tokens and the cost. None until a turn has something to
/// measure, when the rail leaves the section out.
pub fn session_rows(facts: &Facts<'_>) -> Vec<Row> {
    let totals = facts.session.map(|session| &session.total);
    let cost = totals.and_then(|totals| totals.cost_label().or_else(|| totals.plan_label()));
    let mut rows: Vec<Row> = context_row(facts).into_iter().collect();
    rows.extend(totals.filter(|t| t.prompt_tokens > 0 || t.completion_tokens > 0).map(tokens_row));
    rows.extend(cost.map(|value| plain_row("cost", None, value)));
    rows
}

/// `42% of 200k`, on the bar's ramp: the one value that is also a warning.
fn context_row(facts: &Facts<'_>) -> Option<Row> {
    let (used, window) = facts.context.filter(|(_, window)| *window > 0)?;
    let percent = context_percent(used, window);
    let tone = match ContextFill::at(percent) {
        ContextFill::Roomy => Tone::Plain,
        ContextFill::Filling => Tone::Warning,
        ContextFill::Full => Tone::Error,
    };
    let value = format!("{percent:.0}% of {}", compact_number(u64::from(window)));
    let glyph = Some(Icon::Context.at(GlyphTier::Nerd));
    Some(Row { label: "context", glyph, value, tone: RowTone::Bar(tone) })
}

/// `↑2.3M ↓210k ↺98.7M`, the cache reuse only when there is any. A page wraps rather than cuts,
/// so it keeps all three where the rail drops the third for room.
fn tokens_row(totals: &UsageTotals) -> Row {
    let tokens = (totals.prompt_tokens, totals.completion_tokens);
    let reuse = Mark::CacheReuse.at(GlyphTier::Nerd);
    plain_row("tokens", None, tokens_text(tokens, totals.cache_read_tokens, reuse))
}

fn plain_row(label: &'static str, glyph: Option<&'static str>, value: String) -> Row {
    Row { label, glyph, value, tone: RowTone::Bar(Tone::Plain) }
}

#[cfg(test)]
mod tests;
