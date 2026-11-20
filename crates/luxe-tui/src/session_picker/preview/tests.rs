use super::*;
use crate::dashboard::PreviewRole;

fn theme() -> ThemeStyles {
    let resolved = luxe_core::theme::ThemeRegistry::new().resolve("nord").expect("nord");
    ThemeStyles::from_resolved(&resolved, "nord")
}

fn turn(role: PreviewRole, text: &str) -> PreviewTurn {
    PreviewTurn { role, text: text.into() }
}

fn session(preview: Vec<PreviewTurn>, search_text: &str) -> SessionEntry {
    SessionEntry {
        id: "s1".into(),
        name: "Fix palette navigation".into(),
        age: "2h ago".into(),
        search_text: search_text.into(),
        preview,
    }
}

fn text(line: &Line<'static>) -> String {
    line.spans.iter().map(|span| span.content.as_ref()).collect()
}

fn texts(lines: &[Line<'static>]) -> Vec<String> {
    lines.iter().map(text).collect()
}

/// A long talk with the word `needle` only in its last reply.
fn long_session() -> SessionEntry {
    let mut preview: Vec<PreviewTurn> = (0..12)
        .flat_map(|index| {
            [
                turn(PreviewRole::User, &format!("question {index}")),
                turn(PreviewRole::Assistant, &format!("answer {index}")),
            ]
        })
        .collect();
    preview.push(turn(PreviewRole::Assistant, "the needle was in the config"));
    session(preview, "the needle was in the config")
}

/// The break this catches: the preview drawing a user's turn its own way (a role header and
/// plain text) instead of on the user band the transcript uses.
#[test]
fn a_users_turn_is_drawn_on_the_transcript_band() {
    let theme = theme();
    let entry = session(vec![turn(PreviewRole::User, "why do arrows skip")], "");
    let lines = preview_lines(Some(&entry), "", 60, 20, &theme);
    let user = lines.iter().find(|line| text(line).contains("why do arrows skip")).expect("drawn");
    assert_eq!(user.style.bg, theme.user_msg_bg.bg);
}

/// The break this catches: a preview that always opens at the session's start, so a match deep
/// in it sits below the pane's edge where the reader can't see why the session matched.
#[test]
fn the_preview_scrolls_to_the_first_match() {
    let lines = preview_lines(Some(&long_session()), "needle", 60, 12, &theme());
    let shown = texts(&lines);
    assert!(shown.iter().any(|line| line.contains("needle")), "{shown:?}");
    assert!(lines.len() <= 12, "it fits the pane: {}", lines.len());
}

/// The break this catches: scrolling with no query, which would hide the session's start for
/// nothing.
#[test]
fn without_a_query_the_preview_opens_at_the_start() {
    let shown = texts(&preview_lines(Some(&long_session()), "", 60, 12, &theme()));
    assert!(shown.iter().any(|line| line.contains("question 0")), "{shown:?}");
}

/// The break this catches: the match shown but not lit, so the reader hunts for it in the turn.
#[test]
fn the_query_is_lit_where_it_shows() {
    let theme = theme();
    let lines = preview_lines(Some(&long_session()), "needle", 60, 12, &theme);
    let lit: Vec<&str> = lines
        .iter()
        .flat_map(|line| &line.spans)
        .filter(|span| span.style.bg == theme.match_bg().bg)
        .map(|span| span.content.as_ref())
        .collect();
    assert_eq!(lit, ["needle"]);
}

/// The break this catches: a session matched past the turns the catalog keeps, previewed with
/// no word about it, so it reads as a wrong match.
#[test]
fn a_match_past_the_kept_turns_says_so() {
    let entry = session(vec![turn(PreviewRole::User, "start")], "start and later a needle");
    let shown = texts(&preview_lines(Some(&entry), "needle", 60, 12, &theme()));
    assert!(shown.iter().any(|line| line.contains("later in this session")), "{shown:?}");
}

/// The break this catches: the note claiming a later match for a fuzzy hit, whose letters only
/// line up spread out, when nothing later says it either.
#[test]
fn a_fuzzy_hit_claims_no_later_match() {
    let entry = session(vec![turn(PreviewRole::User, "start")], "start");
    let shown = texts(&preview_lines(Some(&entry), "srt", 60, 12, &theme()));
    assert!(!shown.iter().any(|line| line.contains("later in this session")), "{shown:?}");
}

/// A 400-turn session, the most the catalog keeps, with `needle` in turn 300.
fn huge_session() -> SessionEntry {
    let mut preview: Vec<PreviewTurn> =
        (0..400).map(|index| turn(PreviewRole::Assistant, &format!("answer {index}"))).collect();
    preview[300] = turn(PreviewRole::User, "the needle");
    session(preview, "the needle")
}

/// The break this catches: drawing every turn of a long session on each frame, to show a pane's
/// worth of it around the match.
#[test]
fn only_the_turns_around_a_match_are_drawn() {
    let matcher = matcher("needle");
    let lines = body_lines(&huge_session(), matcher.as_ref(), 60, 12, &theme());
    assert!(lines.len() <= 40, "{} lines", lines.len());
    assert!(texts(&lines).iter().any(|line| line.contains("needle")));
}

/// The break this catches: the same, without a query, where the pane shows the session's start.
#[test]
fn only_the_opening_turns_are_drawn_without_a_query() {
    let lines = body_lines(&huge_session(), None, 60, 12, &theme());
    assert!(lines.len() <= 40, "{} lines", lines.len());
    assert!(texts(&lines).iter().any(|line| line.contains("answer 0")));
}

/// The break this catches: a long tool line cut a cell too late, running into the frame.
#[test]
fn a_long_tool_line_fits_the_pane() {
    let call = format!("read(path: \"{}\")", "x".repeat(80));
    let entry = session(vec![turn(PreviewRole::Tool, &call)], "");
    let lines = preview_lines(Some(&entry), "", 40, 12, &theme());
    let tool = lines.iter().find(|line| text(line).contains("read(")).expect("the tool line");
    assert_eq!(tool.width(), 40, "{:?}", text(tool));
}

/// The break this catches: a session saved without turns showing an empty pane, when its
/// search text could say what it was about.
#[test]
fn a_session_without_turns_shows_its_text_wrapped() {
    let entry = session(Vec::new(), "one two three four five six seven eight nine ten");
    let lines = preview_lines(Some(&entry), "", 20, 12, &theme());
    let body = texts(&lines);
    assert!(body.len() > 1, "wrapped to the pane: {body:?}");
    assert!(body.iter().all(|line| line.chars().count() <= 20), "{body:?}");
}
