//! Custom dock sections — `dock.custom`, sections drawn from a program the user names.
//!
//! The definitions only; [`crate::custom_dock`] runs them and the TUI draws what they answer. A
//! definition runs a program, so a project's copy is dropped unless the project is trusted (see
//! `config::trust`).

use super::{known_or, Lenient};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// How long a run may take when the definition doesn't say.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2);

/// The most any run may take, whatever the definition says: a section is a glance, and a program
/// still running after this long is stuck rather than slow.
const MAX_TIMEOUT: Duration = Duration::from_secs(10);

/// One `dock.custom.<name>` entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CustomDockSection {
    /// Program and arguments, run directly without a shell; a leading `~/` is expanded
    pub command: Vec<String>,
    /// The heading; `None` uses the section's name, uppercased
    pub title: Option<String>,
    /// Which region it joins; `None` is the bottom, beside MODEL and SESSION
    pub place: Option<DockPlace>,
    /// Most a run may take, in milliseconds; `None` is two seconds, and ten is the cap
    pub timeout_ms: Option<u64>,
}

impl CustomDockSection {
    /// The heading for a section named `name`.
    pub fn heading(&self, name: &str) -> String {
        self.title.clone().unwrap_or_else(|| name.to_uppercase())
    }

    /// The region it joins, with an unknown value read as the bottom.
    pub fn place(&self) -> DockPlace {
        known_or(self.place)
    }

    /// How long a run may take: `timeout_ms`, or two seconds, and never past ten.
    pub fn timeout(&self) -> Duration {
        bounded_timeout(self.timeout_ms)
    }
}

/// A program's deadline from its `timeout_ms`, shared with the statusline's custom modules.
pub(super) fn bounded_timeout(timeout_ms: Option<u64>) -> Duration {
    timeout_ms.map_or(DEFAULT_TIMEOUT, Duration::from_millis).min(MAX_TIMEOUT)
}

/// The dock region a custom section joins.
///
/// The regions are fixed, so `dock.sections` orders a section within its region and never moves it
/// between them. Guessing the region from a neighbor in that list would let reordering two
/// unrelated sections move a third.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockPlace {
    /// The scrolling middle, with PLAN, FILES and ELSEWHERE
    Middle,
    /// Pinned beside the composer, with MODEL, USAGE and SESSION
    Bottom,
    /// Unknown values fall back to the bottom.
    #[serde(other)]
    Unknown,
}

impl Lenient for DockPlace {
    const UNKNOWN: Self = Self::Unknown;
    const DEFAULT: Self = Self::Bottom;
    const DEFAULT_NAME: &'static str = "bottom";
    const NAME: &'static str = "custom dock section place";
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pinned against how a config file spells it.
    #[test]
    fn a_definition_reads_from_a_config_file() {
        let section: CustomDockSection = serde_json::from_str(
            r#"{"command": ["~/bin/luxe-quota", "--short"], "title": "QUOTA", "place": "middle", "timeout_ms": 500}"#,
        )
        .expect("a custom section");
        assert_eq!(section.command, ["~/bin/luxe-quota", "--short"]);
        assert_eq!(section.heading("quota"), "QUOTA");
        assert_eq!(section.place(), DockPlace::Middle);
        assert_eq!(section.timeout(), Duration::from_millis(500));
    }

    #[test]
    fn an_untitled_section_is_headed_by_its_name() {
        assert_eq!(CustomDockSection::default().heading("quota"), "QUOTA");
    }

    #[test]
    fn an_unset_or_unknown_place_is_the_bottom() {
        assert_eq!(CustomDockSection::default().place(), DockPlace::Bottom);
        let odd: CustomDockSection = serde_json::from_str(r#"{"place": "sideways"}"#).unwrap();
        assert_eq!(odd.place(), DockPlace::Bottom);
    }

    #[test]
    fn the_timeout_defaults_to_two_seconds_and_stops_at_ten() {
        assert_eq!(CustomDockSection::default().timeout(), Duration::from_secs(2));
        let slow = CustomDockSection { timeout_ms: Some(60_000), ..CustomDockSection::default() };
        assert_eq!(slow.timeout(), Duration::from_secs(10));
    }
}
