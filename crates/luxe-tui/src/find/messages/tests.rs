use super::*;

fn item(index: usize, block: ConvoBlock) -> ConversationItem {
    let id =
        ConversationItemId::Provisional { session_id: "s".into(), turn_id: 1, block_index: index };
    ConversationItem::new(id, block)
}

fn user(text: &str) -> ConvoBlock {
    ConvoBlock::User { text: text.into(), attached: Vec::new(), chips: Vec::new() }
}

fn conversation() -> Vec<ConversationItem> {
    vec![
        item(0, user("why does the build panic?")),
        item(1, ConvoBlock::Assistant("Reading the log.\n  The PANIC comes from `parse`.".into())),
        item(2, ConvoBlock::System("panic in system notes".into())),
        item(3, ConvoBlock::Assistant("Fixed it.".into())),
    ]
}

fn block_index(hit: &MessageHit) -> usize {
    match &hit.item {
        ConversationItemId::Provisional { block_index, .. } => *block_index,
        other => panic!("{other:?}"),
    }
}

/// The break this catches: a message left out for its case, a system note listed as if someone
/// said it, or the rows in transcript order when the newest is what's wanted first.
#[test]
fn messages_saying_the_query_are_listed_newest_first() {
    let hits = message_hits(&conversation(), "panic");
    let order: Vec<usize> = hits.iter().map(block_index).collect();
    assert_eq!(order, [1, 0]);
    assert_eq!((hits[0].role, hits[1].role), ("Assistant", "You"));
}

/// The break this catches: the row showing a message's first line rather than the one that says
/// the query, or its indent kept, or the lit characters counted from the untrimmed line.
#[test]
fn a_row_shows_the_line_that_matched_with_its_match_marked() {
    let hits = message_hits(&conversation(), "panic");
    assert_eq!(hits[0].line, "The PANIC comes from `parse`.");
    assert_eq!(hits[0].matched, [4, 5, 6, 7, 8]);
}

/// The break this catches: a capital in the query making it match case, as Text's smart case
/// does, so the tab lists less than the conversation's own search then finds.
#[test]
fn a_capital_still_matches_any_case() {
    let hits = message_hits(&conversation(), "PANIC");
    assert_eq!(hits.len(), 2, "the user's lowercase `panic` too");
}

/// The break this catches: an empty query listing every message as a match.
#[test]
fn an_empty_query_lists_nothing() {
    assert!(message_hits(&conversation(), "").is_empty());
    assert!(message_hits(&conversation(), "  ").is_empty());
}

/// The break this catches: a regex character in the query read as a pattern, so `(` matches
/// nothing or `.` everything, unlike the conversation's own search it hands over to.
#[test]
fn the_query_is_plain_text() {
    let items = vec![item(0, user("call f(x) here")), item(1, user("no parens"))];
    let hits = message_hits(&items, "f(x)");
    assert_eq!(hits.len(), 1);
    assert!(message_hits(&items, ".").is_empty());
}

/// The break this catches: a long conversation listing without bound.
#[test]
fn the_rows_are_capped() {
    let items: Vec<ConversationItem> =
        (0..MAX_MESSAGE_HITS + 5).map(|index| item(index, user("same words"))).collect();
    assert_eq!(message_hits(&items, "words").len(), MAX_MESSAGE_HITS);
}

fn theme() -> ThemeStyles {
    let resolved = luxe_core::theme::ThemeRegistry::new().resolve("nord").expect("nord");
    ThemeStyles::from_resolved(&resolved, "nord")
}

/// The break this catches: the pane showing the message's top while the matching line sits
/// below its edge, or showing the match unlit.
#[test]
fn the_preview_scrolls_to_the_match_and_lights_it() {
    let theme = theme();
    let long: String = (0..40).map(|index| format!("line {index}\n\n")).collect();
    let items = vec![item(0, ConvoBlock::Assistant(format!("{long}the needle line")))];
    let hits = message_hits(&items, "needle");
    let lines = message_preview(&hits[0], "needle", 60, 10, &theme);
    assert!(lines.len() <= 10);
    let lit: Vec<&str> = lines
        .iter()
        .flat_map(|line| &line.spans)
        .filter(|span| span.style.bg == theme.match_bg().bg)
        .map(|span| span.content.as_ref())
        .collect();
    assert_eq!(lit, ["needle"]);
}
