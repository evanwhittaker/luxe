//! Custom dock input — what a custom program reads on stdin, kept current from the event stream.
//!
//! Everything here is already on the stream a front-end reads, so a program sees what the dock
//! sees and nothing more. `plan_usage` and `session_usage` are the types their events carry, whose
//! shape `tests/event_serialization.rs` already pins, so the payload adds no second contract.

use super::{CustomPlace, CustomProgram, RowWidths};
use crate::event::Event;
use crate::provider::PlanUsage;
use crate::usage::SessionUsage;
use serde::Serialize;

/// The payload's version. Fields are only ever added, so a program can ignore ones it doesn't know.
const INPUT_VERSION: u32 = 1;

/// What a feed knows before any event arrives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockInputContext {
    /// The session the engine starts with; a resume changes it
    pub session_id: String,
    /// The project folder, which is also where programs run
    pub cwd: String,
}

/// How full the context window is.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct DockContextFill {
    /// Tokens the last request carried
    pub used_tokens: u32,
    /// The model's window, in tokens
    pub window: u32,
    /// `used_tokens` as a share of `window`, 0 to 100
    pub used_percent: f64,
}

/// The JSON a custom program reads on stdin.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DockInput {
    /// The payload's version, 1
    pub version: u32,
    /// The section or module being run, as `dock.custom` or `statusline.custom` names it
    pub section: String,
    /// Where the answer is drawn, so one program can serve both and answer shorter for the bar
    pub place: CustomPlace,
    /// The session's id
    pub session_id: String,
    /// The project folder
    pub cwd: String,
    /// The provider in use, once the engine has said
    pub provider: Option<String>,
    /// The model in use, once the engine has said
    pub model: Option<String>,
    /// The context window's fill after the last request
    pub context: Option<DockContextFill>,
    /// The subscription's allowance, for providers that report one
    pub plan_usage: Option<PlanUsage>,
    /// The session's running spend
    pub session_usage: Option<SessionUsage>,
    /// When `plan_usage` arrived, so a run after a later turn is told how long until each reset
    /// from now, not from the report
    #[serde(skip)]
    pub(crate) plan_usage_at: Option<std::time::Instant>,
    /// How wide the front-end draws rows, once it has said. Passed as `COLUMNS`, not in the JSON,
    /// since that is where a program already looks for its width
    #[serde(skip)]
    pub(crate) row_widths: Option<RowWidths>,
}

impl DockInput {
    /// A payload with nothing reported yet.
    pub fn new(context: &DockInputContext) -> Self {
        Self {
            version: INPUT_VERSION,
            section: String::new(),
            place: CustomPlace::Dock,
            session_id: context.session_id.clone(),
            cwd: context.cwd.clone(),
            provider: None,
            model: None,
            context: None,
            plan_usage: None,
            session_usage: None,
            plan_usage_at: None,
            row_widths: None,
        }
    }

    /// The payload as `program` reads it, with the plan's resets counted to now.
    pub(crate) fn for_program(&self, program: &CustomProgram) -> Self {
        let plan_usage = match (&self.plan_usage, self.plan_usage_at) {
            (Some(plan), Some(at)) => Some(plan.aged(at.elapsed())),
            (plan, _) => plan.clone(),
        };
        Self { section: program.name.clone(), place: program.place, plan_usage, ..self.clone() }
    }

    /// Folds `event` in, and says whether it's a moment to run the programs: a session starting or
    /// resuming, a turn ending, or the provider reporting plan usage. Those are the only points
    /// where anything a program gets changes, which is why there's no timer.
    ///
    /// The engine's first state report counts as the session starting, so a section fills in
    /// before the first turn rather than after it. A load only counts once that has happened: at
    /// startup the load comes first, and a run then would be told no model.
    ///
    /// A change of width is a moment too, once a run has had a model to report: what fit before may
    /// not fit now. The same width again is not, so a redraw that moves nothing runs nothing.
    pub(crate) fn observe(&mut self, event: &Event) -> bool {
        let known = self.model.is_some();
        if let Event::CustomRowWidths(widths) = event {
            return self.row_widths.replace(*widths) != Some(*widths) && known;
        }
        self.record(event);
        match event {
            Event::StateChanged(_) => !known,
            Event::SessionLoaded { .. } => known,
            Event::PlanUsage(_) | Event::TurnEnded { .. } => true,
            _ => false,
        }
    }

    fn record(&mut self, event: &Event) {
        match event {
            Event::StateChanged(state) => {
                self.provider = Some(state.model.provider.clone());
                self.model = Some(state.model.model.clone());
            }
            Event::SessionLoaded { session_id, .. } => self.session_id = session_id.clone(),
            Event::ContextUsage { used_tokens, context_window } => {
                self.context = context_fill(*used_tokens, *context_window);
            }
            Event::SessionUsage(usage) => self.session_usage = Some(usage.clone()),
            Event::PlanUsage(plan) => {
                self.plan_usage = Some(plan.clone());
                self.plan_usage_at = Some(std::time::Instant::now());
            }
            _ => {}
        }
    }
}

/// The fill, or nothing for a model whose window isn't known.
fn context_fill(used_tokens: u32, window: u32) -> Option<DockContextFill> {
    if window == 0 {
        return None;
    }
    let used_percent = f64::from(used_tokens) * 100.0 / f64::from(window);
    Some(DockContextFill { used_tokens, window, used_percent })
}

#[cfg(test)]
mod tests;
