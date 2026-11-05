//! Telemetry — the sidebar's MODEL and SESSION sections, as the TUI's rail draws them.
//!
//! The rail's grid of muted labels and values, under the same badges the sidebar's other parts
//! wear: MODEL lit as the rail lights it, SESSION recessed. The rows' words come from the
//! [`Renderer`], which `luxe-cli` hands the TUI's; a section with no rows draws nothing, so the
//! app leaves it out, as the rail leaves out a SESSION with nothing measured yet.

use super::{escape, Renderer, StatusFacts};

/// One `label  value` row of a section, as words; the bridge escapes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryRow {
    /// The muted word in the label column
    pub label: &'static str,
    /// The mark in front of the value, if any
    pub glyph: Option<String>,
    /// The value
    pub value: String,
    /// How to color the value
    pub tone: TelemetryTone,
}

/// How a row's value is colored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryTone {
    /// Body text
    Plain,
    /// Dimmed, for a stand-in until the value is reported
    Dim,
    /// The theme's warning color
    Warning,
    /// The theme's error color
    Error,
    /// The color of the mode the value names
    Mode,
}

/// The MODEL section for `mode`, or nothing from a renderer without the rail's words.
pub(super) fn model(renderer: &dyn Renderer, facts: &StatusFacts, mode: &str) -> String {
    section("MODEL", "badge", &renderer.model_rows(facts, mode))
}

/// The SESSION section, or nothing until a turn has something to measure.
pub(super) fn session(renderer: &dyn Renderer, facts: &StatusFacts) -> String {
    section("SESSION", "badge recessed", &renderer.session_rows(facts))
}

fn section(title: &str, badge: &str, rows: &[TelemetryRow]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let rows: String = rows.iter().map(row).collect();
    format!(
        "<p class=\"dock-head\"><span class=\"{badge}\">{title}</span></p>\
         <dl class=\"telemetry\">{rows}</dl>"
    )
}

fn row(row: &TelemetryRow) -> String {
    let glyph = row.glyph.as_deref().map(|glyph| {
        format!("<span class=\"tm-glyph\" aria-hidden=\"true\">{}</span>", escape(glyph))
    });
    format!(
        "<div class=\"tm-row\"><dt>{}</dt><dd class=\"{}\">{}{}</dd></div>",
        escape(row.label),
        tone_class(row),
        glyph.unwrap_or_default(),
        escape(&row.value)
    )
}

/// The class the stylesheet colors a value by. A mode's is its name, which the composer's mode
/// button is colored by too.
fn tone_class(row: &TelemetryRow) -> String {
    match row.tone {
        TelemetryTone::Plain => "hue-text".to_owned(),
        TelemetryTone::Dim => "dim".to_owned(),
        TelemetryTone::Warning => "tone-warning".to_owned(),
        TelemetryTone::Error => "tone-error".to_owned(),
        TelemetryTone::Mode => format!("mode-{}", escape(&row.value)),
    }
}

#[cfg(test)]
mod tests;
