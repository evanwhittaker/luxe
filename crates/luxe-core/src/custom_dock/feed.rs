//! Custom dock feed — runs each custom program beside the event stream, never in its way.
//!
//! The feed sits between the engine and a front-end. Every event passes through unchanged and in
//! order; the ones that mark a moment to run (see [`DockInput::observe`]) start each program as a
//! task, and its answer joins the stream as an [`Event::DockSection`] or
//! [`Event::StatuslineModule`]. Nothing waits on a program, so a slow one delays its own section
//! and nothing else.
//!
//! It reads the stream rather than hooking the loop because a hook sees none of what a program is
//! given (the model, the context fill, the usage figures), and every one of them is already an event.

use super::input::{DockInput, DockInputContext};
use super::runner::run_program;
use super::{CustomProgram, DockAnswer};
use crate::event::Event;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::task::{Id, JoinError, JoinSet};

/// What a feed runs, and for which session.
#[derive(Debug, Clone)]
pub struct FeedSession {
    /// What the payload says before any event arrives
    pub context: DockInputContext,
    /// Each program with something on screen, as [`super::shown_programs`] lists them
    pub programs: Vec<CustomProgram>,
}

/// `events` with each program's answers added. With no programs it's `events` itself, so a screen
/// without custom sections or modules costs nothing.
pub fn feed(events: mpsc::Receiver<Event>, session: FeedSession) -> mpsc::Receiver<Event> {
    if session.programs.is_empty() {
        return events;
    }
    let (out, fed) = mpsc::channel(256);
    let input = DockInput::new(&session.context);
    let feed = Feed {
        session,
        input,
        out,
        runs: JoinSet::new(),
        running: HashMap::new(),
        missed: HashSet::new(),
    };
    tokio::spawn(feed.run(events));
    fed
}

/// A running feed. Dropping it drops its runs, and with them every program's process tree.
struct Feed {
    session: FeedSession,
    input: DockInput,
    out: mpsc::Sender<Event>,
    runs: JoinSet<()>,
    /// The section (by index) each run in flight belongs to, so none runs twice at once
    running: HashMap<Id, usize>,
    /// Sections whose moment came while they were running, so they run once more when done
    missed: HashSet<usize>,
}

impl Feed {
    async fn run(mut self, mut events: mpsc::Receiver<Event>) {
        loop {
            tokio::select! {
                event = events.recv() => {
                    let Some(event) = event else { return };
                    if !self.pass(event).await {
                        return;
                    }
                }
                Some(done) = self.runs.join_next_with_id(), if !self.runs.is_empty() => {
                    self.finish(done);
                }
            }
        }
    }

    /// Forwards `event`, then starts the runs it calls for. False once nobody is listening.
    async fn pass(&mut self, event: Event) -> bool {
        let due = self.input.observe(&event);
        if self.out.send(event).await.is_err() {
            return false;
        }
        if due {
            self.start_runs();
        }
        true
    }

    /// Starts every program that isn't already running. One still running is marked to run again
    /// when it finishes, never twice at once: what it was given may be out of date (the turn ends,
    /// then the plan reports), and there may be no later moment until the next turn.
    fn start_runs(&mut self) {
        for index in 0..self.session.programs.len() {
            if self.running.values().any(|running| *running == index) {
                self.missed.insert(index);
                continue;
            }
            self.start(index);
        }
    }

    fn start(&mut self, index: usize) {
        let program = self.session.programs[index].clone();
        let cwd = PathBuf::from(&self.input.cwd);
        let input = self.input.for_program(&program);
        let run = run_one(program, input, cwd);
        let handle = self.runs.spawn(send_answer(run, self.out.clone()));
        self.running.insert(handle.id(), index);
    }

    fn finish(&mut self, done: Result<(Id, ()), JoinError>) {
        let id = match done {
            Ok((id, ())) => id,
            Err(error) => {
                tracing::error!(%error, "custom program run failed to finish");
                error.id()
            }
        };
        let Some(index) = self.running.remove(&id) else { return };
        if self.missed.remove(&index) {
            self.start(index);
        }
    }
}

async fn send_answer(run: impl std::future::Future<Output = Event>, out: mpsc::Sender<Event>) {
    // The front-end closing mid-run is the feed ending, not a failure
    let _ = out.send(run.await).await;
}

/// One run of `program` as the event that reports it. A failure is logged as well as shown,
/// since the row is short and the log can say more about which program it was.
async fn run_one(program: CustomProgram, input: DockInput, cwd: PathBuf) -> Event {
    let columns = input.row_widths.map(|widths| widths.columns(program.place));
    let ran = match serde_json::to_vec(&input) {
        Ok(payload) => {
            run_program(&program, &payload, &cwd, columns).await.map_err(|error| error.to_string())
        }
        Err(error) => Err(format!("could not build its input: {error}")),
    };
    let name = program.name.clone();
    let answer = match ran {
        Ok(rows) => DockAnswer { name, rows, failure: None },
        Err(failure) => {
            tracing::warn!(name = %name, place = ?program.place, command = ?program.command, %failure, "custom program run failed");
            DockAnswer { name, rows: Vec::new(), failure: Some(failure) }
        }
    };
    program.event(answer)
}

#[cfg(all(test, unix))]
mod tests;
