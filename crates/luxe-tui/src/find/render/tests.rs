use super::*;
use crate::find::FindOutcome;
use crate::mouse::{HitRegions, MouseTarget, OverlayList, TabStrip};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use luxe_core::project_search::{
    MatchRange, SearchAnswer, SearchFailed, SearchFailure, SearchOptions, SearchRequestId,
    SearchResults,
};
use std::cell::RefCell;

const WIDTH: u16 = 100;
const HEIGHT: u16 = 30;

fn hit(path: &str, line: usize, text: &str, matches: &[(usize, usize)]) -> SearchHit {
    let matches = matches.iter().map(|&(start, end)| MatchRange { start, end }).collect();
    SearchHit { path: path.into(), line, text: text.into(), clipped: false, matches, scope: None }
}

/// The break this catches: a hit's scope dropped from its row, beside a pane that shows the line
/// but not which function holds it.
#[test]
fn a_hit_names_the_scope_it_sits_in() {
    let scoped =
        SearchHit { scope: Some("FindState › open".into()), ..hit("a.rs", 9, "foo", &[]) };
    let shown = screen(&searched(vec![scoped], 1));
    assert!(shown.contains("a.rs:9") && shown.contains("FindState › open"), "{shown}");
}

/// A Text tab that searched for `foo` and got `hits` back, out of `total`.
fn searched(hits: Vec<SearchHit>, total: usize) -> FindState {
    let mut state = FindState::open(FindTab::Text);
    state.paste("foo");
    let request = SearchRequestId::new();
    state.searching(request.clone(), ("foo".into(), SearchOptions::default()));
    let results = SearchResults { query: "foo".into(), hits, total, ..SearchResults::default() };
    state.accept(&SearchAnswer { request_id: request, results });
    state
}

/// Draws `state` and returns the screen as text rows, with the click targets it published.
fn drawn(state: &FindState) -> (Vec<String>, HitRegions) {
    drawn_at(state, WIDTH, HEIGHT)
}

fn drawn_at(state: &FindState, width: u16, height: u16) -> (Vec<String>, HitRegions) {
    use ratatui::{backend::TestBackend, Terminal};
    let resolved = luxe_core::theme::ThemeRegistry::new().resolve("nord").expect("nord");
    let theme = ThemeStyles::from_resolved(&resolved, "nord");
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
    let hit = RefCell::new(HitRegions::default());
    terminal.draw(|f| render_find(f, f.area(), state, &theme, &hit)).expect("draw");
    let buffer = terminal.backend().buffer().clone();
    let rows = (0..height).map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect()).collect();
    (rows, hit.into_inner())
}

fn screen(state: &FindState) -> String {
    drawn(state).0.join("\n")
}

/// The break this catches: lighting the matches by their offsets in the untrimmed line, which
/// after dropping the indent lights the wrong letters.
#[test]
fn a_hit_lights_its_matches_after_the_indent_is_dropped() {
    let (line, matched) = hit_line(&hit("a.rs", 1, "    let foo = foo;", &[(8, 11), (14, 17)]));
    assert_eq!(line, "let foo = foo;");
    assert_eq!(matched, [4, 5, 6, 10, 11, 12]);
}

/// A tab inside the line would draw as nothing or a jump; a space keeps every index in place.
#[test]
fn a_tab_in_a_hit_draws_as_one_space() {
    let (line, matched) = hit_line(&hit("a.rs", 1, "\tx\tfoo", &[(3, 6)]));
    assert_eq!(line, "x foo");
    assert_eq!(matched, [2, 3, 4]);
}

/// The break this catches: with no room for a preview, a row that drops the line number or the
/// line, so a hit can't be told from the others in one file and there's nowhere else to read it.
#[test]
fn without_room_for_a_preview_a_hit_row_carries_its_line() {
    let state = searched(vec![hit("src/lib.rs", 12, "  let foo = 1;", &[(6, 9)])], 1);
    let screen = drawn_at(&state, 60, 14).0.join("\n");
    assert!(screen.contains("src/lib.rs:12  let foo = 1;"), "{screen}");
    assert!(!screen.contains("Reading…"), "no pane: {screen}");
}

/// The break this catches: the list repeating the line the pane beside it shows, which narrows
/// the list for nothing; or a list that went bare with no pane to read the line in.
#[test]
fn beside_a_preview_a_hit_row_is_where_the_hit_is() {
    let state = searched(vec![hit("src/lib.rs", 12, "  let foo = 1;", &[(6, 9)])], 1);
    let screen = screen(&state);
    assert!(screen.contains("src/lib.rs:12"), "{screen}");
    assert!(!screen.contains("src/lib.rs:12  let foo"), "the row leaves the line to the pane");
    assert!(screen.contains("Reading…"), "the pane is up, reading the file: {screen}");
}

/// The break this catches: a footer that counts only the hits listed, hiding that the search
/// found more than it sent.
#[test]
fn the_footer_counts_every_match_not_just_the_listed_ones() {
    let hits = vec![hit("a.rs", 1, "foo", &[(0, 3)]), hit("b.rs", 2, "foo", &[(0, 3)])];
    let screen = screen(&searched(hits, 40));
    assert!(screen.contains("1/2 of 40"), "{screen}");
}

/// The rows of the last search stay up while the next runs, and the footer says one is out.
#[test]
fn a_search_still_out_says_so() {
    let mut state = searched(vec![hit("a.rs", 1, "foo", &[(0, 3)])], 1);
    state.paste("d");
    state.searching(SearchRequestId::new(), ("food".into(), SearchOptions::default()));
    let screen = screen(&state);
    assert!(screen.contains("searching…"), "{screen}");
    assert!(screen.contains("a.rs:1"), "the last rows stay up meanwhile");
}

/// The break this catches: an option toggled with no sign of it, so the reader can't tell why
/// the rows changed.
#[test]
fn the_footer_shows_each_options_setting() {
    let mut state = FindState::open(FindTab::Text);
    assert!(screen(&state).contains("regex off · alt+c case smart · alt+i ignored off"));
    for c in ['r', 'c', 'i'] {
        state.on_key(&KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT));
    }
    let drawn = screen(&state);
    assert!(drawn.contains("regex on · alt+c case exact · alt+i ignored on"), "{drawn}");
    // With a count beside it the footer has less room, and the settings still have to fit
    let counted = screen(&searched(vec![hit("a.rs", 1, "foo", &[(0, 3)])], 40));
    assert!(counted.contains("alt+i ignored off"), "{counted}");
}

/// The break this catches: a bad regex leaving the list blank, or saying "No matches" when the
/// query was never run.
#[test]
fn a_query_that_is_not_a_regex_says_why() {
    let mut state = FindState::open(FindTab::Text);
    state.paste("(");
    let request = SearchRequestId::new();
    state.searching(request.clone(), ("(".into(), SearchOptions::default()));
    let error = SearchFailure::InvalidPattern("unclosed group".into());
    state.fail(&SearchFailed { request_id: request, query: "(".into(), error });
    assert!(screen(&state).contains("Not a valid regex: unclosed group"));
}

/// The break this catches: the Conversation tab's rows drawn blank, or the message under the
/// cursor missing from the pane beside them.
#[test]
fn a_message_row_says_who_and_the_pane_shows_the_message() {
    let mut state = FindState::open(FindTab::Conversation);
    state.paste("panic");
    let id = crate::app::ConversationItemId::Provisional {
        session_id: "s".into(),
        turn_id: 1,
        block_index: 0,
    };
    let text = "Looking at it.\n\nThe panic is in `parse`.";
    let items =
        [crate::app::ConversationItem::new(id, crate::app::ConvoBlock::Assistant(text.into()))];
    state.set_messages(crate::find::messages::message_hits(&items, "panic"));
    let (rows, _) = drawn_at(&state, 140, 40);
    let row = rows.iter().find(|row| row.contains("Assistant")).expect("the message's row");
    assert!(row.contains("The panic is in `parse`."), "the row carries its line: {row}");
    assert!(rows.iter().any(|row| row.contains("Looking at it.")), "the pane shows the message");
}

/// Each tab's empty list says what it's waiting for, rather than a blank that reads as broken.
#[test]
fn each_empty_tab_says_what_it_needs() {
    assert!(screen(&FindState::open(FindTab::Text)).contains("Type to search every file"));
    let mut conversation = FindState::open(FindTab::Conversation);
    assert!(screen(&conversation).contains("Type to search what was said"));
    conversation.paste("panic");
    let drawn = screen(&conversation);
    assert!(drawn.contains("No messages say that"), "{drawn}");
    let capped = SearchResults { walk_capped: true, files_searched: 20_000, ..Default::default() };
    let mut state = searched(Vec::new(), 0);
    state.results = Some(capped);
    assert!(screen(&state).contains("No matches in the first 20000 files"));
}

/// The break this catches: rows or tabs drawn without click targets, so the mouse that works
/// on every other picker does nothing here.
#[test]
fn rows_and_tabs_answer_the_mouse() {
    let state = searched(vec![hit("a.rs", 1, "foo", &[(0, 3)])], 1);
    let (rows, hit) = drawn(&state);
    let row_y = rows.iter().position(|row| row.contains("a.rs:1")).expect("the row is drawn");
    let x = rows[row_y].find("a.rs").expect("on the row") as u16;
    assert_eq!(hit.at(x, row_y as u16), Some(&MouseTarget::FindRow(0)));
    let files = MouseTarget::Tab { strip: TabStrip::Picker(OverlayList::Find), index: 1 };
    assert!(hit.clicks().any(|(_, target)| *target == files), "the Files tab clicks");
}

/// Clicking the tab that is already open asks for nothing.
#[test]
fn showing_the_open_tab_again_changes_nothing() {
    let mut state = FindState::open(FindTab::Files);
    assert_eq!(state.show_tab(1), FindOutcome::Consumed);
    assert_eq!(state.show_tab(9), FindOutcome::Consumed, "no tab there");
    assert_eq!(state.show_tab(0), FindOutcome::Refresh);
    assert_eq!(state.tab(), FindTab::Text);
}
