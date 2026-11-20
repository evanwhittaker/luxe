use super::*;
use luxe_core::message::Message;
use luxe_core::session::EntryId;

fn entry(kind: EntryKind) -> Entry {
    Entry { id: EntryId::new(), parent_id: None, ts: 0, kind }
}

fn summary(id: &str, name: &str) -> SessionSummary {
    SessionSummary {
        id: id.into(),
        name: name.into(),
        age: "1d ago".into(),
        search_text: name.into(),
        preview: Vec::new(),
        worktree: None,
    }
}

/// The same prompt run three times used to fill the recent list with three identical rows — the
/// worst use of five lines available, since it hides three other sessions *and* says nothing
/// new about the one it repeats.
#[test]
fn the_recent_list_folds_out_repeated_titles() {
    let catalog = vec![
        summary("a", "Read the file rust-toolchain.toml"),
        summary("b", "read the FILE rust-toolchain.toml"),
        summary("c", "Audit the provider stall"),
    ];
    let recent = dedupe_recent(&catalog);
    assert_eq!(recent.len(), 2, "case is not a different session: {recent:?}");
    assert_eq!(recent[0].id, "a", "the first one seen is the one kept");
    assert_eq!(recent[1].id, "c");
}

#[test]
fn the_recent_list_is_bounded() {
    let catalog: Vec<SessionSummary> =
        (0..20).map(|index| summary(&index.to_string(), &format!("session {index}"))).collect();
    assert_eq!(dedupe_recent(&catalog).len(), RECENT_ROWS);
}

#[test]
fn first_user_text_skips_non_user_and_empty_content() {
    let entries = vec![
        entry(EntryKind::Message(Message::assistant("not a title"))),
        entry(EntryKind::Message(Message::user("   "))),
        entry(EntryKind::Message(Message::user("Refine the startup dashboard"))),
    ];
    assert_eq!(first_user_text(&entries).as_deref(), Some("Refine the startup dashboard"));
}

#[test]
fn session_preview_tags_roles_surfaces_tool_calls_and_skips_noise() {
    use luxe_core::message::ContentBlock;

    let assistant_with_tool = Message {
        role: Role::Assistant,
        content: vec![
            ContentBlock::Text("Reading the file now.".into()),
            ContentBlock::ToolCall {
                id: "call-1".into(),
                name: "read".into(),
                arguments: serde_json::json!({ "path": "src/lib.rs" }),
            },
        ],
    };
    let entries = vec![
        entry(EntryKind::Message(Message {
            role: Role::System,
            content: vec![ContentBlock::Text("you are a helpful agent".into())],
        })),
        entry(EntryKind::Message(Message::user("Explain the layout."))),
        entry(EntryKind::Message(assistant_with_tool)),
        entry(EntryKind::Message(Message::tool_result("call-1", "file body", false))),
    ];

    let preview = session_preview(&entries);
    let roles: Vec<PreviewRole> = preview.iter().map(|turn| turn.role).collect();
    // System prompt and tool-result are dropped; user text, assistant text, and the tool
    // call survive in conversation order.
    assert_eq!(roles, vec![PreviewRole::User, PreviewRole::Assistant, PreviewRole::Tool]);
    assert_eq!(preview[0].text, "Explain the layout.");
    assert_eq!(preview[1].text, "Reading the file now.");
    assert_eq!(preview[2].text, "read(path: src/lib.rs)");
}

#[test]
fn session_preview_is_bounded_by_turn_count() {
    let mut entries = Vec::new();
    for index in 0..(PREVIEW_MAX_TURNS + 20) {
        entries.push(entry(EntryKind::Message(Message::user(format!("message {index}")))));
    }
    assert_eq!(session_preview(&entries).len(), PREVIEW_MAX_TURNS);
}

fn user(text: String) -> Entry {
    entry(EntryKind::Message(Message::user(text)))
}

fn preview_says(entries: &[Entry], word: &str) -> bool {
    session_preview(entries).iter().any(|turn| turn.text.contains(word))
}

/// The break this catches: a preview that stops short of the text the session search reads, so a
/// session listed for a word can't show where it says it.
#[test]
fn the_preview_reaches_as_far_as_the_search() {
    let mut entries: Vec<Entry> = (0..8).map(|_| user("filler ".repeat(250))).collect();
    entries.push(user("the needle is here".into()));
    assert!(searchable_text(&entries).contains("needle"), "the search reads it");
    assert!(preview_says(&entries, "needle"), "and so does the preview");
}

/// The break this catches: a long message cut to a turn's own cap, losing its end.
#[test]
fn a_long_message_keeps_its_end() {
    let entries = vec![user(format!("{} needle", "filler ".repeat(700)))];
    assert!(preview_says(&entries, "needle"));
}

/// The break this catches: tool calls spending the text budget, or a turn cap a busy session
/// reaches before its text does; both leave the preview short of what the search reads.
#[test]
fn tool_calls_do_not_shorten_the_preview() {
    let call = |index: usize| ContentBlock::ToolCall {
        id: format!("call-{index}"),
        name: "read".into(),
        arguments: serde_json::json!({ "path": "x".repeat(200) }),
    };
    let mut entries: Vec<Entry> = (0..200)
        .map(|index| {
            entry(EntryKind::Message(Message { role: Role::Assistant, content: vec![call(index)] }))
        })
        .collect();
    entries.push(user("the needle is here".into()));
    assert!(preview_says(&entries, "needle"));
}

/// The break this catches: a preview with no bound on its text, which every catalog row holds.
#[test]
fn the_preview_text_stays_within_the_search_budget() {
    let entries: Vec<Entry> = (0..40).map(|_| user("filler ".repeat(300))).collect();
    let preview = session_preview(&entries);
    let text: usize = preview.iter().map(|turn| turn.text.chars().count()).sum();
    assert!(text <= SESSION_SEARCH_TEXT_LIMIT, "{text} characters");
    assert!(text >= SESSION_SEARCH_TEXT_LIMIT / 2, "but uses it: {text} characters");
}

#[test]
fn clamp_chars_truncates_with_ellipsis() {
    assert_eq!(clamp_chars("hello", 10), "hello");
    assert_eq!(clamp_chars("hello world", 5), "hell…");
}

/// The count on the dashboard is the number of files the prompt will actually carry.
///
/// It used to be its own hardcoded pair — the root `AGENTS.md` and the global one — which
/// disagreed with prompt assembly for any repo with nested instructions or a `CLAUDE.md`. Both
/// now come from `instructions::context_files`, so a wrong number needs one bug rather than
/// two implementations drifting.
#[test]
fn context_count_matches_prompt_inputs() {
    let repo = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(repo.path().join(".git")).unwrap();
    std::fs::write(repo.path().join("AGENTS.md"), "rules").unwrap();
    // A repo plan document is not an input to the prompt, so the dashboard must not count it.
    std::fs::write(repo.path().join("PROJECT.md"), "plan").unwrap();

    let counted: Vec<_> = luxe_core::instructions::context_files(repo.path())
        .into_iter()
        .map(|file| file.path)
        .collect();
    assert!(counted.contains(&repo.path().join("AGENTS.md")));
    assert!(!counted.contains(&repo.path().join("PROJECT.md")));
}

/// A session that worked on a pull request says so on its row, and typing the number finds it.
#[test]
fn a_linked_session_shows_and_finds_its_pull_requests() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = luxe_core::session::JsonlSessionStore::new(dir.path());
    let id = store.create("/work").unwrap();
    let said = store
        .append(
            &id,
            None,
            EntryKind::Message(Message {
                role: luxe_core::message::Role::User,
                content: vec![ContentBlock::Text("fix the loop".into())],
            }),
        )
        .unwrap();
    for number in [146, 7, 146] {
        store
            .append(
                &id,
                Some(said.clone()),
                EntryKind::PullRequestLink { number, url: format!("https://x/pull/{number}") },
            )
            .unwrap();
    }
    let (catalog, _) = collect_session_catalog(&store);
    assert_eq!(catalog[0].name, "fix the loop · #146 #7");
    assert!(
        catalog[0].search_text.starts_with("#146 #7 fix the loop"),
        "{}",
        catalog[0].search_text
    );
}

/// A session that works in a worktree of its own says which, so the start screen can tell it has
/// to move there before resuming it rather than loading it in the checkout it is standing in.
#[test]
fn a_session_in_a_worktree_says_which() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = luxe_core::session::JsonlSessionStore::new(dir.path());
    let id = store.create("/work").unwrap();
    let said = store.append(&id, None, EntryKind::Message(Message::user("fix the loop"))).unwrap();
    let worktree =
        EntryKind::Worktree { branch: "topic".into(), path: ".luxe/worktrees/topic".into() };
    store.append(&id, Some(said), worktree).unwrap();
    let plain = store.create("/work").unwrap();
    store.append(&plain, None, EntryKind::Message(Message::user("tidy the dock"))).unwrap();

    let (catalog, _) = collect_session_catalog(&store);
    let worktree_of = |id: &luxe_core::session::SessionId| {
        catalog.iter().find(|session| session.id == id.0).map(|session| session.worktree.clone())
    };
    assert_eq!(worktree_of(&id), Some(Some("topic".into())));
    assert_eq!(worktree_of(&plain), Some(None), "one in the main checkout says so too");
}
