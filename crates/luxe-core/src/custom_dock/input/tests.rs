use super::*;
use crate::event::{EngineState, ModelRef, StopReason};

fn fresh() -> DockInput {
    DockInput::new(&DockInputContext { session_id: "s1".into(), cwd: "/work".into() })
}

fn state(model: &str) -> Event {
    Event::StateChanged(EngineState {
        model: ModelRef::new("chatgpt", model),
        ..Default::default()
    })
}

#[test]
fn the_first_state_report_is_the_session_starting() {
    let mut input = fresh();
    assert!(input.observe(&state("gpt-5.5")), "the first report runs the sections");
    assert!(!input.observe(&state("gpt-5.5")), "later ones only update the model");
    assert!(!input.observe(&state("o3")));
    assert_eq!((input.provider.as_deref(), input.model.as_deref()), (Some("chatgpt"), Some("o3")));
}

/// What fit before a resize may not fit after it, so a new width runs the programs. The same width
/// again, which a redraw sends, must not, and neither may one before there is a model to report.
#[test]
fn a_new_width_runs_the_programs_once_a_model_is_known() {
    let mut input = fresh();
    let widths = RowWidths { dock: 40, dock_label: 15, statusline: 32 };
    assert!(!input.observe(&Event::CustomRowWidths(widths)), "no model to report yet");
    input.observe(&state("gpt-5.5"));
    assert!(!input.observe(&Event::CustomRowWidths(widths)), "the same width again");
    assert!(input.observe(&Event::CustomRowWidths(RowWidths { dock: 60, ..widths })));
    assert_eq!(input.row_widths.map(|widths| widths.dock), Some(60));
}

#[test]
fn a_turn_ending_and_plan_usage_run_the_sections() {
    let mut input = fresh();
    assert!(input.observe(&Event::TurnEnded { reason: StopReason::EndTurn }));
    assert!(input.observe(&Event::PlanUsage(PlanUsage::default())));
}

/// At startup the engine sends the load before its first state report, as a real `--rpc` run
/// does: running on the load would tell the program no model, so the state report runs it instead.
/// Once the session is going, a load is a resume, and runs it.
#[test]
fn a_load_runs_the_sections_only_once_the_model_is_known() {
    let mut input = fresh();
    let loaded = |id: &str| Event::SessionLoaded { session_id: id.into(), messages: vec![] };
    assert!(!input.observe(&loaded("s1")), "startup: wait for the model");
    assert!(input.observe(&state("gpt-5.5")));
    assert!(input.observe(&loaded("s2")), "a resume mid-session runs them");
    assert_eq!(input.session_id, "s2");
}

#[test]
fn usage_figures_are_kept_without_running() {
    let mut input = fresh();
    assert!(!input.observe(&Event::ContextUsage { used_tokens: 50_000, context_window: 200_000 }));
    assert!(!input.observe(&Event::SessionUsage(SessionUsage::default())));
    assert!(!input.observe(&Event::TokenDelta { block: 0, delta: "hi".into() }));
    let fill = input.context.expect("a known window gives a fill");
    assert_eq!((fill.used_tokens, fill.window, fill.used_percent), (50_000, 200_000, 25.0));
    assert!(input.session_usage.is_some());
}

#[test]
fn an_unknown_window_gives_no_fill() {
    let mut input = fresh();
    input.observe(&Event::ContextUsage { used_tokens: 10, context_window: 0 });
    assert_eq!(input.context, None);
}

/// Pinned against the literal a program reads, since programs are written against it.
#[test]
fn the_payload_reads_as_documented() {
    let mut input = fresh();
    input.observe(&state("gpt-5.5"));
    input.observe(&Event::ContextUsage { used_tokens: 50_000, context_window: 200_000 });
    let program = CustomProgram::dock_section("quota", &Default::default());
    let json = serde_json::to_value(input.for_program(&program)).unwrap();
    let expected = serde_json::json!({
        "version": 1,
        "section": "quota",
        "place": "dock",
        "session_id": "s1",
        "cwd": "/work",
        "provider": "chatgpt",
        "model": "gpt-5.5",
        "context": {"used_tokens": 50000, "window": 200000, "used_percent": 25.0},
        "plan_usage": null,
        "session_usage": null
    });
    assert_eq!(json, expected);
}

/// A run after a later turn gets the reset counted from now: a plan reported an hour ago with three
/// hours to go has two left, as the built-in USAGE section says.
#[test]
fn the_plan_resets_are_counted_from_now() {
    let mut input = fresh();
    let window = crate::provider::PlanWindow {
        used_percent: 30.0,
        window_minutes: 300,
        resets_in_secs: Some(10_800),
    };
    input.observe(&Event::PlanUsage(PlanUsage { primary: Some(window), ..PlanUsage::default() }));
    let hour = std::time::Duration::from_secs(3_600);
    input.plan_usage_at = std::time::Instant::now().checked_sub(hour);
    let program = CustomProgram::dock_section("quota", &Default::default());
    let plan = input.for_program(&program).plan_usage.expect("a plan");
    let left = plan.primary.and_then(|window| window.resets_in_secs).expect("a reset");
    assert!((7_195..=7_200).contains(&left), "{left}");
}

/// A statusline module's program is told so, which is how one script answers both places.
#[test]
fn a_statusline_module_is_told_where_it_is_drawn() {
    let program = CustomProgram::statusline_module("quota", &Default::default());
    let json = serde_json::to_value(fresh().for_program(&program)).unwrap();
    assert_eq!((&json["section"], &json["place"]), (&"quota".into(), &"statusline".into()));
}
