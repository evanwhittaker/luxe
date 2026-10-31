//! Custom dock — dock sections and statusline modules drawn from a program the user configures.
//!
//! A program named under `dock.custom` or `statusline.custom` gets the session's state as JSON on
//! stdin and answers with rows; a front-end draws them with its theme, and the statusline draws the
//! first. [`feed`] watches the event stream for the moments that state changes, runs each program
//! off the stream, and adds an [`crate::Event::DockSection`] or [`crate::Event::StatuslineModule`]
//! with what it answered.
//!
//! This is the one extension point that draws UI, so it holds to what keeps a program from breaking
//! the screen: rows are sanitized and bounded here, in core, before any front-end sees them.

mod feed;
mod input;
mod output;
mod runner;

pub use feed::{feed, FeedSession};
pub use input::{DockInput, DockInputContext};

use crate::config::{Config, CustomDockSection, CustomStatuslineModule};
use crate::event::Event;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Where a program's answer is drawn, as its input's `place` tells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CustomPlace {
    /// A section of the Context Dock, from `dock.custom`
    Dock,
    /// A module on the statusline, from `statusline.custom`
    Statusline,
}

/// One program a feed runs, from either config key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomProgram {
    /// Which key defined it, and so which event carries its answer
    pub place: CustomPlace,
    /// Its name under that key
    pub name: String,
    /// Program and arguments
    pub command: Vec<String>,
    /// Most a run may take
    pub timeout: Duration,
}

impl CustomProgram {
    /// The program behind the dock section `name`.
    pub fn dock_section(name: &str, section: &CustomDockSection) -> Self {
        let (command, timeout) = (section.command.clone(), section.timeout());
        Self { place: CustomPlace::Dock, name: name.to_string(), command, timeout }
    }

    /// The program behind the statusline module `name`.
    pub fn statusline_module(name: &str, module: &CustomStatuslineModule) -> Self {
        let (command, timeout) = (module.command.clone(), module.timeout());
        Self { place: CustomPlace::Statusline, name: name.to_string(), command, timeout }
    }

    /// The event that reports `answer`, by where it's drawn.
    pub(crate) fn event(&self, answer: DockAnswer) -> Event {
        match self.place {
            CustomPlace::Dock => Event::DockSection(answer),
            CustomPlace::Statusline => Event::StatuslineModule(answer),
        }
    }
}

/// How wide a front-end draws custom rows. It reports them with `Action::CustomRowWidths` when its
/// layout changes, and each run passes its place's on as `COLUMNS` and `LUXE_LABEL_COLUMNS`, so a
/// program can size a bar to a dock someone dragged wider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RowWidths {
    /// Cells in a dock row, its label included
    pub dock: u16,
    /// Cells the dock's label column takes from the start of a labeled row
    pub dock_label: u16,
    /// Cells a statusline module gets, its label included
    pub statusline: u16,
}

impl RowWidths {
    /// `(COLUMNS, LUXE_LABEL_COLUMNS)` for a program drawn at `place`. A statusline module's label
    /// is inline, not a column, so it takes nothing up front.
    pub(crate) fn columns(self, place: CustomPlace) -> (u16, u16) {
        match place {
            CustomPlace::Dock => (self.dock, self.dock_label),
            CustomPlace::Statusline => (self.statusline, 0),
        }
    }
}

/// Every program `config` has something on screen for: the dock's shown custom sections, then the
/// statusline's listed custom modules. One defined but not shown isn't run.
pub fn shown_programs(config: &Config) -> Vec<CustomProgram> {
    let dock = config.dock.shown_custom_sections();
    let statusline = config.statusline.shown_custom_modules();
    let dock = dock.iter().map(|(name, section)| CustomProgram::dock_section(name, section));
    let statusline =
        statusline.iter().map(|(name, module)| CustomProgram::statusline_module(name, module));
    dock.chain(statusline).collect()
}

/// What a program answered, as [`crate::Event::DockSection`] and
/// [`crate::Event::StatuslineModule`] carry it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DockAnswer {
    /// The section or module, as `dock.custom` or `statusline.custom` names it
    pub name: String,
    /// What to draw, already sanitized and capped. Empty on a failure, which keeps the last rows.
    #[serde(default)]
    pub rows: Vec<DockRow>,
    /// Why the run drew nothing new, short enough for a dock row
    #[serde(default)]
    pub failure: Option<String>,
}

/// One row a custom section draws: an optional label in the label column, then its value.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DockRow {
    /// Drawn in the telemetry label column, so the row lines up with MODEL and SESSION
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The value, as runs of text in theme roles
    #[serde(default)]
    pub spans: Vec<DockSpan>,
}

/// A run of text in one theme role.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DockSpan {
    /// The text, already stripped of control characters
    pub text: String,
    /// How it's colored
    #[serde(default)]
    pub ink: DockInk,
}

/// A theme role, never a color, so a theme change recolors a section without running it again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockInk {
    /// Body text (default)
    #[default]
    Text,
    /// Secondary text
    Muted,
    /// Barely there
    Dim,
    /// The theme's accent
    Accent,
    /// Something going well
    Success,
    /// Something to keep an eye on
    Warning,
    /// Something wrong
    Error,
    /// An ink this build doesn't know, drawn as body text.
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod tests;
