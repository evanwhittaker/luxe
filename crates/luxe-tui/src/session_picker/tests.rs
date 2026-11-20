use super::*;
use crate::dashboard::PreviewRole;
use crossterm::event::KeyModifiers;

fn fresh() -> SessionPickerState {
    SessionPickerState::new(vec![
        SessionEntry {
            id: "a".into(),
            name: "Fix palette navigation".into(),
            age: "2h ago".into(),
            search_text: "arrow keys command search".into(),
            preview: vec![
                PreviewTurn { role: PreviewRole::User, text: "arrow keys command search".into() },
                PreviewTurn {
                    role: PreviewRole::Assistant,
                    text: "Done — **rebound** the keys.".into(),
                },
            ],
        },
        SessionEntry {
            id: "b".into(),
            name: "Refine dashboard".into(),
            age: "1d ago".into(),
            search_text: "session picker modal".into(),
            preview: Vec::new(),
        },
        SessionEntry {
            id: "c".into(),
            name: "Provider work".into(),
            age: "2d ago".into(),
            search_text: "stream completion usage".into(),
            preview: Vec::new(),
        },
    ])
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn filter_searches_titles_and_transcript_text() {
    let mut picker = fresh();
    picker.set_query("modal");
    assert_eq!(picker.match_count(), 1);
    assert_eq!(picker.selected_session().map(|s| s.id.as_str()), Some("b"));
}

#[test]
fn empty_query_shows_all() {
    let mut picker = fresh();
    picker.set_query("dashboard");
    picker.set_query("");
    assert_eq!(picker.match_count(), 3);
}

#[test]
fn arrow_navigation_moves_both_directions() {
    let mut picker = fresh();
    picker.on_key(&key(KeyCode::Down));
    assert_eq!(picker.selected_session().map(|s| s.id.as_str()), Some("b"));
    picker.on_key(&key(KeyCode::Up));
    assert_eq!(picker.selected_session().map(|s| s.id.as_str()), Some("a"));
    picker.on_key(&key(KeyCode::Up));
    assert_eq!(picker.selected_session().map(|s| s.id.as_str()), Some("c"));
}

#[test]
fn printable_j_and_k_filter_instead_of_navigating() {
    let mut picker = fresh();
    picker.on_key(&key(KeyCode::Char('j')));
    picker.on_key(&key(KeyCode::Char('k')));
    assert_eq!(picker.query(), "jk");
}

#[test]
fn enter_returns_selected_session() {
    let mut picker = fresh();
    assert_eq!(picker.on_key(&key(KeyCode::Enter)), SessionPickerKey::Selected("a".into()));
}

#[test]
fn backspace_updates_query() {
    let mut picker = fresh();
    picker.on_key(&key(KeyCode::Char('p')));
    picker.on_key(&key(KeyCode::Backspace));
    assert_eq!(picker.query(), "");
    assert_eq!(picker.match_count(), 3);
}

fn dark_theme() -> ThemeStyles {
    let registry = luxe_core::theme::ThemeRegistry::new();
    let resolved = registry.resolve("dark").expect("dark theme resolves");
    ThemeStyles::from_resolved(&resolved, "dark")
}

/// The picker drawn at `width` by `height`, a row of text per screen row.
fn render_at(width: u16, height: u16) -> Vec<String> {
    use ratatui::{backend::TestBackend, Terminal};
    let picker = fresh();
    let theme = dark_theme();
    let hit = std::cell::RefCell::new(crate::mouse::HitRegions::default());
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal
        .draw(|frame| picker.view(frame, frame.area(), &theme, &hit))
        .expect("render session picker");
    let buffer = terminal.backend().buffer();
    (0..height).map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect::<String>()).collect()
}

fn row_of(rows: &[String], needle: &str) -> Option<usize> {
    rows.iter().position(|row| row.contains(needle))
}

/// The break this catches: the preview dropped, or showing some session other than the one
/// under the cursor.
#[test]
fn the_preview_shows_the_session_under_the_cursor() {
    let rows = render_at(90, 24);
    assert!(row_of(&rows, "arrow keys command search").is_some(), "{rows:#?}");
    assert!(row_of(&rows, "stream completion usage").is_none(), "only the selected session");
}

/// The break this catches: markdown in a kept reply dumped as its source.
#[test]
fn the_preview_renders_markdown() {
    let rows = render_at(90, 24).join("\n");
    assert!(rows.contains("rebound"), "the reply is drawn");
    assert!(!rows.contains("**"), "emphasis syntax must not render literally");
}

/// The break this catches: a wide screen stacking the preview under the list, wasting the
/// width; the pane sits beside the list, its rule between them.
#[test]
fn a_wide_screen_puts_the_preview_beside_the_list() {
    let rows = render_at(140, 30);
    let column =
        |row: &String, needle: &str| row[..row.find(needle).expect(needle)].chars().count();
    let list = row_of(&rows, "Refine dashboard").expect("a list row");
    let preview = row_of(&rows, "arrow keys command search").expect("the preview");
    let name = rows[list].find("Refine dashboard").expect("the name");
    let rule = column(&rows[list][name..].to_string(), "│") + column(&rows[list], "Refine");
    assert!(preview < list + 5, "side by side, both near the top: {rows:#?}");
    assert!(column(&rows[preview], "arrow keys") > rule, "the preview is right of the rule");
}

/// The break this catches: a narrow screen losing the preview, which then has room only under
/// the list.
#[test]
fn a_narrow_screen_puts_the_preview_under_the_list() {
    let rows = render_at(60, 30);
    let list = row_of(&rows, "Provider work").expect("a list row");
    let preview = row_of(&rows, "arrow keys command search").expect("the preview");
    assert!(preview > list, "the preview comes after the list: {rows:#?}");
}

/// The break this catches: the sessions count dropped from the frame, or a search still
/// counting every session.
#[test]
fn the_count_says_sessions_or_matches() {
    let mut picker = fresh();
    assert_eq!(picker.count().as_deref(), Some("3 sessions"));
    picker.set_query("modal");
    assert_eq!(picker.count().as_deref(), Some("1 match"));
}
