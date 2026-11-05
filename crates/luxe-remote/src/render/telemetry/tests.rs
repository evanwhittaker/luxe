use crate::render::{escape, render, Piece, Renderer, StatusFacts, StatusSegment, Voice};
use crate::{TelemetryRow, TelemetryTone};

/// Rows that echo what reached them, with markup in them so escaping shows.
struct Echo;

impl Renderer for Echo {
    fn markdown(&self, text: &str, _voice: Voice) -> String {
        escape(text)
    }
    fn tool_glyph(&self, _tool: &str) -> String {
        String::new()
    }
    fn status(&self, _facts: &StatusFacts) -> Vec<StatusSegment> {
        Vec::new()
    }
    fn model_rows(&self, facts: &StatusFacts, mode: &str) -> Vec<TelemetryRow> {
        let model = facts.model.as_ref().map_or("", |model| model.model.as_str());
        vec![
            echoed("model", Some("<m>"), model, TelemetryTone::Plain),
            echoed("mode", None, mode, TelemetryTone::Mode),
        ]
    }
    fn session_rows(&self, facts: &StatusFacts) -> Vec<TelemetryRow> {
        let Some(context) = facts.context else { return Vec::new() };
        let used = context.used_tokens.to_string();
        vec![
            echoed("context", None, &used, TelemetryTone::Error),
            echoed("<cost>", None, "none", TelemetryTone::Dim),
            echoed("tokens", None, "many", TelemetryTone::Warning),
        ]
    }
}

fn echoed(
    label: &'static str,
    glyph: Option<&str>,
    value: &str,
    tone: TelemetryTone,
) -> TelemetryRow {
    TelemetryRow { label, glyph: glyph.map(str::to_owned), value: value.to_owned(), tone }
}

/// `piece` as the app sends it, rendered by [`Echo`].
fn rendered(piece: serde_json::Value) -> String {
    let piece: Piece = serde_json::from_value(piece).expect("the app's piece parses");
    render(&Echo, &piece)
}

/// The break this catches: the app's piece not parsing (the mode or the facts beside the kind
/// lost), the MODEL badge not lit as the rail's is, a row's mark or value left unescaped, or the
/// mode's value not colored by the class the composer's mode button uses.
#[test]
fn the_model_section_is_the_rails_grid_under_its_badge() {
    let html = rendered(serde_json::json!({
        "kind": "model",
        "model": {"provider": "anthropic", "model": "claude-<x>"},
        "thinking": "high",
        "mode": "plan",
    }));
    let expected = "<p class=\"dock-head\"><span class=\"badge\">MODEL</span></p>\
        <dl class=\"telemetry\">\
        <div class=\"tm-row\"><dt>model</dt><dd class=\"hue-text\">\
        <span class=\"tm-glyph\" aria-hidden=\"true\">&lt;m&gt;</span>claude-&lt;x&gt;</dd></div>\
        <div class=\"tm-row\"><dt>mode</dt><dd class=\"mode-plan\">plan</dd></div></dl>";
    assert_eq!(html, expected);
}

/// The break this catches: SESSION lit as loudly as MODEL, where the rail recesses it; a label
/// left unescaped; or a tone's class misspelled so the stylesheet can't color it.
#[test]
fn the_session_section_is_recessed_and_toned() {
    let html = rendered(serde_json::json!({
        "kind": "session",
        "context": {"used_tokens": 950, "context_window": 1000},
    }));
    assert!(
        html.starts_with("<p class=\"dock-head\"><span class=\"badge recessed\">SESSION</span>"),
        "{html}"
    );
    assert!(html.contains("<dt>context</dt><dd class=\"tone-error\">950</dd>"), "{html}");
    assert!(html.contains("<dt>&lt;cost&gt;</dt><dd class=\"dim\">none</dd>"), "{html}");
    assert!(html.contains("<dd class=\"tone-warning\">many</dd>"), "{html}");
}

/// The break this catches: a heading drawn over no rows, so the sidebar shows a SESSION with
/// nothing under it before a turn has measured anything, where the rail leaves it out.
#[test]
fn a_section_with_no_rows_draws_nothing() {
    assert_eq!(rendered(serde_json::json!({ "kind": "session" })), "");
}
