//! Custom statusline modules — `statusline.custom`, modules drawn from a program the user names.
//!
//! The statusline's counterpart to `dock.custom`, run by the same feed with the same input. A
//! module is named in `left` or `right` like a built-in; nothing shows unless it is, since the bar
//! has two sides and little room, so there's no right place to guess. A project's definitions are
//! dropped unless it's trusted (see `config::trust`).

use super::custom_dock::bounded_timeout;
use super::{StatuslineConfig, StatuslineModule};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// One `statusline.custom.<name>` entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CustomStatuslineModule {
    /// Program and arguments, run directly without a shell; a leading `~/` is expanded
    pub command: Vec<String>,
    /// Most a run may take, in milliseconds; `None` is two seconds, and ten is the cap
    pub timeout_ms: Option<u64>,
}

impl CustomStatuslineModule {
    /// How long a run may take: `timeout_ms`, or two seconds, and never past ten.
    pub fn timeout(&self) -> Duration {
        bounded_timeout(self.timeout_ms)
    }
}

impl StatuslineModule {
    /// The name a config file uses for this module.
    pub fn name(&self) -> &str {
        match self {
            Self::Project => "project",
            Self::Branch => "branch",
            Self::Pr => "pr",
            Self::Todo => "todo",
            Self::Changes => "changes",
            Self::Cwd => "cwd",
            Self::Activity => "activity",
            Self::Cost => "cost",
            Self::Tokens => "tokens",
            Self::Context => "context",
            Self::Thinking => "thinking",
            Self::Model => "model",
            Self::Other(name) => name,
        }
    }
}

impl StatuslineConfig {
    /// The custom module defined under `name`, if there is one.
    pub fn custom_module(&self, name: &str) -> Option<&CustomStatuslineModule> {
        self.custom.as_ref()?.get(name.trim())
    }

    /// What a module in `left` or `right` means. A custom definition wins over a built-in of the
    /// same name: that's how a custom module replaces one, and why a built-in added later can't
    /// break a custom module someone already named the same.
    pub fn resolve(&self, module: &StatuslineModule) -> StatuslineModule {
        match self.custom_module(module.name()) {
            Some(_) => StatuslineModule::Other(module.name().trim().to_string()),
            None => module.clone(),
        }
    }

    /// The custom modules the bar shows, left side first, with their definitions: what a feed
    /// runs. One named on both sides runs once, and one defined but not listed doesn't run.
    pub fn shown_custom_modules(&self) -> Vec<(String, CustomStatuslineModule)> {
        let listed = self.left.iter().chain(self.right.iter()).flatten();
        let mut shown: Vec<(String, CustomStatuslineModule)> = Vec::new();
        for module in listed {
            let name = module.name().trim();
            let Some(definition) = self.custom_module(name) else { continue };
            if !shown.iter().any(|(known, _)| known == name) {
                shown.push((name.to_string(), definition.clone()));
            }
        }
        shown
    }
}

#[cfg(test)]
mod tests;
