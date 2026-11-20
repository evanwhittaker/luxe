use super::*;
use luxe_core::project_search::{MatchRange, SearchFailure, SearchHit};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn alt(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT)
}

fn typed(state: &mut FindState, text: &str) {
    for c in text.chars() {
        state.on_key(&key(KeyCode::Char(c)));
    }
}

fn hit(path: &str, line: usize) -> SearchHit {
    let matches = vec![MatchRange { start: 0, end: 2 }];
    let text = "ab cd".into();
    SearchHit { path: path.into(), line, text, clipped: false, matches, scope: None }
}

fn answer(request_id: SearchRequestId, hits: Vec<SearchHit>) -> SearchAnswer {
    let total = hits.len();
    let results = SearchResults { query: "ab".into(), hits, total, ..SearchResults::default() };
    SearchAnswer { request_id, results }
}

/// A Text tab that searched for `ab` and found two lines.
fn with_hits() -> FindState {
    let mut state = FindState::open(FindTab::Text);
    typed(&mut state, "ab");
    let request = SearchRequestId::new();
    let asked = state.next_search().expect("a query to search");
    state.searching(request.clone(), asked);
    state.accept(&answer(request, vec![hit("src/a.rs", 3), hit("src/b.rs", 7)]));
    state
}

/// The break this catches: what lights the rows' matches following the field instead of the
/// rows, so text typed or an option flipped since the answer lights something the rows never
/// matched; or the Files tab lighting a file name inside a file.
#[test]
fn the_rows_query_is_what_the_rows_were_searched_for() {
    let mut state = with_hits();
    typed(&mut state, "x");
    state.on_key(&alt('c'));
    assert_eq!(state.rows_query(), Some(("ab", SearchOptions::default())));
    state.on_key(&key(KeyCode::Tab));
    assert_eq!(state.rows_query(), None, "the Files tab has no text query");
}

fn candidate(path: &str) -> Candidate {
    Candidate { label: path.into(), insert: path.into(), path: true, ..Candidate::default() }
}

/// The break this catches: forgetting what was last asked, so every refresh (a tab switch, a
/// redraw) sends the same search again.
#[test]
fn a_query_asks_for_one_search_until_it_changes() {
    let mut state = FindState::open(FindTab::Text);
    assert_eq!(state.on_key(&key(KeyCode::Char('a'))), FindOutcome::Refresh);
    let asked = state.next_search().expect("a typed query needs a search");
    assert_eq!(asked, ("a".to_string(), SearchOptions::default()));
    state.searching(SearchRequestId::new(), asked);
    assert_eq!(state.next_search(), None, "the same query was already asked");
    typed(&mut state, "b");
    assert_eq!(state.next_search().map(|(query, _)| query).as_deref(), Some("ab"));
}

/// The break this catches: taking any answer that arrives, so a slow search for an old query
/// replaces the rows for the one typed since.
#[test]
fn an_answer_for_an_older_search_is_dropped() {
    let mut state = FindState::open(FindTab::Text);
    typed(&mut state, "ab");
    let (old, new) = (SearchRequestId::new(), SearchRequestId::new());
    state.searching(old.clone(), ("a".into(), SearchOptions::default()));
    state.searching(new.clone(), ("ab".into(), SearchOptions::default()));
    assert!(!state.accept(&answer(old, vec![hit("stale.rs", 1)])));
    assert_eq!(state.results(), None);
    assert!(state.is_searching(), "the newer search is still out");
    assert!(state.accept(&answer(new, vec![hit("fresh.rs", 1)])));
    assert_eq!(state.results().map(|results| results.hits[0].path.as_str()), Some("fresh.rs"));
    assert!(!state.is_searching());
}

/// The break this catches: a failure for a search the reader has moved past replacing the rows
/// with an error about a query they no longer have.
#[test]
fn only_the_search_in_flight_can_fail() {
    let mut state = FindState::open(FindTab::Text);
    let (old, new) = (SearchRequestId::new(), SearchRequestId::new());
    state.searching(old.clone(), ("(".into(), SearchOptions::default()));
    state.searching(new.clone(), ("(a)".into(), SearchOptions::default()));
    let failed = |request_id| SearchFailed {
        request_id,
        query: "(".into(),
        error: SearchFailure::InvalidPattern("unclosed group".into()),
    };
    assert!(!state.fail(&failed(old)));
    assert_eq!(state.error(), None);
    assert!(state.fail(&failed(new)));
    assert_eq!(state.error(), Some("not a valid regex: unclosed group"));
}

/// The break this catches: an emptied query leaving the last search's rows up, where Enter would
/// still open a hit for text no longer in the field.
#[test]
fn emptying_the_query_clears_the_rows() {
    let mut state = with_hits();
    state.on_key(&key(KeyCode::Backspace));
    assert_eq!(state.on_key(&key(KeyCode::Backspace)), FindOutcome::Refresh);
    assert_eq!(state.next_search(), None, "an empty query has nothing to ask");
    assert_eq!(state.results(), None);
    assert_eq!(state.on_key(&key(KeyCode::Enter)), FindOutcome::Consumed);
}

/// Backspace on an empty field changes nothing, so it asks for nothing.
#[test]
fn a_key_that_changes_nothing_does_not_refresh() {
    let mut state = FindState::open(FindTab::Text);
    assert_eq!(state.on_key(&key(KeyCode::Backspace)), FindOutcome::Consumed);
}

/// The break this catches: Enter opening the first hit whatever the cursor is on, or dropping
/// the line so the preview opens at the top.
#[test]
fn enter_opens_the_selected_hit_at_its_line() {
    let mut state = with_hits();
    state.on_key(&key(KeyCode::Down));
    let want = FindTarget { path: "src/b.rs".into(), line: Some(7) };
    assert_eq!(state.on_key(&key(KeyCode::Enter)), FindOutcome::Open(want));
}

/// The cursor wraps both ways, as the palette's does.
#[test]
fn the_cursor_wraps_at_both_ends() {
    let mut state = with_hits();
    state.on_key(&key(KeyCode::Up));
    assert_eq!(state.selected(), 1, "up from the top lands on the last row");
    state.on_key(&KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL));
    assert_eq!(state.selected(), 0, "down from the last row lands on the top");
}

/// The break this catches: Up moving down, which two rows can't show since either way lands on
/// the other one; Ctrl+P moves up as Up does.
#[test]
fn up_moves_up_among_three_rows() {
    let mut state = FindState::open(FindTab::Files);
    state.set_files(vec![candidate("a.rs"), candidate("b.rs"), candidate("c.rs")]);
    state.on_key(&key(KeyCode::Up));
    assert_eq!(state.selected(), 2);
    state.on_key(&KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
    assert_eq!(state.selected(), 1);
}

/// The break this catches: a Ctrl chord Find doesn't use typing its letter into the query.
#[test]
fn a_ctrl_chord_types_nothing() {
    let mut state = FindState::open(FindTab::Text);
    let outcome = state.on_key(&KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL));
    assert_eq!(outcome, FindOutcome::Consumed);
    assert_eq!(state.query(), "");
}

/// The break this catches: a click that doesn't move the cursor, or one past the last row that
/// leaves it there with nothing to open.
#[test]
fn a_click_moves_the_cursor_within_the_rows() {
    let mut state = with_hits();
    state.select(1);
    assert_eq!(state.selected(), 1);
    state.select(9);
    assert_eq!(state.selected(), 1, "held to the last row");
}

/// The break this catches: a search the engine never got leaving the old rows up as if it ran,
/// and asking nothing next time since it thinks the query was asked.
#[test]
fn a_search_that_was_not_sent_says_why_and_asks_again() {
    let mut state = with_hits();
    state.unsent("luxe is not accepting requests");
    assert_eq!(state.results(), None);
    assert_eq!(state.error(), Some("luxe is not accepting requests"));
    assert!(state.next_search().is_some(), "the query is still to be asked");
}

/// The break this catches: a mention losing its line, or using a syntax the composer doesn't
/// read. Both forms are the ones the `@` picker and the preview's Ask already write.
#[test]
fn a_mention_points_at_the_line_when_there_is_one() {
    let mut state = with_hits();
    let want = FindTarget { path: "src/a.rs".into(), line: Some(3) };
    assert_eq!(state.on_key(&alt('m')), FindOutcome::Mention(want.clone()));
    assert_eq!(want.mention(), "@[file:src/a.rs:3]");
    let file = FindTarget { path: "src/a.rs".into(), line: None };
    assert_eq!(file.mention(), "@[file:src/a.rs]");
}

/// The Files tab's rows are whole files: Enter opens one with no line to land on.
#[test]
fn a_file_row_opens_the_file() {
    let mut state = FindState::open(FindTab::Files);
    typed(&mut state, "lib");
    state.set_files(vec![candidate("src/lib.rs"), candidate("crates/x/lib.rs")]);
    state.on_key(&key(KeyCode::Down));
    let want = FindTarget { path: "crates/x/lib.rs".into(), line: None };
    assert_eq!(state.on_key(&key(KeyCode::Enter)), FindOutcome::Open(want));
}

/// The break this catches: the Files tab sending a text search the engine runs for nothing.
#[test]
fn only_the_text_tab_searches_the_project() {
    let mut state = FindState::open(FindTab::Files);
    typed(&mut state, "lib");
    assert_eq!(state.next_search(), None);
    state.on_key(&KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    assert_eq!(state.tab(), FindTab::Text);
    assert!(state.next_search().is_some(), "the query carries over to the tab that searches");
}

fn said(index: usize, text: &str) -> crate::app::ConversationItem {
    let id = crate::app::ConversationItemId::Provisional {
        session_id: "s".into(),
        turn_id: 1,
        block_index: index,
    };
    crate::app::ConversationItem::new(id, crate::app::ConvoBlock::Assistant(text.into()))
}

/// The break this catches: Enter on the Conversation tab going to the wrong message, or to one
/// with no row under the cursor, or a message offered as a file to mention.
#[test]
fn the_conversation_tab_goes_to_the_message_under_the_cursor() {
    let mut state = FindState::open(FindTab::Conversation);
    assert_eq!(state.on_key(&key(KeyCode::Enter)), FindOutcome::Consumed, "no rows yet");
    typed(&mut state, "panic");
    let items = [said(0, "a panic here"), said(1, "and a panic there")];
    state.set_messages(messages::message_hits(&items, state.query()));
    assert_eq!(state.row_count(), 2);
    state.on_key(&key(KeyCode::Down));
    assert_eq!(state.on_key(&alt('m')), FindOutcome::Consumed, "no file to mention");
    let want = FindOutcome::ShowMessage { item: items[0].id.clone(), query: "panic".into() };
    assert_eq!(state.on_key(&key(KeyCode::Enter)), want, "newest first, so the second row");
}

/// Tab walks Text, Files, Conversation and back round; Shift+Tab walks the other way.
#[test]
fn tab_cycles_through_the_tabs() {
    let mut state = FindState::open(FindTab::Text);
    let mut seen = Vec::new();
    for _ in 0..3 {
        assert_eq!(state.on_key(&key(KeyCode::Tab)), FindOutcome::Refresh);
        seen.push(state.tab());
    }
    assert_eq!(seen, [FindTab::Files, FindTab::Conversation, FindTab::Text]);
}

/// The break this catches: an option toggle that changes the label but not the search, so the
/// rows keep answering the old question.
#[test]
fn each_option_key_changes_what_is_asked() {
    let mut state = FindState::open(FindTab::Text);
    typed(&mut state, "a");
    let asked = |state: &mut FindState| state.next_search().expect("a changed search").1;
    state.on_key(&alt('r'));
    assert!(asked(&mut state).regex);
    state.on_key(&alt('i'));
    assert!(asked(&mut state).include_ignored);
    let mut cases = Vec::new();
    for _ in 0..3 {
        state.on_key(&alt('c'));
        cases.push(asked(&mut state).case_sensitive);
    }
    assert_eq!(cases, [Some(true), Some(false), None], "exact, ignored, then smart again");
}

/// Typing with Shift held still types: some terminals report a capital with the modifier on.
#[test]
fn a_shifted_letter_is_typed() {
    let mut state = FindState::open(FindTab::Text);
    state.on_key(&KeyEvent::new(KeyCode::Char('F'), KeyModifiers::SHIFT));
    assert_eq!(state.query(), "F");
}

/// A pasted line searches like typing it; the newline a copy often carries is dropped.
#[test]
fn a_paste_types_into_the_query() {
    let mut state = FindState::open(FindTab::Text);
    assert_eq!(state.paste("fn main\n"), FindOutcome::Refresh);
    assert_eq!(state.query(), "fn main");
    assert_eq!(state.paste(""), FindOutcome::Consumed);
}

/// The break this catches: new rows keeping a cursor past their end, so Enter opens nothing.
#[test]
fn new_rows_put_the_cursor_back_on_the_first() {
    let mut state = with_hits();
    state.on_key(&key(KeyCode::Down));
    let request = SearchRequestId::new();
    state.searching(request.clone(), ("abc".into(), SearchOptions::default()));
    state.accept(&answer(request, vec![hit("only.rs", 1)]));
    assert_eq!(state.selected(), 0);
}
