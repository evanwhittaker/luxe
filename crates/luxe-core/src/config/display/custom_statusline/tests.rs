use super::*;
use std::collections::BTreeMap;

fn with_custom(left: &[&str], right: &[&str], defined: &[&str]) -> StatuslineConfig {
    let modules = |names: &[&str]| -> Vec<StatuslineModule> {
        names.iter().map(|name| serde_json::from_value(name.to_string().into()).unwrap()).collect()
    };
    let custom: BTreeMap<String, CustomStatuslineModule> =
        defined.iter().map(|name| (name.to_string(), CustomStatuslineModule::default())).collect();
    StatuslineConfig {
        left: Some(modules(left)),
        right: Some(modules(right)),
        custom: Some(custom),
        ..StatuslineConfig::default()
    }
}

/// Pinned against how a config file spells it.
#[test]
fn a_custom_module_reads_from_a_config_file() {
    let config: StatuslineConfig = serde_json::from_str(
        r#"{"left": ["project", "quota"], "custom": {"quota": {"command": ["~/bin/quota"], "timeout_ms": 500}}}"#,
    )
    .expect("a statusline config");
    assert_eq!(
        config.left,
        Some(vec![StatuslineModule::Project, StatuslineModule::Other("quota".into())])
    );
    let quota = config.custom_module("quota").expect("defined");
    assert_eq!(quota.command, ["~/bin/quota"]);
    assert_eq!(quota.timeout(), Duration::from_millis(500));
}

/// A saved config has to read back the same, so a custom name is written as the bare name.
#[test]
fn a_custom_name_is_written_as_itself() {
    let written = serde_json::to_value(StatuslineModule::Other("quota".into())).unwrap();
    assert_eq!(written, serde_json::json!("quota"));
    assert_eq!(serde_json::to_value(StatuslineModule::Model).unwrap(), serde_json::json!("model"));
}

#[test]
fn the_timeout_defaults_to_two_seconds_and_stops_at_ten() {
    assert_eq!(CustomStatuslineModule::default().timeout(), Duration::from_secs(2));
    let slow = CustomStatuslineModule { timeout_ms: Some(60_000), ..Default::default() };
    assert_eq!(slow.timeout(), Duration::from_secs(10));
}

/// How a custom module replaces a built-in, and why a built-in added later can't break one.
#[test]
fn a_custom_module_wins_over_a_built_in_of_the_same_name() {
    let replaced = with_custom(&[], &["model"], &["model"]);
    assert_eq!(replaced.resolve(&StatuslineModule::Model), StatuslineModule::Other("model".into()));
    let plain = with_custom(&[], &["model"], &[]);
    assert_eq!(plain.resolve(&StatuslineModule::Model), StatuslineModule::Model);
}

/// Running a program nothing draws would cost a spawn every turn for nothing, and one named on
/// both sides needs only one run.
#[test]
fn only_listed_custom_modules_run_and_each_runs_once() {
    let config =
        with_custom(&["git", "quota"], &["quota", "stranger"], &["quota", "git", "unused"]);
    let names: Vec<String> =
        config.shown_custom_modules().into_iter().map(|(name, _)| name).collect();
    assert_eq!(names, ["git", "quota"]);
}

/// With no lists, the defaults hold no custom names, so a definition alone runs nothing.
#[test]
fn a_definition_without_a_listing_runs_nothing() {
    let config = StatuslineConfig {
        custom: Some(BTreeMap::from([("quota".into(), CustomStatuslineModule::default())])),
        ..StatuslineConfig::default()
    };
    assert!(config.shown_custom_modules().is_empty());
}
