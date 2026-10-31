//! Custom fixtures — the guide's recipes for `dock.custom` and `statusline.custom`, drawn.
//!
//! Each answer is a `--rpc` frame copied from a real run of the recipe's script, decoded the way a
//! client decodes it: the plan bars on a GitHub Copilot plan, and LM Studio with a model loaded. So a
//! picture shows what the script sent, not a guess at it, and drifts with the view code like any
//! other capture.

use super::*;
use luxe_core::config::{CustomDockSection, CustomStatuslineModule, StatuslineModule};

/// What Copilot reported for the plan in the recorded run.
const COPILOT_PLAN: &str = r#"{"kind":"plan_usage","content":{"plan":"individual","primary":{"used_percent":31.6,"window_minutes":43200,"resets_in_secs":330784}}}"#;

/// The plan-bars script's answer for the dock, in that run, told the default rail's 40-cell row and
/// 15-cell label column: the bar takes the 16 cells its tail leaves.
const PLAN_BARS_DOCK: &str = r#"{"kind":"dock_section","content":{"name":"quota","rows":[{"label":"monthly","spans":[{"text":"▰▰▰▰▰","ink":"accent"},{"text":"▱▱▱▱▱▱▱▱▱▱▱","ink":"dim"},{"text":" 32% · 3d","ink":"muted"}]}],"failure":null}}"#;

/// The same script's answer for the statusline, shorter because the input said `place: statusline`.
const PLAN_BARS_BAR: &str = r#"{"kind":"statusline_module","content":{"name":"quota","rows":[{"label":"monthly","spans":[{"text":"▰▰","ink":"accent"},{"text":"▱▱▱","ink":"dim"},{"text":" 32%","ink":"muted"}]}],"failure":null}}"#;

/// The LM Studio script's answer, with the GPU past 90% so its row came back in `warning`.
const LMSTUDIO_DOCK: &str = r#"{"kind":"dock_section","content":{"name":"local","rows":[{"label":"model","spans":[{"text":"Qwen3.8 27B UD","ink":"text"},{"text":" · Q4_K_XL","ink":"muted"}]},{"label":"context","spans":[{"text":"256k loaded","ink":"text"}]},{"label":"vram","spans":[{"text":"29.8 / 31.8 GB","ink":"warning"}]}],"failure":null}}"#;

/// Seeds `app` for a custom-section scenario, reporting whether this family owns it.
pub(super) fn seed(app: &mut App, scenario: FixtureScenario) -> Result<bool, String> {
    match scenario {
        FixtureScenario::DockPlanBars => seed_plan_bars(app, true)?,
        FixtureScenario::StatuslinePlanBars => seed_plan_bars(app, false)?,
        FixtureScenario::DockLmstudio => seed_lmstudio(app)?,
        _ => return Ok(false),
    }
    Ok(true)
}

/// A Copilot session configured as the recipe says, with the dock open or put away.
fn seed_plan_bars(app: &mut App, dock_open: bool) -> Result<(), String> {
    let ask = ("What does the dock show when I'm on a plan?", PLAN_REPLY);
    seed_exchange(app, ("github-copilot", "gpt-6-luna", 1_100_000), ask);
    let sections = ["project", "plan", "files", "elsewhere", "model", "quota", "session"];
    app.config.dock.sections = Some(sections.iter().map(ToString::to_string).collect());
    app.config.dock.custom = Some([("quota".into(), CustomDockSection::default())].into());
    let right = ["context", "quota", "thinking", "model"].map(module);
    app.config.statusline.right = Some(right.to_vec());
    app.config.statusline.custom =
        Some([("quota".into(), CustomStatuslineModule::default())].into());
    app.status.session = session_usage(0.004);
    for recorded in [COPILOT_PLAN, PLAN_BARS_DOCK, PLAN_BARS_BAR] {
        replay(app, recorded)?;
    }
    match dock_open {
        true => app.dock.show(),
        false => app.dock.hide(),
    }
    Ok(())
}

/// An LM Studio session with the recipe's `local` section beside it.
fn seed_lmstudio(app: &mut App) -> Result<(), String> {
    let ask = ("Is the model I picked the one LM Studio has loaded?", LOCAL_REPLY);
    seed_exchange(app, ("lmstudio", "qwen3.8-27b", 262_144), ask);
    app.status.session = session_usage(0.0);
    app.config.dock.custom = Some([("local".into(), CustomDockSection::default())].into());
    replay(app, LMSTUDIO_DOCK)?;
    app.dock.show();
    Ok(())
}

const PLAN_REPLY: &str =
    "How much of the plan is used and when it resets, from whatever your script prints.";
const LOCAL_REPLY: &str =
    "It is: the LOCAL section names what's loaded, and would warn if it weren't.";

/// A short finished exchange, so the sections sit beside a real session. `on` is the provider, the
/// model and its window; `ask` the question and its answer.
fn seed_exchange(app: &mut App, on: (&str, &str, u32), ask: (&str, &str)) {
    let ((provider, model, window), (question, reply)) = (on, ask);
    app.push_convo(ConvoBlock::User {
        text: question.into(),
        attached: Vec::new(),
        chips: Vec::new(),
    });
    app.push_convo(ConvoBlock::Assistant(reply.into()));
    app.status.provider = provider.into();
    app.status.model = model.into();
    app.status.context_window = window;
    app.status.context_pct = 0.6;
    app.status.thinking = luxe_core::provider::ThinkingLevel::High;
    app.workspace = Some(dock_workspace_snapshot());
}

/// A few agent turns' worth, at what such a session really cost: nothing locally, and a fraction
/// of a cent on a plan, where the price is the catalog's estimate.
fn session_usage(cost_usd: f64) -> luxe_core::usage::SessionUsage {
    use luxe_core::usage::{SessionUsage, SourceUsage, UsageSource, UsageTotals};
    let agent = UsageTotals {
        calls: 3,
        prompt_tokens: 11_200,
        completion_tokens: 40,
        cache_read_tokens: 5_700,
        cost_usd,
        ..Default::default()
    };
    let by_source = vec![SourceUsage { source: UsageSource::Agent, totals: agent }];
    SessionUsage { total: agent, by_source }
}

fn module(name: &str) -> StatuslineModule {
    serde_json::from_value(serde_json::Value::String(name.into()))
        .expect("a module name always reads, as a built-in or a custom one")
}

/// Feeds a recorded frame through the reducer, as the engine would have.
fn replay(app: &mut App, frame: &str) -> Result<(), String> {
    let event: Event = serde_json::from_str(frame)
        .map_err(|error| format!("a recorded frame won't read: {error}"))?;
    app.update(Msg::Engine(event));
    Ok(())
}
