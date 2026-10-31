//! Config trust — project commands require the user's explicit consent.

use std::path::Path;

use super::{load_project_config, Config};

impl Config {
    /// Load standard config layers, ignoring project-supplied gate commands.
    pub fn load(repo_root: &Path) -> Self {
        Self::load_trusted(repo_root, false)
    }
}

pub(super) fn project_config(repo_root: &Path, trust_project: bool) -> Option<Config> {
    let mut project = load_project_config(repo_root)?;
    if !trust_project && !project.gate.commands.is_empty() {
        tracing::warn!(
            path = %repo_root.join(".luxe/config.json").display(),
            "ignoring project gate.commands; use --trust-project to allow them"
        );
        project.gate.commands.clear();
    }
    if !trust_project {
        drop_custom_programs(&mut project, repo_root);
    }
    Some(project)
}

/// Drops an untrusted project's custom dock sections and statusline modules. Each runs its command
/// on every turn, so a cloned repo's would be code run just by opening it.
fn drop_custom_programs(project: &mut Config, repo_root: &Path) {
    let path = repo_root.join(".luxe/config.json");
    if project.dock.custom.take().is_some() {
        tracing::warn!(path = %path.display(), "ignoring project dock.custom; use --trust-project to allow it");
    }
    if project.statusline.custom.take().is_some() {
        tracing::warn!(path = %path.display(), "ignoring project statusline.custom; use --trust-project to allow it");
    }
}
