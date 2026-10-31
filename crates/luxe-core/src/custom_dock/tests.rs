use super::*;
use crate::Event;

/// Pinned against the literal a client reads, since `--rpc` clients built elsewhere parse it and a
/// round trip can't catch a rename.
#[test]
fn the_dock_section_frame_reads_as_published() {
    let answer = DockAnswer {
        name: "quota".into(),
        rows: vec![DockRow {
            label: Some("weekly".into()),
            spans: vec![
                DockSpan { text: "▰▰▱".into(), ink: DockInk::Accent },
                DockSpan { text: " 30%".into(), ink: DockInk::Text },
            ],
        }],
        failure: None,
    };
    let frame = serde_json::to_string(&Event::DockSection(answer.clone())).unwrap();
    let literal = r#"{"kind":"dock_section","content":{"name":"quota","rows":[{"label":"weekly","spans":[{"text":"▰▰▱","ink":"accent"},{"text":" 30%","ink":"text"}]}],"failure":null}}"#;
    assert_eq!(frame, literal);
    let read: Event = serde_json::from_str(literal).unwrap();
    assert!(matches!(read, Event::DockSection(back) if back == answer));
}

#[test]
fn a_failure_frame_reads_as_published() {
    let failed =
        DockAnswer { name: "quota".into(), rows: vec![], failure: Some("exited 1".into()) };
    let literal =
        r#"{"kind":"dock_section","content":{"name":"quota","rows":[],"failure":"exited 1"}}"#;
    assert_eq!(serde_json::to_string(&Event::DockSection(failed)).unwrap(), literal);
}

/// The statusline's frame carries the same answer under its own kind.
#[test]
fn the_statusline_module_frame_reads_as_published() {
    let row = DockRow {
        label: None,
        spans: vec![DockSpan { text: "42%".into(), ink: DockInk::Warning }],
    };
    let answer = DockAnswer { name: "quota".into(), rows: vec![row], failure: None };
    let literal = r#"{"kind":"statusline_module","content":{"name":"quota","rows":[{"spans":[{"text":"42%","ink":"warning"}]}],"failure":null}}"#;
    assert_eq!(serde_json::to_string(&Event::StatuslineModule(answer.clone())).unwrap(), literal);
    let read: Event = serde_json::from_str(literal).unwrap();
    assert!(matches!(read, Event::StatuslineModule(back) if back == answer));
}

/// Only what's shown runs, dock sections first, each answered by the event for its place.
#[test]
fn shown_programs_lists_both_places_in_order() {
    let config: Config = serde_json::from_str(
        r#"{"dock": {"sections": ["model", "bars"], "custom": {"bars": {"command": ["a"]}, "idle": {"command": ["b"]}}},
            "statusline": {"right": ["quota", "model"], "custom": {"quota": {"command": ["c"]}}}}"#,
    )
    .unwrap();
    let programs = shown_programs(&config);
    let listed: Vec<(&str, CustomPlace)> =
        programs.iter().map(|program| (program.name.as_str(), program.place)).collect();
    assert_eq!(listed, [("bars", CustomPlace::Dock), ("quota", CustomPlace::Statusline)]);
    let answer = || DockAnswer { name: "x".into(), rows: vec![], failure: None };
    assert!(matches!(programs[0].event(answer()), Event::DockSection(_)));
    assert!(matches!(programs[1].event(answer()), Event::StatuslineModule(_)));
}

/// A hand-written client may leave out what it has nothing to say about.
#[test]
fn a_frame_with_only_a_name_still_reads() {
    let read: Event =
        serde_json::from_str(r#"{"kind":"dock_section","content":{"name":"quota"}}"#).unwrap();
    assert!(
        matches!(read, Event::DockSection(answer) if answer.rows.is_empty() && answer.failure.is_none())
    );
}
