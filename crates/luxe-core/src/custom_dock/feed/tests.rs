use super::*;
use crate::event::StopReason;
use std::time::Duration;

fn sh(script: &str) -> Vec<String> {
    vec!["sh".into(), "-c".into(), script.into()]
}

fn session(scripts: &[(&str, &str)]) -> FeedSession {
    let section = |name: &str, script: &str| {
        let definition =
            crate::config::CustomDockSection { command: sh(script), ..Default::default() };
        CustomProgram::dock_section(name, &definition)
    };
    FeedSession {
        context: DockInputContext { session_id: "s1".into(), cwd: "/".into() },
        programs: scripts.iter().map(|(name, script)| section(name, script)).collect(),
    }
}

fn turn_ended() -> Event {
    Event::TurnEnded { reason: StopReason::EndTurn }
}

async fn next(fed: &mut mpsc::Receiver<Event>) -> Event {
    tokio::time::timeout(Duration::from_secs(5), fed.recv())
        .await
        .expect("an event within five seconds")
        .expect("the feed is still open")
}

#[tokio::test]
async fn a_turn_ending_runs_the_section_and_its_answer_follows() {
    let (engine, events) = mpsc::channel(8);
    let mut fed = feed(events, session(&[("quota", "echo 30%")]));
    engine.send(turn_ended()).await.unwrap();
    assert!(matches!(next(&mut fed).await, Event::TurnEnded { .. }), "the trigger passes first");
    let Event::DockSection(DockAnswer { name, rows, failure }) = next(&mut fed).await else {
        panic!("expected the section's answer");
    };
    assert_eq!((name.as_str(), failure), ("quota", None));
    assert_eq!(rows[0].spans[0].text, "30%");
}

/// Each program hears its own place's width: a dock section the row and the label column taken
/// from it, a statusline module its cells and no column.
#[tokio::test]
async fn each_program_is_told_its_row_width() {
    let (engine, events) = mpsc::channel(8);
    let script = "echo $COLUMNS $LUXE_LABEL_COLUMNS";
    let mut session = session(&[("quota", script)]);
    let module =
        crate::config::CustomStatuslineModule { command: sh(script), ..Default::default() };
    session.programs.push(CustomProgram::statusline_module("bar", &module));
    let mut fed = feed(events, session);
    let widths = crate::custom_dock::RowWidths { dock: 40, dock_label: 15, statusline: 32 };
    engine.send(Event::CustomRowWidths(widths)).await.unwrap();
    engine.send(turn_ended()).await.unwrap();
    let mut heard = std::collections::BTreeMap::new();
    while heard.len() < 2 {
        if let Event::DockSection(answer) | Event::StatuslineModule(answer) = next(&mut fed).await {
            heard.insert(answer.name, answer.rows[0].spans[0].text.clone());
        }
    }
    assert_eq!(heard["quota"], "40 15");
    assert_eq!(heard["bar"], "32 0");
}

/// The stream must never wait on a program: an event sent after a trigger arrives while the
/// program is still running.
#[tokio::test]
async fn events_keep_flowing_while_a_program_runs() {
    let (engine, events) = mpsc::channel(8);
    let mut fed = feed(events, session(&[("slow", "sleep 3; echo late")]));
    engine.send(turn_ended()).await.unwrap();
    engine.send(Event::Notice("next".into())).await.unwrap();
    assert!(matches!(next(&mut fed).await, Event::TurnEnded { .. }));
    let passed = tokio::time::timeout(Duration::from_secs(1), fed.recv()).await;
    assert!(matches!(passed, Ok(Some(Event::Notice(_)))), "got {passed:?}");
}

async fn next_answer(fed: &mut mpsc::Receiver<Event>) -> DockAnswer {
    loop {
        if let Event::DockSection(answer) = next(fed).await {
            return answer;
        }
    }
}

/// Moments that come while a section runs fold into one more run after it, never one beside it.
#[tokio::test]
async fn triggers_during_a_run_fold_into_one_run_after_it() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("runs");
    let script = format!("echo start >> {0}; sleep 0.5; echo end >> {0}; echo done", log.display());
    let (engine, events) = mpsc::channel(8);
    let mut fed = feed(events, session(&[("once", &script)]));
    for _ in 0..3 {
        engine.send(turn_ended()).await.unwrap();
    }
    next_answer(&mut fed).await;
    next_answer(&mut fed).await;
    let extra = tokio::time::timeout(Duration::from_millis(800), next_answer(&mut fed)).await;
    assert!(extra.is_err(), "two runs, not three: {extra:?}");
    let runs = std::fs::read_to_string(&log).unwrap();
    assert_eq!(runs.lines().collect::<Vec<_>>(), ["start", "end", "start", "end"]);
}

/// The turn ends, then the plan reports while that run is going: the rerun is the one with the
/// plan in it, or the section would show stale numbers until the next turn.
#[tokio::test]
async fn a_run_after_a_missed_moment_sees_what_changed() {
    let script = r#"sleep 0.5; grep -q '"plan_usage":{' && echo fresh || echo stale"#;
    let (engine, events) = mpsc::channel(8);
    let mut fed = feed(events, session(&[("quota", script)]));
    engine.send(turn_ended()).await.unwrap();
    engine.send(Event::PlanUsage(crate::provider::PlanUsage::default())).await.unwrap();
    assert_eq!(next_answer(&mut fed).await.rows[0].spans[0].text, "stale");
    assert_eq!(next_answer(&mut fed).await.rows[0].spans[0].text, "fresh");
}

/// A statusline module's answer comes as its own event, so a dock section of the same name can't
/// take its place.
#[tokio::test]
async fn a_statusline_module_answers_as_a_statusline_module() {
    let definition =
        crate::config::CustomStatuslineModule { command: sh("echo 42%"), ..Default::default() };
    let program = CustomProgram::statusline_module("quota", &definition);
    let (engine, events) = mpsc::channel(8);
    let context = DockInputContext { session_id: "s1".into(), cwd: "/".into() };
    let mut fed = feed(events, FeedSession { context, programs: vec![program] });
    engine.send(turn_ended()).await.unwrap();
    next(&mut fed).await;
    let Event::StatuslineModule(DockAnswer { name, rows, .. }) = next(&mut fed).await else {
        panic!("expected the module's answer");
    };
    assert_eq!((name.as_str(), rows[0].spans[0].text.as_str()), ("quota", "42%"));
}

#[tokio::test]
async fn a_failing_program_reports_why() {
    let (engine, events) = mpsc::channel(8);
    let mut fed = feed(events, session(&[("broken", "exit 2")]));
    engine.send(turn_ended()).await.unwrap();
    next(&mut fed).await;
    let Event::DockSection(DockAnswer { rows, failure, .. }) = next(&mut fed).await else {
        panic!("expected the section's answer");
    };
    assert!(rows.is_empty());
    assert_eq!(failure.as_deref(), Some("exited 2"));
}

#[tokio::test]
async fn events_that_change_nothing_run_nothing() {
    let (engine, events) = mpsc::channel(8);
    let mut fed = feed(events, session(&[("quota", "echo 30%")]));
    engine.send(Event::Notice("hello".into())).await.unwrap();
    drop(engine);
    assert!(matches!(next(&mut fed).await, Event::Notice(_)));
    assert!(fed.recv().await.is_none(), "no run, so the stream simply ends");
}
