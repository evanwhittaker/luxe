//! Dock usage — the USAGE block: how much of the subscription each of its windows has spent.
//!
//! A plan's allowance outlives any one session, so it gets a block of its own between MODEL and
//! SESSION rather than a row in SESSION's receipt. Absent on an API key, and until a provider
//! reports a window.

use super::chrome::{section_heading_line, Section};
use super::telemetry::{telemetry_row, telemetry_value_width};
use crate::app::App;
use luxe_core::provider::PlanWindow;
use ratatui::text::Line;
use unicode_width::UnicodeWidthStr;

/// The USAGE block, one row per window the plan reports, or nothing when it reports none.
pub(super) fn usage_lines(app: &App, row: usize, column: usize) -> Vec<Line<'static>> {
    let windows = app.status.plan_windows(app.frame_now());
    if windows.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![section_heading_line(Section::Usage, app)];
    lines.extend(windows.iter().map(|window| window_row(window, row, column, app)));
    lines
}

/// `weekly   30% · resets 3d 4h`, in the context row's severity ramp.
///
/// A figure, not a bar: the rail draws every level as text, and the color is what turns a reader's
/// head as the window fills.
fn window_row(window: &PlanWindow, row: usize, column: usize, app: &App) -> Line<'static> {
    let percent = window.used_percent.clamp(0.0, 100.0);
    let value = window_value(percent, window.reset_label(), telemetry_value_width(row, column));
    let ink = app.theme_styles.context_fill(percent);
    telemetry_row(&window.window_label(), value, ink, column, app)
}

/// The figure and its reset, in the room the row has.
///
/// Gives up the word `resets` first, then the reset itself: the share spent is the reading, and the
/// countdown is a footnote to it. A row that wrapped would move the block pinned under it.
pub(super) fn window_value(percent: f32, reset: Option<String>, room: usize) -> String {
    let spent = format!("{percent:.0}%");
    let Some(reset) = reset else { return spent };
    [format!("{spent} · resets {reset}"), format!("{spent} · {reset}")]
        .into_iter()
        .find(|value| value.width() <= room)
        .unwrap_or(spent)
}
