//! Dock runtime blocks — MODEL, USAGE and SESSION, in the order `dock.sections` gives them.
//!
//! The blocks themselves are drawn in [`super::telemetry`] and [`super::usage`]; this decides
//! which show, in what order, and where MODEL's clickable rows landed.

use super::chrome::{section_heading_line, Section};
use super::row_width;
use super::telemetry::{
    approvals_row, model_telemetry, session_telemetry, telemetry_label_width,
    WIDEST_TELEMETRY_LABEL,
};
use crate::app::App;
use luxe_core::config::{DockPlace, DockSectionName};
use ratatui::text::Line;

/// The runtime blocks, a blank between each, and the line MODEL's heading landed on.
pub(super) struct RuntimeBlocks {
    pub(super) lines: Vec<Line<'static>>,
    /// `None` when MODEL is hidden, which leaves its rows nothing to click
    pub(super) model_start: Option<usize>,
}

impl RuntimeBlocks {
    /// Adds `block` after a blank, or nothing for an empty one.
    fn push(&mut self, block: Vec<Line<'static>>, is_model: bool) {
        if block.is_empty() {
            return;
        }
        if !self.lines.is_empty() {
            self.lines.push(Line::default());
        }
        if is_model {
            self.model_start = Some(self.lines.len());
        }
        self.lines.extend(block);
    }
}

/// MODEL, USAGE and SESSION in the order `dock.sections` lists them, leaving out what it leaves out.
///
/// Separate sections, because they answer separate questions. MODEL is *identity and setting* —
/// what is answering, and how hard it is thinking. USAGE is the subscription's allowance, which
/// outlives the session and is absent on an API key. SESSION is *consumption* — how full the
/// window is, how many tokens went through it, what that cost. The window leads SESSION because it
/// is the only row with a deadline attached: the rest is a receipt, and this is the one about to
/// make something happen. What keeps it from reading as more accounting is its color, which is the
/// datum.
///
/// Every metric row is the same shape: a label naming it, then a value that leads with its own
/// mark. The MODEL block is always here unless the config hides it — a session with nothing chosen
/// is a state worth reporting, and its row is the pointer's way into the picker; SESSION waits for
/// a turn to measure, so an idle, pre-turn dock stays short.
///
/// Hiding MODEL never hides auto-approval: a session approving its own escalations must not be
/// able to drop out of sight through a config list, so that row keeps a MODEL heading of its own.
pub(super) fn runtime_blocks(app: &App) -> RuntimeBlocks {
    let row = row_width(app);
    let column = telemetry_label_width(row, WIDEST_TELEMETRY_LABEL);
    let sections = app.config.dock.sections();
    let mut blocks = RuntimeBlocks { lines: Vec::new(), model_start: None };
    if !sections.contains(&DockSectionName::Model) {
        blocks.push(approvals_block(app, column), false);
    }
    for section in &sections {
        let block = runtime_block(app, section, row, column);
        blocks.push(block, *section == DockSectionName::Model);
    }
    blocks
}

/// Only the custom bottom sections, for a dock beside a statusline. The bar carries MODEL, USAGE and
/// SESSION, so the dock leaves those to it, but no bar carries a custom section unless its owner
/// also made it a statusline module, so hiding it with the rest would lose it.
pub(super) fn custom_blocks(app: &App) -> RuntimeBlocks {
    let row = row_width(app);
    let column = telemetry_label_width(row, WIDEST_TELEMETRY_LABEL);
    let mut blocks = RuntimeBlocks { lines: Vec::new(), model_start: None };
    for section in &app.config.dock.sections() {
        if let DockSectionName::Other(name) = section {
            blocks.push(
                super::custom::placed_lines(app, name, DockPlace::Bottom, row, column),
                false,
            );
        }
    }
    blocks
}

/// The block `section` draws here, or nothing for a section that lives in another region.
fn runtime_block(
    app: &App,
    section: &DockSectionName,
    row: usize,
    column: usize,
) -> Vec<Line<'static>> {
    match section {
        DockSectionName::Model => model_telemetry(app, row, column),
        DockSectionName::Usage => super::usage::usage_lines(app, row, column),
        DockSectionName::Session => session_telemetry(app, row, column),
        DockSectionName::Other(name) => {
            super::custom::placed_lines(app, name, DockPlace::Bottom, row, column)
        }
        _ => Vec::new(),
    }
}

/// MODEL's heading over the auto-approval row alone, for a dock that hides the rest of MODEL.
fn approvals_block(app: &App, column: usize) -> Vec<Line<'static>> {
    match approvals_row(app, column) {
        Some(row) => vec![section_heading_line(Section::Model, app), row],
        None => Vec::new(),
    }
}
