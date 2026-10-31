//! Cargo's install record — whether `cargo install` built this copy from source.
//!
//! `cargo install` writes `.crates2.json` in its install root, one entry per package, keyed
//! `name version (source)`. A copy built with `--path` or `--git` is somebody's own build, often a
//! fork, and `cargo install luxe-cli` would overwrite it with the published crate. Both kinds live
//! in `~/.cargo/bin`, so the path can't tell them apart; this record is how
//! [`super::InstallSource`] does.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// The parts of `.crates2.json` this reads: which binaries each installed package put down.
#[derive(Deserialize)]
struct CrateListing {
    #[serde(default)]
    installs: BTreeMap<String, InstalledPackage>,
}

/// One package in the listing.
#[derive(Deserialize)]
struct InstalledPackage {
    #[serde(default)]
    bins: Vec<String>,
}

/// Whether cargo recorded `binary` as built from a local checkout or a git repository.
///
/// The record sits in the install root, the folder above `bin`. No record, one that doesn't parse,
/// or one that doesn't list the binary all read as `false`, leaving the path to decide.
pub(crate) fn built_from_source(binary: &Path) -> bool {
    let Some(root) = binary.parent().and_then(Path::parent) else { return false };
    let Some(name) = binary.file_name().and_then(|name| name.to_str()) else { return false };
    match std::fs::read_to_string(root.join(".crates2.json")) {
        Ok(text) => records_source_build(&text, name),
        Err(_) => false,
    }
}

/// Whether the package in a `.crates2.json` body whose `bins` include `binary_name` came from a
/// path or a git repository.
pub(crate) fn records_source_build(text: &str, binary_name: &str) -> bool {
    let Ok(listing) = serde_json::from_str::<CrateListing>(text) else { return false };
    // Windows may record `luxe.exe` or `luxe`, so both spellings count
    let stem = binary_name.strip_suffix(std::env::consts::EXE_SUFFIX).unwrap_or(binary_name);
    listing
        .installs
        .iter()
        .find(|(_, package)| package.bins.iter().any(|bin| bin == binary_name || bin == stem))
        .is_some_and(|(key, _)| key_names_source_build(key))
}

/// Whether a key like `luxe-cli 0.1.6 (path+file:///home/dev/luxe)` names a path or git source.
fn key_names_source_build(key: &str) -> bool {
    let Some((_, source)) = key.split_once(" (") else { return false };
    source.starts_with("path+") || source.starts_with("git+")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real entries from a `~/.cargo/.crates2.json`, one per kind of source.
    const LISTING: &str = r#"{"installs": {
        "agg 1.9.0 (git+https://github.com/asciinema/agg#aae34b012382536c8c01c753e4b609424642a06d)": {"version_req": null, "bins": ["agg"], "features": [], "all_features": false, "no_default_features": false, "profile": "release", "target": "x86_64-unknown-linux-gnu", "rustc": "rustc 1.97.1"},
        "asciinema 3.2.0 (registry+https://github.com/rust-lang/crates.io-index)": {"version_req": null, "bins": ["asciinema"], "features": [], "all_features": false, "no_default_features": false, "profile": "release", "target": "x86_64-unknown-linux-gnu", "rustc": "rustc 1.97.1"},
        "glyphsmith 0.1.0 (path+file:///home/dev/src/glyphsmith)": {"version_req": null, "bins": ["glyphsmith"], "features": [], "all_features": false, "no_default_features": false, "profile": "release", "target": "x86_64-unknown-linux-gnu", "rustc": "rustc 1.97.1"}
    }}"#;

    #[test]
    fn a_path_install_is_a_source_build() {
        assert!(records_source_build(LISTING, "glyphsmith"));
    }

    #[test]
    fn a_git_install_is_a_source_build() {
        assert!(records_source_build(LISTING, "agg"));
    }

    #[test]
    fn a_registry_install_is_not() {
        assert!(!records_source_build(LISTING, "asciinema"));
    }

    #[test]
    fn a_binary_the_record_does_not_list_is_not() {
        assert!(!records_source_build(LISTING, "luxe"));
    }

    #[test]
    fn a_record_that_does_not_parse_is_not() {
        assert!(!records_source_build("{not json", "glyphsmith"));
    }

    /// The match is on the binary, not the package name, so a fork that renamed its package still
    /// counts as the copy it installed.
    #[test]
    fn the_binary_decides_not_the_package_name() {
        let renamed =
            r#"{"installs": {"luxe-fork 0.1.7 (path+file:///home/dev/luxe)": {"bins": ["luxe"]}}}"#;
        assert!(records_source_build(renamed, "luxe"));
    }
}
