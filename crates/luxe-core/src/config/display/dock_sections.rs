//! Dock sections — which sections of the Context Dock show, and in what order.
//!
//! `dock.sections` is the dock's counterpart to the statusline's `left` and `right`: a list of
//! names, where leaving one out hides it. The dock keeps its three regions (PROJECT pinned to the
//! top, a scrolling middle, and the runtime blocks pinned beside the composer), so the order a list
//! gives applies within each region rather than moving a section between them.

use super::{CustomDockSection, DockConfig};

/// A section of the Context Dock, as `dock.sections` names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockSectionName {
    /// The project, its branch and pull request
    Project,
    /// The agent's checklist
    Plan,
    /// The project tree
    Files,
    /// Files changed outside the project
    Elsewhere,
    /// Provider, model, thinking level and mode
    Model,
    /// A subscription's plan windows
    Usage,
    /// Context, tokens and cost for this session
    Session,
    /// A name this build has no built-in section for.
    ///
    /// Args: the name as written. Kept rather than dropped so a section defined elsewhere in the
    /// config can claim it.
    Other(String),
}

impl DockSectionName {
    /// Every built-in section, in the order the dock draws them by default.
    pub fn defaults() -> Vec<Self> {
        vec![
            Self::Project,
            Self::Plan,
            Self::Files,
            Self::Elsewhere,
            Self::Model,
            Self::Usage,
            Self::Session,
        ]
    }

    /// The name a config file uses for this section.
    pub fn name(&self) -> &str {
        match self {
            Self::Project => "project",
            Self::Plan => "plan",
            Self::Files => "files",
            Self::Elsewhere => "elsewhere",
            Self::Model => "model",
            Self::Usage => "usage",
            Self::Session => "session",
            Self::Other(name) => name,
        }
    }

    /// The built-in section a config name means. Names are matched without regard to case or
    /// surrounding space, the way a hand-edited list is likely to drift.
    pub fn parse(name: &str) -> Self {
        let name = name.trim();
        match name.to_ascii_lowercase().as_str() {
            "project" => Self::Project,
            "plan" => Self::Plan,
            "files" => Self::Files,
            "elsewhere" => Self::Elsewhere,
            "model" => Self::Model,
            "usage" => Self::Usage,
            "session" => Self::Session,
            _ => Self::Other(name.to_string()),
        }
    }
}

impl DockConfig {
    /// The sections to draw, in order, without repeats.
    ///
    /// The configured list, or when none is set, every built-in section in its default order with
    /// the custom sections after them, so defining one is enough to see it.
    pub fn sections(&self) -> Vec<DockSectionName> {
        let names = self.sections.clone().unwrap_or_else(|| self.default_names());
        let mut sections: Vec<DockSectionName> = Vec::with_capacity(names.len());
        for section in names.iter().map(|name| self.resolve(name)) {
            if !sections.contains(&section) {
                sections.push(section);
            }
        }
        sections
    }

    /// The custom sections the dock shows, in order, with their definitions: what a feed runs. A
    /// defined section the list leaves out isn't run, since nothing would draw it.
    pub fn shown_custom_sections(&self) -> Vec<(String, CustomDockSection)> {
        let custom = |section: DockSectionName| match section {
            DockSectionName::Other(name) => {
                self.custom_section(&name).map(|definition| (name.clone(), definition.clone()))
            }
            _ => None,
        };
        self.sections().into_iter().filter_map(custom).collect()
    }

    /// The custom section defined under `name`, if there is one.
    pub fn custom_section(&self, name: &str) -> Option<&CustomDockSection> {
        self.custom.as_ref()?.get(name.trim())
    }

    /// What a name in the list means. A custom definition wins over a built-in of the same name:
    /// that's how a custom section replaces one, and why a built-in added later can't break a
    /// custom section someone already named the same.
    fn resolve(&self, name: &str) -> DockSectionName {
        match self.custom_section(name) {
            Some(_) => DockSectionName::Other(name.trim().to_string()),
            None => DockSectionName::parse(name),
        }
    }

    /// Every built-in name in order, then each custom name no built-in already claimed.
    fn default_names(&self) -> Vec<String> {
        let builtins = DockSectionName::defaults();
        let mut names: Vec<String> = builtins.iter().map(|s| s.name().to_string()).collect();
        let custom = self.custom.iter().flat_map(|custom| custom.keys());
        names.extend(custom.filter(|name| !names.contains(name)).cloned().collect::<Vec<_>>());
        names
    }

    /// Whether `section` is drawn.
    pub fn shows(&self, section: &DockSectionName) -> bool {
        self.sections().contains(section)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dock(names: &[&str]) -> DockConfig {
        DockConfig {
            sections: Some(names.iter().map(ToString::to_string).collect()),
            ..DockConfig::default()
        }
    }

    #[test]
    fn unset_shows_every_section_in_the_default_order() {
        assert_eq!(DockConfig::default().sections(), DockSectionName::defaults());
        assert!(DockConfig::default().shows(&DockSectionName::Usage));
    }

    #[test]
    fn a_section_left_out_is_hidden() {
        let config = dock(&["project", "files", "model", "session"]);
        assert!(!config.shows(&DockSectionName::Usage));
        assert!(config.shows(&DockSectionName::Files));
    }

    #[test]
    fn the_list_order_is_kept() {
        let config = dock(&["session", "model"]);
        assert_eq!(config.sections(), [DockSectionName::Session, DockSectionName::Model]);
    }

    #[test]
    fn a_repeat_counts_once_at_its_first_place() {
        let config = dock(&["usage", "model", "usage"]);
        assert_eq!(config.sections(), [DockSectionName::Usage, DockSectionName::Model]);
    }

    #[test]
    fn names_match_regardless_of_case_and_space() {
        assert_eq!(DockSectionName::parse(" Files "), DockSectionName::Files);
    }

    /// An unknown name is kept as written, so a section defined elsewhere in the config can claim it.
    #[test]
    fn an_unknown_name_is_kept_as_written() {
        assert_eq!(DockSectionName::parse("Quota"), DockSectionName::Other("Quota".into()));
    }

    /// An empty list is a choice, not a missing one: it hides every section.
    #[test]
    fn an_empty_list_hides_everything() {
        assert!(dock(&[]).sections().is_empty());
    }

    fn with_custom(config: DockConfig, names: &[&str]) -> DockConfig {
        let custom =
            names.iter().map(|name| (name.to_string(), CustomDockSection::default())).collect();
        DockConfig { custom: Some(custom), ..config }
    }

    /// Defining a section is enough to see it: with no list, it follows the built-ins.
    #[test]
    fn unset_shows_custom_sections_after_the_built_ins() {
        let config = with_custom(DockConfig::default(), &["quota"]);
        let mut expected = DockSectionName::defaults();
        expected.push(DockSectionName::Other("quota".into()));
        assert_eq!(config.sections(), expected);
    }

    /// How a custom section replaces a built-in, and why a built-in added later can't break one.
    #[test]
    fn a_custom_section_wins_over_a_built_in_of_the_same_name() {
        let config = with_custom(dock(&["model", "usage"]), &["usage"]);
        assert_eq!(
            config.sections(),
            [DockSectionName::Model, DockSectionName::Other("usage".into())]
        );
        let unset = with_custom(DockConfig::default(), &["usage"]);
        assert!(unset.sections().contains(&DockSectionName::Other("usage".into())));
        assert!(!unset.sections().contains(&DockSectionName::Usage), "replaced, not doubled");
    }

    /// Running a program nothing draws would cost a spawn every turn for nothing.
    #[test]
    fn only_shown_custom_sections_are_run() {
        let config = with_custom(dock(&["model", "quota", "stranger"]), &["quota", "hidden"]);
        let names: Vec<String> =
            config.shown_custom_sections().into_iter().map(|(name, _)| name).collect();
        assert_eq!(names, ["quota"]);
    }

    #[test]
    fn a_listed_custom_name_finds_its_definition() {
        let config = with_custom(dock(&[" quota "]), &["quota"]);
        assert_eq!(config.sections(), [DockSectionName::Other("quota".into())]);
        assert!(config.custom_section("quota").is_some());
        assert!(config.custom_section("other").is_none());
    }

    /// Pinned against how a config file spells it, since that is what people have written.
    #[test]
    fn the_key_reads_from_a_config_file() {
        let config: DockConfig =
            serde_json::from_str(r#"{"mode": "auto", "sections": ["files", "usage"]}"#)
                .expect("a dock config");
        assert_eq!(config.sections(), [DockSectionName::Files, DockSectionName::Usage]);
    }
}
