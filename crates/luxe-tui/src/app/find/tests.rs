use super::*;
use crossterm::event::{KeyCode, KeyModifiers};
use luxe_core::project_search::{SearchAnswer, SearchHit, SearchResults};
use tokio::sync::mpsc::Receiver;

mod failures;
mod review;

fn app() -> (App, Receiver<Action>) {
    let (tx, rx) = tokio::sync::mpsc::channel(32);
    let config = luxe_core::config::Config::default();
    let mut app = App::new(tx, luxe_core::keymap::KeyMap::defaults(), config);
    app.file_provider = crate::ref_provider::FileRefProvider::with_files(
        ["src/lib.rs", "src/main.rs", "README.md"].map(String::from),
    );
    (app, rx)
}

fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    app.update(Msg::Key(KeyEvent::new(code, modifiers)));
}

fn type_text(app: &mut App, text: &str) {
    for c in text.chars() {
        press(app, KeyCode::Char(c), KeyModifiers::NONE);
    }
}

/// The search the engine was last asked for.
fn last_search(rx: &mut Receiver<Action>) -> (SearchRequestId, String) {
    let mut last = None;
    while let Ok(action) = rx.try_recv() {
        if let Action::SearchProject { request_id, query, .. } = action {
            last = Some((request_id, query));
        }
    }
    last.expect("a search was sent")
}

fn answered(request_id: SearchRequestId, path: &str, line: usize) -> Event {
    let hit = SearchHit {
        path: path.into(),
        line,
        text: "x".into(),
        clipped: false,
        matches: vec![],
        scope: None,
    };
    let results =
        SearchResults { query: "x".into(), hits: vec![hit], total: 1, ..Default::default() };
    Event::ProjectSearched(Box::new(SearchAnswer { request_id, results }))
}

fn find(app: &App) -> &FindState {
    app.overlays.find.as_ref().expect("Find is open")
}

/// The break this catches: keys typed into Find never reaching the engine as a search, or
/// reaching the composer behind the overlay instead.
#[test]
fn typing_sends_a_search_for_the_query() {
    let (mut app, mut rx) = app();
    open(&mut app);
    type_text(&mut app, "todo");
    assert_eq!(last_search(&mut rx).1, "todo");
    assert_eq!(app.editor.current_input(), "", "the composer behind it took nothing");
}

/// The break this catches: the answer event going to the table that ignores it, or a stale
/// answer for an earlier keystroke filling the rows.
#[test]
fn only_the_answer_for_the_latest_search_lands() {
    let (mut app, mut rx) = app();
    open(&mut app);
    type_text(&mut app, "a");
    let (stale, _) = last_search(&mut rx);
    type_text(&mut app, "b");
    let (latest, _) = last_search(&mut rx);
    app.update(Msg::Engine(answered(stale, "old.rs", 1)));
    assert_eq!(find(&app).results(), None);
    app.update(Msg::Engine(answered(latest, "src/lib.rs", 12)));
    assert_eq!(find(&app).row_count(), 1);
}

/// The break this catches: Enter opening the file at its top, on its diff, or without the
/// keyboard, so the reader can't move from the hit they asked for.
#[test]
fn enter_opens_the_hit_in_the_preview_at_its_line() {
    let (mut app, mut rx) = app();
    open(&mut app);
    type_text(&mut app, "x");
    let (request, _) = last_search(&mut rx);
    app.update(Msg::Engine(answered(request, "src/lib.rs", 12)));
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.overlays.find.is_none(), "Find closes on the way out");
    let view = app.changes.as_ref().expect("the preview opened");
    assert_eq!(view.path, "src/lib.rs");
    assert_eq!(view.view, crate::changes::PeekView::File);
    assert_eq!(view.cursor, Some(12));
    assert!(view.focused, "the preview has the keyboard");
    let read = std::iter::from_fn(|| rx.try_recv().ok()).any(
        |action| matches!(action, Action::ReadProjectFile { path, .. } if path == "src/lib.rs"),
    );
    assert!(read, "the file's text was asked for");
}

/// The break this catches: a hit opening in a file with nothing lit, so the reader hunts the line
/// for what they searched; or lit with what was typed after the rows came back.
#[test]
fn a_text_hit_opens_with_its_query_lit() {
    let (mut app, mut rx) = app();
    open(&mut app);
    type_text(&mut app, "x");
    let (request, _) = last_search(&mut rx);
    app.update(Msg::Engine(answered(request, "src/lib.rs", 12)));
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    let view = app.changes.as_ref().expect("the preview opened");
    assert_eq!(view.found, crate::changes::FoundQuery::new("x", Default::default()));
}

/// The break this catches: a file name typed on the Files tab lit inside the file, where it's
/// not what the reader was looking for.
#[test]
fn a_file_opened_from_the_files_tab_lights_nothing() {
    let (mut app, _rx) = app();
    app.overlays.find_tab = FindTab::Files;
    open(&mut app);
    type_text(&mut app, "main");
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    let view = app.changes.as_ref().expect("the preview opened");
    assert_eq!(view.path, "src/main.rs");
    assert_eq!(view.found, None);
}

/// Every file read the engine was asked for, in order.
fn reads(rx: &mut Receiver<Action>) -> Vec<(luxe_core::project_files::FileReadRequestId, String)> {
    std::iter::from_fn(|| rx.try_recv().ok())
        .filter_map(|action| match action {
            Action::ReadProjectFile { request_id, path } => Some((request_id, path)),
            _ => None,
        })
        .collect()
}

fn file_read(request_id: luxe_core::project_files::FileReadRequestId, path: &str) -> Event {
    let file = luxe_core::project_files::ProjectFile {
        path: path.into(),
        text: "x\n".into(),
        bytes: 2,
        truncated: false,
        binary: false,
        image: None,
    };
    Event::ProjectFileRead { request_id, file: Box::new(file) }
}

/// The break this catches: the pane never asking for the file under the cursor, asking again on
/// every frame, or its answer landing somewhere other than Find.
#[test]
fn the_row_under_the_cursor_reads_its_file_once() {
    let (mut app, mut rx) = app();
    open(&mut app);
    type_text(&mut app, "x");
    let (request, _) = last_search(&mut rx);
    app.update(Msg::Engine(answered(request, "src/lib.rs", 12)));
    let asked = reads(&mut rx);
    assert_eq!(asked.iter().map(|(_, path)| path.as_str()).collect::<Vec<_>>(), ["src/lib.rs"]);
    app.update(Msg::Engine(file_read(asked[0].0.clone(), "src/lib.rs")));
    let held = find(&app).previews().get("src/lib.rs");
    assert!(matches!(held, Some(crate::find::PreviewText::Ready(_))), "{held:?}");
    press(&mut app, KeyCode::Down, KeyModifiers::NONE);
    assert!(reads(&mut rx).is_empty(), "a file already here isn't read again");
}

/// The break this catches: Find claiming a read it never sent, so the preview pane behind it
/// waits forever for its own file.
#[test]
fn a_read_find_did_not_send_reaches_the_preview_pane() {
    let (mut app, _rx) = app();
    crate::app::messages::changes::open_at_line(&mut app, "src/main.rs".into(), 1);
    let pane_read = app.changes.as_ref().and_then(|view| view.text_request.clone());
    let pane_read = pane_read.expect("the preview pane asked for its text");
    open(&mut app);
    app.update(Msg::Engine(file_read(pane_read, "src/main.rs")));
    let view = app.changes.as_ref().expect("the pane");
    assert!(matches!(view.text, crate::changes::TextLoad::Ready(_)), "{:?}", view.text);
}

/// The break this catches: Alt+M mentioning the file without the line, or leaving Find open
/// over the draft it just wrote to.
#[test]
fn alt_m_mentions_the_hit_in_the_draft() {
    let (mut app, mut rx) = app();
    app.editor.insert_text("look at");
    open(&mut app);
    type_text(&mut app, "x");
    let (request, _) = last_search(&mut rx);
    app.update(Msg::Engine(answered(request, "src/lib.rs", 12)));
    press(&mut app, KeyCode::Char('m'), KeyModifiers::ALT);
    assert!(app.overlays.find.is_none());
    assert_eq!(app.editor.current_input(), "look at @[file:src/lib.rs:12] ");
}

/// The break this catches: the Files tab showing nothing, or ranking without the query.
#[test]
fn the_files_tab_ranks_names_against_the_query() {
    let (mut app, mut rx) = app();
    app.overlays.find_tab = FindTab::Files;
    open(&mut app);
    type_text(&mut app, "main");
    let first = find(&app).files().first().map(|file| file.label.as_str());
    assert_eq!(first, Some("src/main.rs"));
    let searched = std::iter::from_fn(|| rx.try_recv().ok())
        .any(|action| matches!(action, Action::SearchProject { .. }));
    assert!(!searched, "naming files searches nothing; only the top file is read to preview");
}

/// A conversation long enough to scroll: filler, a message saying `needle`, more filler.
fn long_conversation(app: &mut App) {
    for index in 0..30 {
        app.push_convo(crate::app::ConvoBlock::Assistant(format!("filler reply {index}")));
    }
    app.push_convo(crate::app::ConvoBlock::Assistant("the needle was in the config".into()));
    for index in 30..60 {
        app.push_convo(crate::app::ConvoBlock::Assistant(format!("filler reply {index}")));
    }
}

fn draw(app: &mut App) -> String {
    let backend = ratatui::backend::TestBackend::new(100, 30);
    let mut terminal = ratatui::Terminal::new(backend).expect("terminal");
    terminal.draw(|frame| crate::view::render(frame, app)).expect("a frame");
    let buffer = terminal.backend().buffer();
    buffer.content.iter().map(ratatui::buffer::Cell::symbol).collect()
}

/// The break this catches: the Conversation tab listing nothing as the query is typed, as it
/// did when it only handed the query on.
#[test]
fn the_conversation_tab_lists_the_messages_that_say_it() {
    let (mut app, _rx) = app();
    long_conversation(&mut app);
    app.overlays.find_tab = FindTab::Conversation;
    open(&mut app);
    type_text(&mut app, "needle");
    assert_eq!(find(&app).row_count(), 1);
    assert_eq!(find(&app).messages()[0].line, "the needle was in the config");
}

/// The break this catches: Enter on a message leaving the transcript where it was, at its
/// bottom, so the reader still has to hunt for what they picked; or the search it opens not
/// starting from that message's line.
#[test]
fn going_to_a_message_scrolls_the_transcript_to_it() {
    let (mut app, _rx) = app();
    long_conversation(&mut app);
    draw(&mut app);
    assert!(!draw(&mut app).contains("needle"), "it starts off screen");
    app.overlays.find_tab = FindTab::Conversation;
    open(&mut app);
    type_text(&mut app, "needle");
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.overlays.find.is_none());
    assert_eq!(app.search_state.query, "needle");
    assert!(draw(&mut app).contains("the needle was in the config"), "scrolled to it");
    assert_eq!(app.search_state.current_match, Some(0), "n goes on from its line");
    let line = app.search_state.matches[0];
    let view = &app.scroll_state;
    assert_eq!(line - view.offset, view.viewport / 3, "a third of the way down");
    assert_eq!(app.search_state.jump_to, None, "a jump happens once");
    open(&mut app);
    assert_eq!(find(&app).tab(), FindTab::Conversation, "Find opens where it was left");
}

/// The break this catches: a message whose match the transcript draws differently (here the
/// query is markdown the transcript renders away) leaving the view where it was, or landing on
/// some other message, instead of on the one picked.
#[test]
fn a_match_drawn_differently_still_lands_on_its_message() {
    let (mut app, _rx) = app();
    long_conversation(&mut app);
    app.push_convo(crate::app::ConvoBlock::Assistant("the **marked** word".into()));
    for index in 0..30 {
        app.push_convo(crate::app::ConvoBlock::Assistant(format!("later reply {index}")));
    }
    draw(&mut app);
    app.overlays.find_tab = FindTab::Conversation;
    open(&mut app);
    type_text(&mut app, "**marked**");
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert!(draw(&mut app).contains("the marked word"), "on the message all the same");
}

/// A paste types into the query and searches, rather than landing in the composer behind.
#[test]
fn a_paste_searches() {
    let (mut app, mut rx) = app();
    open(&mut app);
    app.update(Msg::PasteText("fn main".into()));
    assert_eq!(last_search(&mut rx).1, "fn main");
    assert_eq!(app.editor.current_input(), "");
}
