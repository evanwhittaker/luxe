//! Review annotations — the comment and thread cards drawn under the lines they are anchored to.

use super::super::chord;
use crate::app::App;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// Rows one comment may spend on its body before it is clipped.
const MAX_COMMENT_BODY_ROWS: usize = 3;

/// Rows the comment under the cursor may spend on its words, and again on the answer, before the
/// feedback window has to take over. Enough for a full answer to a question, short of one that
/// would push the line it is about off the screen.
const OPEN_COMMENT_ROWS: usize = 24;

pub(in crate::view::guided_review) use crate::guided_review::line_anchor_matches;

/// The comments whose anchor `matches`.
pub(super) fn findings_at(
    app: &App,
    matches: impl Fn(&luxe_core::guided_review::FindingAnchor) -> bool,
) -> Vec<luxe_core::guided_review::ReviewFinding> {
    let Some(session) = app.review.session() else {
        return Vec::new();
    };
    session.findings.iter().filter(|finding| matches(&finding.anchor)).cloned().collect()
}

/// One comment as rows under the line it is anchored to.
///
/// The whole point of rendering it here: under the code it is about, reading your own words needs
/// neither a drawer over the far corner nor an overlay on top of that. The cards on the cursor's
/// line are `open`: they show the whole answer and the keys that act on them, where the rest stay
/// a few rows each so the code between them keeps its place.
pub(in crate::view::guided_review) fn comment_lines(
    app: &App,
    findings: &[luxe_core::guided_review::ReviewFinding],
    width: usize,
    open: bool,
) -> Vec<Line<'static>> {
    let styles = &app.theme_styles;
    let mut lines = Vec::new();
    for finding in findings {
        // Only a question is worth naming. Everything else a reviewer writes is a comment, and a
        // row that labeled every one of them "issue" spent a word saying what the surface is.
        let question = finding.severity == luxe_core::guided_review::FindingSeverity::Question;
        let tone = if question { styles.info } else { styles.error };
        let severity = if question { "question" } else { "" };
        let (state, state_style) = comment_state_label(app, finding, question);
        let mut title = Vec::new();
        if !severity.is_empty() {
            title.push(Span::styled(format!("{severity}   "), tone));
        }
        title.push(Span::styled(state, state_style));
        let inner = card_inner_width(width);
        let mut body = comment_body_rows(app, finding, inner, open);
        body.extend(comment_reply_rows(app, finding, inner, open));
        if open {
            body.extend(comment_key_row(app, finding));
        }
        let fill = if question { styles.card_info_bg } else { styles.card_error_bg };
        lines.extend(card_lines(tone, fill, title, body, width));
    }
    lines
}

/// What a comment is and what happens to it next, and the ink to say it in.
///
/// The second half is what a comment sitting in a drawer never told anyone.
fn comment_state_label(
    app: &App,
    finding: &luxe_core::guided_review::ReviewFinding,
    question: bool,
) -> (String, ratatui::style::Style) {
    use luxe_core::guided_review::FindingState;
    let styles = &app.theme_styles;
    let pull_request = crate::app::github_post_is_pull_request(app);
    // Bound for GitHub rather than the agent: someone else's pull request, and not a question
    let for_github = pull_request && !question;
    let answered = finding.github_thread.as_ref().filter(|thread| thread.resolved);
    match finding.state {
        _ if answered.is_some() => ("resolved on GitHub".to_owned(), styles.success),
        FindingState::Draft | FindingState::Open if finding.posted.is_some() => {
            ("on GitHub \u{00b7} o opens it".to_owned(), styles.dim)
        }
        FindingState::Draft if for_github => (
            format!(
                "{} to post",
                chord(app, luxe_core::keymap::UiAction::ReviewFinish, "alt+enter")
            ),
            styles.warning,
        ),
        FindingState::Draft => ("not sent".to_owned(), styles.warning),
        // Spun here, on the card the reader just wrote, rather than only in the dock across
        // the screen
        FindingState::Open if being_worked(app, &finding.id) => {
            let spinner = crate::statusline::spinner(app.guided_review_workspace.frame);
            let doing = if question { "answering" } else { "working on it" };
            (format!("{spinner} the agent is {doing}\u{2026}"), styles.accent)
        }
        // The answer is in, under it; following up, or asking for the fix, is the window's
        FindingState::Open
            if question
                && finding.thread.last().is_some_and(|message| {
                    message.author == luxe_core::guided_review::FindingMessageAuthor::Agent
                }) =>
        {
            (
                format!(
                    "answered \u{00b7} {} follows up",
                    chord(app, luxe_core::keymap::UiAction::ReviewFindings, "alt+c")
                ),
                styles.success,
            )
        }
        // The fix turn ended with no word on this one, so the agent is not still at it
        FindingState::Open
            if finding.thread.last().is_some_and(|message| {
                message.author == luxe_core::guided_review::FindingMessageAuthor::System
            }) =>
        {
            (
                format!(
                    "no word from the agent \u{00b7} {} says why",
                    chord(app, luxe_core::keymap::UiAction::ReviewFindings, "alt+c")
                ),
                styles.warning,
            )
        }
        // Fixed, not fixed, or a question back for the reader: the agent has had its say
        FindingState::Open
            if !app.review.session().is_some_and(|session| {
                super::super::context::with_the_agent(session, &finding.id)
            }) && finding.thread.last().is_some_and(|message| {
                message.author == luxe_core::guided_review::FindingMessageAuthor::Agent
            }) =>
        {
            (
                format!(
                    "the agent replied \u{00b7} {} reads it",
                    chord(app, luxe_core::keymap::UiAction::ReviewFindings, "alt+c")
                ),
                styles.info,
            )
        }
        FindingState::Open => ("sent \u{00b7} waiting on the agent".to_owned(), styles.dim),
        FindingState::Resolved => ("resolved".to_owned(), styles.success),
        FindingState::Stale => ("outdated".to_owned(), styles.dim),
    }
}

/// The comment's own words, wrapped to the card and capped.
fn comment_body_rows(
    app: &App,
    finding: &luxe_core::guided_review::ReviewFinding,
    inner: usize,
    open: bool,
) -> Vec<Line<'static>> {
    let words = Line::from(Span::styled(finding.body.clone(), app.theme_styles.text));
    capped_rows(app, crate::widgets::wrap_line(&words, inner), open)
}

/// The last word that was not the reader's, under the comment: the answer to a question belongs
/// beside the question, not behind two overlays, and so does luxe saying the agent left none.
fn comment_reply_rows(
    app: &App,
    finding: &luxe_core::guided_review::ReviewFinding,
    inner: usize,
    open: bool,
) -> Vec<Line<'static>> {
    use luxe_core::guided_review::FindingMessageAuthor;
    let styles = &app.theme_styles;
    let reply = finding
        .thread
        .iter()
        .rev()
        .find(|message| message.author != FindingMessageAuthor::Reviewer);
    let Some(reply) = reply else { return Vec::new() };
    let who = match reply.author {
        FindingMessageAuthor::System => "luxe",
        _ => "agent",
    };
    if open {
        let mut rows = vec![Line::from(Span::styled(format!("{who}:"), styles.muted))];
        rows.extend(whole_reply_rows(&reply.body, styles, inner));
        return capped_rows(app, rows, open);
    }
    let said = first_paragraph(&reply.body, styles);
    let opening = Line::from(Span::styled(format!("{who}: {said}"), styles.muted));
    capped_rows(app, crate::widgets::wrap_line(&opening, inner), open)
}

/// A reply rendered whole, as the feedback window shows it, wrapped to the card.
fn whole_reply_rows(
    reply: &str,
    styles: &crate::theme::styles::ThemeStyles,
    inner: usize,
) -> Vec<Line<'static>> {
    let blank = |line: &Line| line.spans.iter().all(|span| span.content.trim().is_empty());
    let mut rendered = crate::markdown::render_markdown(reply, styles);
    while rendered.last().is_some_and(blank) {
        rendered.pop();
    }
    let start = rendered.iter().position(|line| !blank(line)).unwrap_or(rendered.len());
    rendered[start..].iter().flat_map(|line| crate::widgets::wrap_line(line, inner)).collect()
}

/// `rows` cut to what a card may show, saying how many more there are and where to read them.
fn capped_rows(app: &App, mut rows: Vec<Line<'static>>, open: bool) -> Vec<Line<'static>> {
    let cap = if open { OPEN_COMMENT_ROWS } else { MAX_COMMENT_BODY_ROWS };
    let Some(hidden) = rows.len().checked_sub(cap).filter(|hidden| *hidden > 0) else {
        return rows;
    };
    rows.truncate(cap);
    let rest = match open {
        true => format!(
            "{} reads the rest",
            chord(app, luxe_core::keymap::UiAction::ReviewFindings, "alt+c")
        ),
        false => "move to this line to read them".to_owned(),
    };
    let noun = if hidden == 1 { "line" } else { "lines" };
    let note = format!("+{hidden} more {noun} \u{b7} {rest}");
    rows.push(Line::from(Span::styled(note, app.theme_styles.dim)));
    rows
}

/// The keys that act on the comment under the cursor, for an unsent one: every other state names
/// its next step in the card's title already.
fn comment_key_row(
    app: &App,
    finding: &luxe_core::guided_review::ReviewFinding,
) -> Option<Line<'static>> {
    use luxe_core::keymap::UiAction;
    if finding.state != luxe_core::guided_review::FindingState::Draft || finding.posted.is_some() {
        return None;
    }
    let keys = format!(
        "{} edit \u{b7} {} delete \u{b7} right-click for more",
        chord(app, UiAction::ReviewEditFinding, "e"),
        chord(app, UiAction::ReviewDeleteFinding, "x"),
    );
    Some(Line::from(Span::styled(keys, app.theme_styles.dim)))
}

/// Whether a submission carrying `finding` is with the agent right now.
fn being_worked(app: &App, finding: &luxe_core::guided_review::FindingId) -> bool {
    use luxe_core::guided_review::SubmissionDispatch;
    app.review.session().is_some_and(|session| {
        session.submissions.iter().any(|submission| {
            submission.dispatch == SubmissionDispatch::Dispatched
                && submission.findings.iter().any(|sent| &sent.finding_id == finding)
        })
    })
}

/// The opening paragraph of a reply, as it reads rendered: a closed card has three rows for it,
/// and markup shown raw spends them on asterisks and fences. The open card has the rest.
pub(super) fn first_paragraph(reply: &str, styles: &crate::theme::styles::ThemeStyles) -> String {
    let text =
        |line: &Line| line.spans.iter().map(|span| span.content.as_ref()).collect::<String>();
    crate::markdown::render_markdown(reply, styles)
        .iter()
        .map(text)
        .skip_while(|line| line.trim().is_empty())
        .take_while(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether a thread is already feedback: on your own pull request, an unresolved thread becomes a
/// finding, and drawing it a second time as earlier conversation would say everything twice.
pub(in crate::view::guided_review) fn thread_is_feedback(
    app: &App,
    thread: &luxe_core::guided_review::EarlierThread,
) -> bool {
    is_feedback(app, &thread.id)
}

/// Whether the thread or conversation comment GitHub calls `id` became a finding.
pub(in crate::view::guided_review) fn is_feedback(app: &App, id: &str) -> bool {
    !id.is_empty()
        && app.review.session().is_some_and(|session| {
            session
                .findings
                .iter()
                .any(|finding| finding.github_thread.as_ref().is_some_and(|origin| origin.id == id))
        })
}

/// The earlier review threads that sit under `line` of the file `file_id`, on either side.
pub(in crate::view::guided_review) fn threads_at<'a>(
    app: &'a App,
    file_id: &luxe_core::guided_review::FileId,
    line: &luxe_core::guided_review::ReviewLine,
) -> Vec<&'a luxe_core::guided_review::EarlierThread> {
    use luxe_core::guided_review::DiffSide;
    let Some(conversation) =
        app.guided_review.as_ref().and_then(|review| review.conversation.as_ref())
    else {
        return Vec::new();
    };
    conversation
        .threads()
        .iter()
        .filter(|thread| !thread_is_feedback(app, thread))
        .filter(|thread| {
            line.new_line.is_some_and(|number| thread.sits_at(file_id, DiffSide::New, number))
                || line
                    .old_line
                    .is_some_and(|number| thread.sits_at(file_id, DiffSide::Old, number))
        })
        .collect()
}

/// A reviewer's thread as rows under the line it is about.
///
/// A lighter rule than a comment of your own, because it is someone else's and nothing about it is
/// waiting on you: who opened it, whether it was resolved, what they said, and the latest reply.
/// The rest of a long thread is one `o` away on GitHub.
pub(in crate::view::guided_review) fn thread_lines(
    app: &App,
    threads: &[&luxe_core::guided_review::EarlierThread],
    width: usize,
) -> Vec<Line<'static>> {
    let styles = &app.theme_styles;
    let mut lines = Vec::new();
    for thread in threads {
        let tone = if thread.resolved { styles.dim } else { styles.info };
        let fill = if thread.resolved { styles.user_msg_bg } else { styles.card_info_bg };
        let Some(opening) = thread.comments.first() else { continue };
        let state = if thread.resolved { "resolved" } else { "unresolved" };
        let replies = thread.total_comments.saturating_sub(1);
        let mut title = vec![
            Span::styled(display_author(&opening.author), styles.accent_secondary),
            Span::styled(" \u{b7} ", styles.dim),
            Span::styled(state, tone),
        ];
        if replies > 0 {
            let noun = if replies == 1 { "reply" } else { "replies" };
            title.push(Span::styled(format!(" \u{b7} {replies} {noun}"), styles.dim));
        }
        let inner = card_inner_width(width);
        let mut body = Vec::new();
        let mut say = |text: String, style: Style, rows: usize| {
            body.extend(
                crate::widgets::wrap_line(&Line::from(Span::styled(text, style)), inner)
                    .into_iter()
                    .take(rows),
            );
        };
        say(opening.body.clone(), styles.text, MAX_COMMENT_BODY_ROWS);
        if let Some(latest) = thread.comments.iter().skip(1).last() {
            say(format!("{}: {}", display_author(&latest.author), latest.body), styles.muted, 2);
        }
        let unshown = thread.total_comments.saturating_sub(thread.comments.len());
        if unshown > 0 {
            say(format!("{unshown} more on GitHub \u{b7} o"), styles.dim, 1);
        }
        lines.extend(card_lines(tone, fill, title, body, width));
    }
    lines
}

/// Cells between the diff's margin and a card's left edge.
const CARD_INDENT: usize = 2;

fn card_inner_width(width: usize) -> usize {
    crate::widgets::card_inner_width(width, CARD_INDENT)
}

fn card_lines(
    tone: Style,
    fill: Style,
    title: Vec<Span<'static>>,
    body: Vec<Line<'static>>,
    width: usize,
) -> Vec<Line<'static>> {
    crate::widgets::card(tone, fill, title, body, width, CARD_INDENT)
}

/// A login the way GitHub writes one, `@mara`; a deleted account has none.
///
/// The `@` is what says it is a GitHub account and not a name the review made up, which a bare
/// `mara` beside a file path did not.
pub(in crate::view::guided_review) fn display_author(login: &str) -> String {
    match login.strip_prefix("app/").unwrap_or(login) {
        "" => "a deleted account".to_owned(),
        name => format!("@{name}"),
    }
}
