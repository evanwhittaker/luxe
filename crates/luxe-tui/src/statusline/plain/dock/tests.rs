use super::*;
use crate::glyphs::Icon;
use luxe_core::provider::ThinkingLevel;
use luxe_core::usage::SessionUsage;
use luxe_core::ModelRef;

/// `state_changed` and `session_usage` as a Copilot session sent them over the bridge.
const MODEL: &str = r#"{"provider":"github-copilot","model":"claude-haiku-4.5","display_name":"Claude Haiku 4.5 (latest)"}"#;
const SESSION: &str = r#"{"total":{"calls":14,"prompt_tokens":122790,"completion_tokens":2212,"cache_read_tokens":103435,"cache_write_tokens":19237,"reasoning_tokens":888,"cost_usd":0.0,"unpriced_calls":0,"plan_usd":0.04556775,"plan_calls":14},"by_source":[]}"#;

fn parse<T: serde::de::DeserializeOwned>(json: &str) -> T {
    serde_json::from_str(json).expect("a literal copied from a real frame parses")
}

fn words(rows: &[Row]) -> Vec<(&str, &str, RowTone)> {
    rows.iter().map(|row| (row.label, row.value.as_str(), row.tone)).collect()
}

const PLAIN: RowTone = RowTone::Bar(Tone::Plain);

/// The break this catches: a row dropped, crossed or reordered from the rail's provider, model,
/// thinking, mode; the model named by its id where config gave it a name; or the mode not colored
/// as the mode it names.
#[test]
fn model_rows_read_as_the_rail_words_them() {
    let model: ModelRef = parse(MODEL);
    let facts =
        Facts { model: Some(&model), thinking: Some(ThinkingLevel::High), ..Facts::default() };
    let rows = model_rows(&facts, "plan");
    let expected = [
        ("provider", "github-copilot", PLAIN),
        ("model", "Claude Haiku 4.5 (latest)", PLAIN),
        ("thinking", "high", PLAIN),
        ("mode", "plan", RowTone::Mode),
    ];
    assert_eq!(words(&rows), expected);
    assert_eq!(rows[3].glyph, Some(Icon::ModePlan.at(GlyphTier::Nerd)));
}

/// The break this catches: the rows under the provider moving up before one is named, or a
/// session with no model left without saying so, when that's what explains a refused turn.
#[test]
fn before_a_model_the_rows_keep_their_place_and_say_so() {
    let rows = model_rows(&Facts::default(), "");
    let expected =
        [("provider", "none", RowTone::Dim), ("model", "no model", RowTone::Bar(Tone::Warning))];
    assert_eq!(words(&rows), expected, "no thinking or mode row until each is reported");
}

/// The break this catches: the brand marks chosen by this terminal's font, so a host in an older
/// terminal sends the phone a plain mark where its font has Copilot's and Claude's.
#[test]
fn brand_marks_are_the_ones_the_apps_font_has() {
    let model: ModelRef = parse(MODEL);
    crate::glyphs::set_brands(false);
    let rows = model_rows(&Facts { model: Some(&model), ..Facts::default() }, "");
    assert_eq!(rows[0].glyph, Some(Icon::ProviderCopilot.at(GlyphTier::Nerd)));
    assert_eq!(rows[1].glyph, Some(Icon::ModelClaude.at(GlyphTier::Nerd)));
}

/// The break this catches: the window, tokens or cost worded otherwise than the rail, cache reuse
/// cut for room a page doesn't need, or plan-billed calls shown as nothing rather than their worth.
#[test]
fn session_rows_read_as_the_rail_words_them() {
    let session: SessionUsage = parse(SESSION);
    let facts =
        Facts { context: Some((9613, 200_000)), session: Some(&session), ..Facts::default() };
    let reuse = Mark::CacheReuse.at(GlyphTier::Nerd);
    let tokens = format!("↑122k ↓2.2k {reuse}103k");
    let expected =
        [("context", "5% of 200k", PLAIN), ("tokens", &tokens, PLAIN), ("cost", "≈$0.05", PLAIN)];
    assert_eq!(words(&session_rows(&facts)), expected);
}

/// The break this catches: the section drawn before a turn has measured anything, or a window of
/// unknown size read as a division by zero.
#[test]
fn nothing_measured_is_no_session_rows() {
    assert_eq!(session_rows(&Facts::default()), []);
    assert_eq!(session_rows(&Facts { context: Some((10, 0)), ..Facts::default() }), []);
    let idle = SessionUsage::default();
    assert_eq!(session_rows(&Facts { session: Some(&idle), ..Facts::default() }), []);
}

/// The break this catches: the window colored on another ramp than the bar's.
#[test]
fn the_window_colors_on_the_bars_ramp() {
    let tone =
        |used| session_rows(&Facts { context: Some((used, 1000)), ..Facts::default() })[0].tone;
    let expected = [Tone::Plain, Tone::Warning, Tone::Error].map(RowTone::Bar);
    assert_eq!([tone(749), tone(750), tone(900)], expected);
}

/// The break this catches: a cache reuse of none drawn as `↺0`.
#[test]
fn no_cache_reuse_draws_no_reuse() {
    let totals =
        UsageTotals { prompt_tokens: 1200, completion_tokens: 30, ..UsageTotals::default() };
    assert_eq!(tokens_row(&totals).value, "↑1.2k ↓30");
}
