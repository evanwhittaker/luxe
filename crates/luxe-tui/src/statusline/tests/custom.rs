//! Custom statusline modules on the bar: where they draw, what they say, and what gives way.

use super::{app, fragment_foreground, plain, status_line};
use crate::app::App;
use luxe_core::config::{CustomStatuslineModule, StatuslineModule};
use luxe_core::custom_dock::{DockAnswer, DockInk, DockRow, DockSpan};

fn module(name: &str) -> StatuslineModule {
    StatuslineModule::Other(name.into())
}

/// An app with `right` on the right of the bar and each of `defined` set up as a custom module.
fn bar(right: Vec<StatuslineModule>, defined: &[&str]) -> App {
    let mut app = app();
    app.status.model = "claude-sonnet".into();
    app.config.statusline.right = Some(right);
    let custom =
        defined.iter().map(|name| (name.to_string(), CustomStatuslineModule::default())).collect();
    app.config.statusline.custom = Some(custom);
    app
}

fn answer(app: &mut App, name: &str, spans: &[(&str, DockInk)]) {
    let spans = spans.iter().map(|(text, ink)| DockSpan { text: text.to_string(), ink: *ink });
    let row = DockRow { label: Some("5h".into()), spans: spans.collect() };
    let answer = DockAnswer { name: name.into(), rows: vec![row], failure: None };
    app.status.custom_statusline.record(answer);
}

#[test]
fn a_custom_module_draws_its_first_row_with_its_label() {
    let mut app = bar(vec![module("quota"), StatuslineModule::Model], &["quota"]);
    answer(&mut app, "quota", &[("▰▰▱ ", DockInk::Accent), ("62%", DockInk::Warning)]);
    let text = plain(&status_line(&app, 120));
    assert!(text.contains("5h ▰▰▱ 62%"), "{text}");
    assert!(text.contains("claude-sonnet"), "the built-ins still draw: {text}");
}

/// A span's ink is the program's signal, so it keeps the theme's color for that role.
#[test]
fn a_span_keeps_the_color_of_its_ink() {
    let mut app = bar(vec![module("quota")], &["quota"]);
    answer(&mut app, "quota", &[("62%", DockInk::Warning)]);
    let warning = app.theme_styles.warning.fg.expect("the theme has a warning color");
    assert_eq!(fragment_foreground(&status_line(&app, 120), "62%"), warning);
}

/// Nothing until the program first answers: a module isn't a placeholder.
#[test]
fn a_module_with_no_answer_yet_draws_nothing() {
    let app = bar(vec![module("quota"), StatuslineModule::Model], &["quota"]);
    let text = plain(&status_line(&app, 120));
    assert!(!text.contains("quota"), "{text}");
}

/// A first run that fails says why on the bar; one after a good run keeps the good row.
#[test]
fn a_failure_says_why_until_there_is_a_row_to_keep() {
    let mut app = bar(vec![module("quota")], &["quota"]);
    let failed =
        || DockAnswer { name: "quota".into(), rows: vec![], failure: Some("exited 1".into()) };
    app.status.custom_statusline.record(failed());
    assert!(plain(&status_line(&app, 120)).contains("quota: exited 1"));
    answer(&mut app, "quota", &[("62%", DockInk::Text)]);
    app.status.custom_statusline.record(failed());
    let text = plain(&status_line(&app, 120));
    assert!(text.contains("62%") && !text.contains("exited"), "{text}");
}

/// How a custom module replaces a built-in: the name resolves to the definition first.
#[test]
fn a_custom_module_replaces_a_built_in_of_the_same_name() {
    let mut app = bar(vec![StatuslineModule::Model], &["model"]);
    answer(&mut app, "model", &[("mine", DockInk::Text)]);
    let text = plain(&status_line(&app, 120));
    assert!(text.contains("mine") && !text.contains("claude-sonnet"), "{text}");
}

/// Two custom modules are two modules, not a duplicate of one kind.
#[test]
fn two_custom_modules_both_draw() {
    let mut app = bar(vec![module("one"), module("two")], &["one", "two"]);
    answer(&mut app, "one", &[("first", DockInk::Text)]);
    answer(&mut app, "two", &[("second", DockInk::Text)]);
    let text = plain(&status_line(&app, 120));
    assert!(text.contains("first") && text.contains("second"), "{text}");
}

#[test]
fn a_long_row_is_cut_to_the_module_width() {
    let mut app = bar(vec![module("quota")], &["quota"]);
    answer(&mut app, "quota", &[(&"x".repeat(80), DockInk::Text)]);
    let text = plain(&status_line(&app, 200));
    assert!(text.contains(&"x".repeat(20)), "{text}");
    assert!(!text.contains(&"x".repeat(40)), "capped well short of 80: {text}");
}

/// A module someone chose outlasts the built-in extras, the plan reading among them, which the
/// dock and `/cost` both carry too.
#[test]
fn a_custom_module_outlasts_the_cost_reading() {
    let right = vec![StatuslineModule::Cost, module("quota"), StatuslineModule::Model];
    let mut app = bar(right, &["quota"]);
    let weekly = luxe_core::provider::PlanWindow {
        used_percent: 12.0,
        window_minutes: 10_080,
        resets_in_secs: None,
    };
    app.status.plan_usage =
        Some(luxe_core::provider::PlanUsage { primary: Some(weekly), ..Default::default() });
    answer(&mut app, "quota", &[("62%", DockInk::Text)]);
    let at = |width: usize| plain(&status_line(&app, width));
    assert!(at(160).contains("weekly") && at(160).contains("62%"), "{}", at(160));
    let kept_over_cost = (20..160).any(|w| at(w).contains("62%") && !at(w).contains("weekly"));
    assert!(kept_over_cost, "the cost goes first");
}

/// On a narrow bar a custom module outlasts the token counts but gives way before the model.
#[test]
fn a_custom_module_gives_way_after_the_trivia_and_before_the_anchors() {
    let right = vec![StatuslineModule::Tokens, module("quota"), StatuslineModule::Model];
    let mut app = bar(right, &["quota"]);
    app.status.prompt_tokens = 120_000;
    app.status.completion_tokens = 4_000;
    answer(&mut app, "quota", &[("62% of the window", DockInk::Text)]);
    let at = |width: usize| plain(&status_line(&app, width));
    assert!(at(160).contains("↑120k") && at(160).contains("62%"), "{}", at(160));
    let kept_over_tokens = (20..160).any(|w| at(w).contains("62%") && !at(w).contains("↑120k"));
    assert!(kept_over_tokens, "the tokens go first");
    let narrow = at(30);
    assert!(narrow.contains("claude-sonnet") && !narrow.contains("62%"), "{narrow}");
}
