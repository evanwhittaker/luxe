//! Custom dock runner — one run of a section's or module's program, bounded in time and output.
//!
//! The program runs directly, with no shell, in the project folder, as its own process tree so a
//! timeout or a dropped feed kills everything it started. Its stdout is read up to the cap and no
//! further, so a program stuck printing can't grow luxe's memory while it waits out the timeout.

use super::output::{parse_output, DockOutputError, MAX_OUTPUT_BYTES};
use super::{CustomProgram, DockRow};
use crate::program::TreeGuard;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Why a run drew nothing new. The message is what the section's failure row says, so each is
/// short enough to fit beside a label.
#[derive(Debug, thiserror::Error)]
pub(crate) enum DockRunError {
    /// The section has an empty `command`.
    #[error("no command set")]
    NoCommand,
    /// The program couldn't be started.
    ///
    /// Args: the program as configured, and why.
    #[error("could not start {program}: {source}")]
    Spawn { program: String, source: std::io::Error },
    /// Talking to the program failed partway.
    ///
    /// Args: the I/O error.
    #[error("could not read it: {0}")]
    Io(#[source] std::io::Error),
    /// It ran past the section's timeout.
    ///
    /// Args: the timeout, in milliseconds.
    #[error("timed out after {0} ms")]
    TimedOut(u128),
    /// It printed more than [`MAX_OUTPUT_BYTES`].
    #[error("printed more than {MAX_OUTPUT_BYTES} bytes")]
    TooLong,
    /// It exited with a failure.
    ///
    /// Args: the exit code, or none when a signal ended it.
    #[error("exited {}", code.map_or_else(|| "on a signal".to_string(), |code| code.to_string()))]
    Exited { code: Option<i32> },
    /// It answered something that isn't a section.
    ///
    /// Args: why it couldn't be read.
    #[error(transparent)]
    Output(#[from] DockOutputError),
}

/// `(COLUMNS, LUXE_LABEL_COLUMNS)` for a run, as [`super::RowWidths::columns`] gives them.
pub(crate) type Columns = (u16, u16);

/// Runs `program` once with `input` on stdin, in `cwd`, and reads its rows. `columns` is its row's
/// width, when the front-end has said.
pub(crate) async fn run_program(
    program: &CustomProgram,
    input: &[u8],
    cwd: &Path,
    columns: Option<Columns>,
) -> Result<Vec<DockRow>, DockRunError> {
    let timeout = program.timeout;
    let launch = Launch { command: &program.command, cwd, columns };
    let ran = tokio::time::timeout(timeout, run_to_exit(launch, input)).await;
    let stdout = ran.map_err(|_| DockRunError::TimedOut(timeout.as_millis()))??;
    Ok(parse_output(&String::from_utf8_lossy(&stdout))?)
}

/// How long the output may stay open after the program exits before luxe ends whatever it left
/// holding it. Long enough for the last bytes in flight, short enough to go unnoticed. A busy Mac
/// can take most of 100ms just to start a job's subshell, so a tighter grace loses its last line.
const EXIT_GRACE: Duration = Duration::from_millis(250);

/// How to start a run: the program, where, and how wide its row is.
#[derive(Clone, Copy)]
struct Launch<'a> {
    command: &'a [String],
    cwd: &'a Path,
    columns: Option<Columns>,
}

/// The program's stdout once it exits successfully. The guard kills the tree on every early
/// return and on a drop, and is disarmed only once the program has exited by itself.
async fn run_to_exit(launch: Launch<'_>, input: &[u8]) -> Result<Vec<u8>, DockRunError> {
    let mut child = spawn(launch)?;
    let mut guard = TreeGuard::arm(&child);
    let stdout = read_until_done(&mut child, &mut guard, input).await?;
    if stdout.len() > MAX_OUTPUT_BYTES {
        return Err(DockRunError::TooLong);
    }
    let status = child.wait().await.map_err(DockRunError::Io)?;
    guard.disarm();
    match status.success() {
        true => Ok(stdout),
        false => Err(DockRunError::Exited { code: status.code() }),
    }
}

/// Feeds the input and reads stdout until it closes. A background job that inherited stdout keeps
/// it open after the program exits, so once the program is gone and [`EXIT_GRACE`] has passed,
/// its tree is ended and the read finishes with everything the program printed.
async fn read_until_done(
    child: &mut tokio::process::Child,
    guard: &mut TreeGuard,
    input: &[u8],
) -> Result<Vec<u8>, DockRunError> {
    let (stdin, stdout) = (child.stdin.take(), child.stdout.take());
    let io = async { tokio::try_join!(write_input(stdin, input), read_capped(stdout)) };
    tokio::pin!(io);
    tokio::select! {
        done = &mut io => return done.map(|(_, stdout)| stdout),
        () = crate::program::exited_unreaped(child) => {}
    }
    if let Ok(done) = tokio::time::timeout(EXIT_GRACE, &mut io).await {
        return done.map(|(_, stdout)| stdout);
    }
    tracing::debug!("ending what a dock program left holding its output");
    guard.kill_now();
    io.await.map(|(_, stdout)| stdout)
}

fn spawn(launch: Launch<'_>) -> Result<tokio::process::Child, DockRunError> {
    let Some((program, args)) = launch.command.split_first() else {
        return Err(DockRunError::NoCommand);
    };
    let mut process = tokio::process::Command::new(expand_home(program));
    set_columns(&mut process, launch.columns);
    process
        .args(args)
        .current_dir(launch.cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        // Not read: a chatty stderr would fill the pipe and stall the program
        .stderr(Stdio::null())
        .kill_on_drop(true);
    crate::program::detach_from_terminal(&mut process);
    process.spawn().map_err(|source| DockRunError::Spawn { program: program.clone(), source })
}

/// Tells the program its row's width the way a terminal tells a program its own. Before the
/// front-end has said, the variables go rather than pass on luxe's: a `COLUMNS` inherited from the
/// shell is the terminal's width, which no row is.
fn set_columns(process: &mut tokio::process::Command, columns: Option<Columns>) {
    match columns {
        Some((row, label)) => {
            process.env("COLUMNS", row.to_string()).env("LUXE_LABEL_COLUMNS", label.to_string());
        }
        None => {
            process.env_remove("COLUMNS").env_remove("LUXE_LABEL_COLUMNS");
        }
    }
}

/// `program` with a leading `~/` made the home folder, since no shell is there to do it.
fn expand_home(program: &str) -> PathBuf {
    let home = crate::user_dirs::home_dir();
    match (program.strip_prefix("~/"), home) {
        (Some(rest), Some(home)) => home.join(rest),
        _ => PathBuf::from(program),
    }
}

/// Writes the payload and closes stdin. A program that never reads it is fine.
async fn write_input(
    stdin: Option<tokio::process::ChildStdin>,
    input: &[u8],
) -> Result<(), DockRunError> {
    let Some(mut stdin) = stdin else { return Ok(()) };
    match stdin.write_all(input).await {
        Err(error) if error.kind() != std::io::ErrorKind::BrokenPipe => {
            Err(DockRunError::Io(error))
        }
        _ => Ok(()),
    }
}

/// Stdout up to one byte past the cap, which is enough to know the cap was passed.
async fn read_capped(stdout: Option<tokio::process::ChildStdout>) -> Result<Vec<u8>, DockRunError> {
    let mut buffer = Vec::new();
    let Some(stdout) = stdout else { return Ok(buffer) };
    let limit = MAX_OUTPUT_BYTES as u64 + 1;
    stdout.take(limit).read_to_end(&mut buffer).await.map_err(DockRunError::Io)?;
    Ok(buffer)
}

#[cfg(all(test, unix))]
mod tests;
