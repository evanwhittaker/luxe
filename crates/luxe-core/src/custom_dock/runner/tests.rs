use super::*;
use std::time::{Duration, Instant};

fn program(command: &[&str], timeout_ms: u64) -> CustomProgram {
    CustomProgram {
        place: super::super::CustomPlace::Dock,
        name: "test".into(),
        command: command.iter().map(ToString::to_string).collect(),
        timeout: Duration::from_millis(timeout_ms),
    }
}

fn section(script: &str, timeout_ms: u64) -> CustomProgram {
    program(&["sh", "-c", script], timeout_ms)
}

async fn run(script: &str, input: &str) -> Result<Vec<DockRow>, DockRunError> {
    run_program(&section(script, 5_000), input.as_bytes(), Path::new("/"), None).await
}

fn texts(rows: &[DockRow]) -> Vec<String> {
    rows.iter().flat_map(|row| row.spans.iter().map(|span| span.text.clone())).collect()
}

#[tokio::test]
async fn a_program_that_prints_draws_its_lines() {
    let rows = run("echo '30% of the week'", "").await.unwrap();
    assert_eq!(texts(&rows), ["30% of the week"]);
}

/// `cat` answers with what it was given, so a row coming back proves the payload arrived.
#[tokio::test]
async fn the_payload_arrives_on_stdin() {
    let rows = run("cat", r#"{"rows": [{"label": "echoed"}]}"#).await.unwrap();
    assert_eq!(rows[0].label.as_deref(), Some("echoed"));
}

#[tokio::test]
async fn it_runs_in_the_folder_it_is_given() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().canonicalize().unwrap();
    let rows = run_program(&section("pwd", 5_000), b"", &folder, None).await.unwrap();
    assert_eq!(texts(&rows), [folder.to_string_lossy().into_owned()]);
}

#[tokio::test]
async fn a_failing_exit_is_named_with_its_code() {
    let failure = run("echo partial; exit 3", "").await.unwrap_err();
    assert!(matches!(failure, DockRunError::Exited { code: Some(3) }), "{failure:?}");
    assert_eq!(failure.to_string(), "exited 3");
}

#[tokio::test]
async fn output_past_the_cap_is_a_failure() {
    let failure = run("head -c 20000 /dev/zero | tr '\\0' x", "").await.unwrap_err();
    assert!(matches!(failure, DockRunError::TooLong), "{failure:?}");
}

/// A program that keeps printing must not hold luxe until the timeout, or grow its memory.
#[tokio::test]
async fn an_endless_printer_is_stopped_at_the_cap() {
    let started = Instant::now();
    let failure = run_program(&section("yes", 9_000), b"", Path::new("/"), None).await.unwrap_err();
    assert!(matches!(failure, DockRunError::TooLong), "{failure:?}");
    assert!(started.elapsed() < Duration::from_secs(5), "it didn't wait out the timeout");
}

#[tokio::test]
async fn a_slow_program_times_out_and_its_children_die_with_it() {
    let dir = tempfile::tempdir().unwrap();
    let pid_file = dir.path().join("pid");
    let script = format!("sleep 30 & echo $! > {}; wait", pid_file.display());
    let started = Instant::now();
    let failure = run_program(&section(&script, 300), b"", Path::new("/"), None).await.unwrap_err();
    assert!(matches!(failure, DockRunError::TimedOut(300)), "{failure:?}");
    assert!(started.elapsed() < Duration::from_secs(5));
    let pid: u32 = std::fs::read_to_string(&pid_file).unwrap().trim().parse().unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(!crate::program::process_is_alive(pid), "the background sleep was left running");
}

/// A `COLUMNS` luxe inherited from the shell is the terminal's width, which no row is, so before
/// the front-end has said the program gets none rather than that one.
#[test]
fn before_a_width_is_known_an_inherited_columns_is_withheld() {
    let mut process = tokio::process::Command::new("true");
    set_columns(&mut process, None);
    let removed: Vec<_> =
        process.as_std().get_envs().filter(|(_, value)| value.is_none()).collect();
    let names: Vec<_> = removed.iter().map(|(name, _)| name.to_string_lossy()).collect();
    assert_eq!(names, ["COLUMNS", "LUXE_LABEL_COLUMNS"]);
}

fn read_pid(pid_file: &Path) -> u32 {
    std::fs::read_to_string(pid_file).unwrap().trim().parse().unwrap()
}

/// A background job that inherits stdout keeps the output open after the script exits. Waiting for
/// it to close would turn a script that answered into a timeout.
#[tokio::test]
async fn a_child_left_holding_the_output_is_ended_and_the_rows_kept() {
    let dir = tempfile::tempdir().unwrap();
    let pid_file = dir.path().join("pid");
    let script = format!("sleep 30 & echo $! > {}; echo answered", pid_file.display());
    let started = Instant::now();
    let rows = run_program(&section(&script, 5_000), b"", Path::new("/"), None).await.unwrap();
    assert_eq!(texts(&rows), ["answered"]);
    assert!(started.elapsed() < Duration::from_secs(2), "it waited for the leftover");
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        !crate::program::process_is_alive(read_pid(&pid_file)),
        "the leftover was left running"
    );
}

/// Only what holds the output is in the way. A job that let go of it is the script's to leave
/// running, as it would be from a terminal.
#[tokio::test]
async fn a_background_job_that_redirects_its_output_keeps_running() {
    let dir = tempfile::tempdir().unwrap();
    let pid_file = dir.path().join("pid");
    let script = format!("sleep 30 >/dev/null & echo $! > {}; echo answered", pid_file.display());
    let rows = run_program(&section(&script, 5_000), b"", Path::new("/"), None).await.unwrap();
    assert_eq!(texts(&rows), ["answered"]);
    tokio::time::sleep(Duration::from_millis(200)).await;
    let pid = read_pid(&pid_file);
    let alive = crate::program::process_is_alive(pid);
    crate::program::kill_tree(pid);
    assert!(alive, "a job that let go of the output was killed");
}

/// A job finishing a beat after the script still has its say: the grace waits for it rather than
/// killing it the moment the script is gone.
#[tokio::test]
async fn a_job_that_finishes_just_after_the_script_is_heard() {
    let rows = run("(sleep 0.05; echo late) & echo early", "").await.unwrap();
    assert_eq!(texts(&rows), ["early", "late"]);
}

#[tokio::test]
async fn a_missing_program_or_command_says_so() {
    let missing = program(&["luxe-no-such-program"], 5_000);
    let failure = run_program(&missing, b"", Path::new("/"), None).await.unwrap_err();
    assert!(failure.to_string().starts_with("could not start luxe-no-such-program"), "{failure}");
    let empty = run_program(&program(&[], 5_000), b"", Path::new("/"), None).await;
    assert!(matches!(empty, Err(DockRunError::NoCommand)));
}

#[tokio::test]
async fn bad_json_is_a_failure_not_rows() {
    let failure = run("echo '{\"rows\": 3}'", "").await.unwrap_err();
    assert!(failure.to_string().starts_with("not valid output"), "{failure}");
}

#[test]
fn a_leading_tilde_is_the_home_folder() {
    let Some(home) = crate::user_dirs::home_dir() else { return };
    assert_eq!(expand_home("~/bin/quota"), home.join("bin/quota"));
    assert_eq!(expand_home("quota"), PathBuf::from("quota"));
    assert_eq!(expand_home("/x/~/y"), PathBuf::from("/x/~/y"));
}
