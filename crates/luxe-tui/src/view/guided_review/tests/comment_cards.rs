//! Comment cards under the diff: closed ones stay short, the one on the cursor's line opens.

use crate::fixture::{seeded_app, FixtureScenario};
use luxe_core::guided_review::{
    FindingMessage, FindingMessageAuthor, FindingSeverity, FindingState, ReviewFinding,
};

/// Three paragraphs, so a card that kept only the opening one can be told apart from a whole one.
const LONG_ANSWER: &str = "It is a bug: the router repeats the scheduler's decision.\n\n\
    The second paragraph says where it happens.\n\n\
    The third paragraph says how to fix it.";

fn answered_question() -> (crate::app::App, ReviewFinding) {
    let mut app = seeded_app("nord", FixtureScenario::GuidedReviewFindings).expect("fixture app");
    let session = app.review.session_mut().expect("review session");
    let question = &mut session.findings[0];
    question.severity = FindingSeverity::Question;
    question.state = FindingState::Open;
    question.thread.push(FindingMessage {
        author: FindingMessageAuthor::Agent,
        body: LONG_ANSWER.into(),
        created_at_ms: 1,
    });
    let finding = question.clone();
    (app, finding)
}

fn card_text(app: &crate::app::App, finding: &ReviewFinding, open: bool) -> String {
    super::diff_render::comment_lines(app, std::slice::from_ref(finding), 80, open)
        .iter()
        .map(|line| line.spans.iter().map(|span| span.content.as_ref()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The break this catches: the card under the cursor still showing only the answer's first
/// paragraph, so the rest of what the agent said is behind the feedback window.
#[test]
fn the_card_on_the_cursor_line_shows_the_whole_answer() {
    let (app, finding) = answered_question();
    let open = card_text(&app, &finding, true);
    assert!(open.contains("The third paragraph says how to fix it."), "{open}");
    assert!(!open.contains("more line"), "nothing is left out: {open}");
}

/// The break this catches: every card opening to its whole answer, which spreads the code apart,
/// or a closed card clipping the answer without saying there is more.
#[test]
fn a_closed_card_keeps_the_opening_and_says_how_to_read_the_rest() {
    let (mut app, mut finding) = answered_question();
    let closed = card_text(&app, &finding, false);
    assert!(closed.contains("It is a bug"), "{closed}");
    assert!(!closed.contains("second paragraph"), "{closed}");

    let session = app.review.session_mut().expect("review session");
    finding.thread[0].body = "word ".repeat(200);
    session.findings[0] = finding.clone();
    let clipped = card_text(&app, &finding, false);
    assert!(clipped.contains("more lines · move to this line to read them"), "{clipped}");
}

/// The break this catches: an open card growing without limit, pushing the line it is about off
/// the screen, or clipping without pointing at the window that holds the rest.
#[test]
fn an_open_card_caps_a_very_long_answer_and_points_at_the_window() {
    let (mut app, mut finding) = answered_question();
    finding.thread[0].body = (1..=60).map(|n| format!("Paragraph {n}.\n\n")).collect();
    app.review.session_mut().expect("review session").findings[0] = finding.clone();
    let open = card_text(&app, &finding, true);
    assert!(open.contains("more lines · alt+c reads the rest"), "{open}");
    assert!(!open.contains("Paragraph 60."), "{open}");
}

/// The break this catches: an unsent comment's card giving no hint that `e` and `x` act on it,
/// or a card that is not under the cursor carrying the hint too.
#[test]
fn an_unsent_comment_under_the_cursor_names_its_keys() {
    let app = seeded_app("nord", FixtureScenario::GuidedReviewFindings).expect("fixture app");
    let session = app.review.session().expect("review session");
    let draft = session.findings.iter().find(|finding| finding.state == FindingState::Draft);
    let draft = draft.expect("the fixture has a draft").clone();
    let hint = "e edit · x delete · right-click for more";
    assert!(card_text(&app, &draft, true).contains(hint));
    assert!(!card_text(&app, &draft, false).contains(hint));

    let sent = session.findings.iter().find(|finding| finding.state == FindingState::Open);
    let sent = sent.expect("the fixture has a sent comment").clone();
    assert!(!card_text(&app, &sent, true).contains("x delete"), "a sent comment cannot be");
}
