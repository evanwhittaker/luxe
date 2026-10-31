//! Usage — the USAGE block: one text row per plan window, between MODEL and SESSION.

use super::super::usage::window_value;
use super::*;
use luxe_core::provider::{PlanUsage, PlanWindow};
use std::time::{Duration, Instant};

fn window(used_percent: f32, window_minutes: u64, resets_in_secs: Option<u64>) -> PlanWindow {
    PlanWindow { used_percent, window_minutes, resets_in_secs }
}

/// A weekly window and a five-hour one, as a ChatGPT plan reports them, arrived at `at`.
fn plan_app(at: Instant) -> App {
    let mut app = app();
    let plan = PlanUsage {
        plan: Some("plus".into()),
        primary: Some(window(30.0, 10_080, Some(273_600))),
        secondary: Some(window(80.0, 300, Some(1_800))),
        credits_balance: None,
    };
    app.status.frame_clock = Some(at);
    app.status.on_plan_usage(plan);
    app
}

fn rows(app: &App) -> Vec<String> {
    telemetry_lines(app).iter().map(text).collect()
}

fn row_starting(app: &App, label: &str) -> Option<Line<'static>> {
    telemetry_lines(app).into_iter().find(|line| text(line).starts_with(label))
}

#[test]
fn no_plan_draws_no_usage_block() {
    let app = app();
    assert!(!rows(&app).iter().any(|row| row.contains("USAGE")), "{:?}", rows(&app));
}

/// A plan name alone has no window to show, so it is no reason to draw the heading.
#[test]
fn a_plan_with_no_windows_draws_no_usage_block() {
    let mut app = app();
    let named = PlanUsage { plan: Some("plus".into()), ..PlanUsage::default() };
    app.status.on_plan_usage(named);
    assert!(!rows(&app).iter().any(|row| row.contains("USAGE")), "{:?}", rows(&app));
}

#[test]
fn each_window_is_a_row_under_its_own_heading_after_model() {
    let mut app = plan_app(Instant::now());
    app.status.context_window = 200_000;
    let rows = rows(&app);
    let at = |needle: &str| rows.iter().position(|row| row.contains(needle));
    let (model, usage, session) = (at("MODEL"), at("USAGE"), at("SESSION"));
    assert!(model < usage && usage < session, "MODEL, USAGE, SESSION in order: {rows:?}");
    let weekly = row_starting(&app, "weekly").map(|line| text(&line));
    assert!(weekly.is_some_and(|row| row.ends_with("30% · resets 3d 4h")), "{rows:?}");
    let short = row_starting(&app, "5h").map(|line| text(&line));
    assert!(short.is_some_and(|row| row.ends_with("80% · resets 30m")), "{rows:?}");
}

/// The value takes the context row's ramp: body text with room left, the warning hue as it fills.
#[test]
fn a_filling_window_takes_the_warning_hue() {
    let app = plan_app(Instant::now());
    let ink = |label: &str| row_starting(&app, label).and_then(|line| line.spans[1].style.fg);
    assert_eq!(ink("5h"), app.theme_styles.warning.fg, "80% is past the warning mark");
    assert_eq!(ink("weekly"), app.theme_styles.text.fg, "30% is not news");
}

/// The provider counts down from when it answered; the dock keeps counting after that.
#[test]
fn the_reset_counts_down_from_when_the_plan_reported() {
    let reported = Instant::now();
    let mut app = plan_app(reported);
    app.status.frame_clock = Some(reported + Duration::from_secs(25 * 60));
    let short = row_starting(&app, "5h").map(|line| text(&line));
    assert!(short.is_some_and(|row| row.ends_with("80% · resets 5m")), "{:?}", rows(&app));
}

/// The rail draws a level as a figure; a block element here would be a bar back in the dock.
#[test]
fn a_window_is_a_figure_rather_than_a_bar() {
    let app = plan_app(Instant::now());
    for row in rows(&app).iter().filter(|row| row.starts_with("weekly") || row.starts_with("5h")) {
        assert!(!row.chars().any(|c| ('\u{2580}'..='\u{259F}').contains(&c)), "{row:?}");
    }
}

#[test]
fn the_reset_gives_up_its_word_before_itself() {
    let reset = || Some("6d 23h".to_string());
    assert_eq!(window_value(100.0, reset(), 25), "100% · resets 6d 23h");
    assert_eq!(window_value(100.0, reset(), 15), "100% · 6d 23h");
    assert_eq!(window_value(100.0, reset(), 8), "100%");
}

#[test]
fn a_window_with_no_reset_shows_only_its_share() {
    assert_eq!(window_value(12.0, None, 25), "12%");
}
