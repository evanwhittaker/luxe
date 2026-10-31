//! probe — what the terminal says about itself when asked.
//!
//! Environment variables name a terminal; they do not say what it can do, and for Sixel what it
//! can do depends on its version. Windows Terminal answers a device-attributes query with a list
//! that includes `4` when it draws Sixel, and a cell-size query with the pixels a cell is — the
//! one number a Sixel image is scaled against, and one Windows has no `TIOCGWINSZ` to give.
//!
//! Asked once, before the event loop takes over the input, because a reply arrives as input and is
//! indistinguishable from typing once anything else is reading it. On Windows the console hands it
//! over as key events; under WSL it is bytes on `/dev/tty`. The parsing is the same either way.

use super::CellSize;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

/// What a terminal answered to the startup queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TerminalReport {
    /// Whether the device attributes list Sixel graphics.
    pub sixel: bool,
    /// Pixels in one cell, when the terminal said.
    pub cell: Option<CellSize>,
}

/// The two questions: device attributes, then the size of a cell in pixels.
const QUERIES: &[u8] = b"\x1b[c\x1b[16t";

/// How long to wait for the replies. A local terminal answers in a few milliseconds; this is for
/// one that never will.
const PATIENCE: Duration = Duration::from_millis(400);

static RECORDED: Mutex<Option<TerminalReport>> = Mutex::new(None);

/// Remembers what the terminal answered, replacing an earlier answer.
pub(crate) fn record(report: Option<TerminalReport>) {
    *RECORDED.lock().unwrap_or_else(PoisonError::into_inner) = report;
}

/// What the terminal answered at startup, or `None` if nothing asked or nothing answered.
pub(crate) fn recorded() -> Option<TerminalReport> {
    *RECORDED.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Device attributes: the numbers in `ESC [ ? … c`, or `None` before the reply is complete.
///
/// Matched from the `[`: the Windows console hands the reply over as keystrokes, and measured on
/// Windows Terminal 1.25 the `ESC` is not among them.
fn attributes(text: &str) -> Option<Vec<u32>> {
    let start = text.find("[?")? + 2;
    let end = start + text[start..].find('c')?;
    Some(text[start..end].split(';').filter_map(|number| number.parse().ok()).collect())
}

/// Cell size from `ESC [ 6 ; height ; width t`, or `None` before the reply is complete or when it
/// reports a cell with no area. Matched from the `[`, for the reason [`attributes`] is.
fn cell_pixels(text: &str) -> Option<CellSize> {
    let start = text.find("[6;")? + 3;
    let end = start + text[start..].find('t')?;
    let (height, width) = text[start..end].split_once(';')?;
    let (height, width) = (height.parse::<u16>().ok()?, width.parse::<u16>().ok()?);
    (height > 0 && width > 0).then_some(CellSize { width, height })
}

/// Reads the replies in `text`. `None` means the device-attributes reply is missing, which is the
/// one every terminal that is listening sends.
pub(crate) fn parse_replies(text: &str) -> Option<TerminalReport> {
    let attributes = attributes(text)?;
    Some(TerminalReport { sixel: attributes.contains(&4), cell: cell_pixels(text) })
}

/// Whether `text` holds both replies, so there is nothing left to wait for.
fn complete(text: &str) -> bool {
    attributes(text).is_some() && cell_pixels(text).is_some()
}

/// Asks the terminal, when it is one worth asking: Windows Terminal, natively or under WSL, which
/// forwards `WT_SESSION` into Linux. Not behind a multiplexer, which answers for itself, and whose
/// images get words whatever it says.
pub(crate) fn run() {
    let env = |name: &str| std::env::var(name);
    if should_ask(&env) {
        record(query());
    }
}

/// [`run`]'s decision, against an environment lookup.
fn should_ask(env: super::detect::Env) -> bool {
    env("WT_SESSION").is_ok_and(|session| !session.is_empty())
        && !super::detect::behind_multiplexer(env)
}

#[cfg(windows)]
fn query() -> Option<TerminalReport> {
    use std::io::Write;

    let mut stdout = std::io::stdout();
    stdout.write_all(QUERIES).ok()?;
    stdout.flush().ok()?;
    parse_replies(&read_replies())
}

/// The characters the terminal typed back, up to both replies or the time it is given.
///
/// Through crossterm, because that is how the Windows console delivers them: as key events.
#[cfg(windows)]
fn read_replies() -> String {
    use crossterm::event::{poll, read, Event};

    let deadline = Instant::now() + PATIENCE;
    let mut text = String::new();
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        if !poll(left).unwrap_or(false) {
            break;
        }
        if let Ok(Event::Key(key)) = read() {
            push_typed(&mut text, key);
        }
        if complete(&text) {
            break;
        }
    }
    text
}

/// Adds what a key stands for to the reply being collected.
#[cfg(windows)]
fn push_typed(text: &mut String, key: crossterm::event::KeyEvent) {
    use crossterm::event::{KeyCode, KeyEventKind};

    match key.code {
        _ if key.kind == KeyEventKind::Release => {}
        KeyCode::Esc => text.push('\x1b'),
        KeyCode::Char(character) => text.push(character),
        _ => {}
    }
}

/// Asks through `/dev/tty`, which raw mode already covers by the time this runs.
///
/// Not through crossterm: on unix it takes a device-attributes reply as its own internal event and
/// never hands it back, so the reply has to be read before crossterm is listening.
#[cfg(unix)]
fn query() -> Option<TerminalReport> {
    use std::io::Write;

    let mut tty = std::fs::OpenOptions::new().read(true).write(true).open("/dev/tty").ok()?;
    tty.write_all(QUERIES).ok()?;
    tty.flush().ok()?;
    parse_replies(&read_tty_replies(&mut tty))
}

/// The bytes the terminal sent back, up to both replies or the time it is given.
#[cfg(unix)]
fn read_tty_replies(tty: &mut std::fs::File) -> String {
    use std::io::Read;

    let deadline = Instant::now() + PATIENCE;
    let mut bytes = Vec::new();
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        if !readable_within(tty, left) {
            break;
        }
        let mut chunk = [0u8; 256];
        let Ok(read @ 1..) = tty.read(&mut chunk) else { break };
        bytes.extend_from_slice(&chunk[..read]);
        if complete(&String::from_utf8_lossy(&bytes)) {
            break;
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Whether `tty` has something to read before `wait` is up.
#[cfg(unix)]
fn readable_within(tty: &std::fs::File, wait: Duration) -> bool {
    use rustix::event::{poll, PollFd, PollFlags, Timespec};

    let timeout = Timespec::try_from(wait).ok();
    let mut ready = [PollFd::new(tty, PollFlags::IN)];
    matches!(poll(&mut ready, timeout.as_ref()), Ok(count) if count > 0)
}

#[cfg(not(any(windows, unix)))]
fn query() -> Option<TerminalReport> {
    None
}

#[cfg(test)]
mod tests;
