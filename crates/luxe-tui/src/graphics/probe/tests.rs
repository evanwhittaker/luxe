use super::*;

/// What Windows Terminal 1.25 actually sent for `ESC [ c` and `ESC [ 16 t`, copied from a probe
/// run rather than written from the spec, so a change in how it replies shows up here.
const WINDOWS_TERMINAL: &str = "\x1b[?61;4;6;7;14;21;22;23;24;28;32;42;52c\x1b[6;20;10t";

#[test]
fn windows_terminal_reports_sixel_and_its_cell_size() {
    let report = parse_replies(WINDOWS_TERMINAL).expect("answered");
    assert!(report.sixel);
    assert_eq!(report.cell, Some(CellSize { width: 10, height: 20 }));
}

/// A terminal that answers but does not list `4` has no Sixel, however it is named. The break this
/// catches: reading the `6` in the list as the capability, or any digit match at all.
#[test]
fn a_terminal_without_four_in_its_attributes_has_no_sixel() {
    let report = parse_replies("\x1b[?61;6;7;14;21;22;23;24;28;32;42;52c").expect("answered");
    assert!(!report.sixel);
    assert_eq!(report.cell, None, "and it never said how big a cell is");
}

/// A cell size can arrive without the attributes (or the attributes be cut short); neither is
/// treated as an answer, because the attributes are what says the terminal is listening at all.
#[test]
fn an_incomplete_reply_is_no_answer() {
    assert_eq!(parse_replies(""), None);
    assert_eq!(parse_replies("\x1b[6;20;10t"), None);
    assert_eq!(parse_replies("\x1b[?61;4;6"), None);
}

/// A cell reported with no area would divide every fit by zero; it is dropped.
#[test]
fn a_cell_with_no_area_is_not_a_cell() {
    assert_eq!(cell_pixels("\x1b[6;0;0t"), None);
    assert_eq!(cell_pixels("\x1b[6;20;0t"), None);
    assert_eq!(cell_pixels("\x1b[6;nonsense;10t"), None);
}

/// What the Windows console actually delivered for the same two replies: the characters, with the
/// `ESC` of each missing. Copied from a logged run of the probe.
///
/// The break this catches: requiring the `ESC`, which finds no reply at all on Windows and leaves
/// every session without images.
const WINDOWS_CONSOLE_KEYSTROKES: &str = "[?61;4;6;7;14;21;22;23;24;28;32;42;52c[6;20;10t";

#[test]
fn the_replies_are_read_without_their_escape_characters() {
    let report = parse_replies(WINDOWS_CONSOLE_KEYSTROKES).expect("answered");
    assert!(report.sixel);
    assert_eq!(report.cell, Some(CellSize { width: 10, height: 20 }));
}

fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Result<String, std::env::VarError> {
    let pairs: Vec<(String, String)> =
        pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect();
    move |name: &str| {
        let found = pairs.iter().find(|(key, _)| key == name).map(|(_, value)| value.clone());
        found.ok_or(std::env::VarError::NotPresent)
    }
}

/// Windows Terminal is asked, natively (no `TERM`) and under WSL (`TERM` set, `WT_SESSION`
/// forwarded through `WSLENV`).
///
/// The break this catches: a platform check left in front of the question, which leaves every WSL
/// session with a line of text where its screenshot should be.
#[test]
fn windows_terminal_is_asked_natively_and_under_wsl() {
    assert!(should_ask(&env_of(&[("WT_SESSION", "37c93e90")])));
    assert!(should_ask(&env_of(&[("WT_SESSION", "37c93e90"), ("TERM", "xterm-256color")])));
}

/// Nothing else is asked: a terminal that is not Windows Terminal, or Windows Terminal behind tmux,
/// which answers for itself and costs every launch the full wait for a reply that never comes.
#[test]
fn nothing_else_is_asked() {
    assert!(!should_ask(&env_of(&[("TERM", "xterm-256color")])));
    assert!(!should_ask(&env_of(&[("WT_SESSION", "")])));
    assert!(!should_ask(&env_of(&[("WT_SESSION", "x"), ("TMUX", "/tmp/tmux-0/default,1,0")])));
    assert!(!should_ask(&env_of(&[("WT_SESSION", "x"), ("TERM", "screen-256color")])));
}
