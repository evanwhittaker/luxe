//! Brief pull request — the facts about the pull request, what it closes, and the conversation
//! before this review.

use super::super::diff_render::{display_author, is_feedback, thread_is_feedback};

use super::bullet_lines;
use crate::app::App;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// One row of facts about a pull request: who opened it, and where it stands.
///
/// Drawn in the pane's fixed head under the row that names the pull request, so it is on screen
/// however far the guide has scrolled. Every fact is a word, colored only for emphasis: `draft`,
/// `2 failing`, `changes requested`.
pub(in crate::view::guided_review) fn pull_request_facts(
    app: &App,
    pr: &luxe_core::guided_review::PullRequestContext,
    width: usize,
) -> Vec<Line<'static>> {
    use luxe_core::guided_review::{PullRequestStatus, PullRequestVerdict};
    let styles = &app.theme_styles;
    let separator = || Span::styled(" \u{b7} ", styles.dim);
    let mut spans = vec![Span::styled(
        match pr.author_is_bot {
            true => format!("by {} (bot)", pr.author_name()),
            false => format!("by {}", pr.author_name()),
        },
        styles.text,
    )];
    let mut fact = |text: String, style: Style| {
        spans.push(separator());
        spans.push(Span::styled(text, style));
    };
    if pr.draft {
        fact("draft".into(), styles.warning);
    }
    match pr.status {
        PullRequestStatus::Open => {}
        PullRequestStatus::Merged => fact("merged".into(), styles.success),
        PullRequestStatus::Closed => fact("closed".into(), styles.dim),
    }
    let checks = pr.checks;
    if checks.passed + checks.failed + checks.pending + checks.skipped > 0 {
        let mut parts = Vec::new();
        if checks.failed > 0 {
            parts.push((format!("{} failing", checks.failed), styles.error));
        }
        if checks.pending > 0 {
            parts.push((format!("{} running", checks.pending), styles.warning));
        }
        if checks.passed > 0 {
            let noun = if checks.passed == 1 { "check" } else { "checks" };
            parts.push((format!("{} {noun} passed", checks.passed), styles.dim));
        }
        for (text, style) in parts {
            fact(text, style);
        }
    }
    match pr.verdict {
        Some(PullRequestVerdict::Approved) => fact("approved".into(), styles.success),
        Some(PullRequestVerdict::ChangesRequested) => {
            fact("changes requested".into(), styles.warning)
        }
        Some(PullRequestVerdict::ReviewRequired) => fact("review required".into(), styles.dim),
        None => {}
    }
    let mut lines = crate::widgets::wrap_line(&Line::from(spans), width.max(20));
    lines.extend(branch_lines(app, pr, width));
    lines
}

/// Where the pull request goes, and the key that reads its description over the review. The head
/// is where that stays on screen, now the guide no longer repeats the description.
fn branch_lines(
    app: &App,
    pr: &luxe_core::guided_review::PullRequestContext,
    width: usize,
) -> Vec<Line<'static>> {
    let styles = &app.theme_styles;
    let mut spans =
        vec![Span::styled(format!("{} \u{2192} {}", pr.head_ref, pr.base_ref), styles.dim)];
    if !pr.body.trim().is_empty() {
        spans.push(Span::styled(" \u{b7} ", styles.dim));
        spans.push(Span::styled("p description", styles.dim));
    }
    crate::widgets::wrap_line(&Line::from(spans), width.max(20))
}

/// The issues the pull request says it closes. Its title and description are not repeated here:
/// GitHub's view of it and `p` show them whole.
pub(super) fn closing_issue_lines(
    app: &App,
    pr: &luxe_core::guided_review::PullRequestContext,
    width: usize,
) -> Vec<Line<'static>> {
    let styles = &app.theme_styles;
    if pr.closing_issues.is_empty() {
        return Vec::new();
    }
    let mut lines =
        vec![Line::default(), Line::from(Span::styled("What it closes", styles.accent))];
    for issue in &pr.closing_issues {
        let reference = if issue.repository.eq_ignore_ascii_case(&pr.repository) {
            format!("#{}", issue.number)
        } else {
            format!("{}#{}", issue.repository, issue.number)
        };
        let title = issue.title.as_deref().unwrap_or("could not be read");
        lines.extend(bullet_lines(&format!("{reference} {title}"), app, width));
    }
    if pr.more_closing_issues > 0 {
        let more = format!("   and {} more", pr.more_closing_issues);
        lines.push(Line::from(Span::styled(more, styles.dim)));
    }
    lines
}

/// What reviewers already said: a count, then every thread that has no line to sit on.
///
/// Two kinds of thing live here, and GitHub keeps them apart the same way: a *thread* is a review
/// comment on the code, with replies and a resolved state; a *comment* is a post in the pull
/// request's conversation, on no line at all. The threads that do map are drawn under their lines
/// in the diff, so this lists only the rest — outdated, on a whole file, or on a line no hunk shows
/// — and never drops one. A conversation that could not be read says so, rather than looking like
/// one nobody started.
pub(super) fn earlier_conversation_lines(
    app: &App,
    conversation: &luxe_core::guided_review::EarlierConversation,
    width: usize,
) -> Vec<Line<'static>> {
    use luxe_core::guided_review::EarlierConversation;
    let styles = &app.theme_styles;
    let heading = || Line::from(Span::styled("Discussion on GitHub", styles.accent));
    let (threads, more, said) = match conversation {
        EarlierConversation::Unreadable => {
            return vec![
                Line::default(),
                heading(),
                Line::from(Span::styled(
                    " Could not be read from GitHub \u{b7} o opens the pull request",
                    styles.dim,
                )),
            ];
        }
        EarlierConversation::Read { threads, more, said } => (threads, *more, said),
    };
    // Threads and comments that became feedback are listed there, not here
    let as_feedback = threads.iter().filter(|thread| thread_is_feedback(app, thread)).count()
        + said.iter().filter(|entry| is_feedback(app, &entry.id)).count();
    let threads: Vec<&luxe_core::guided_review::EarlierThread> =
        threads.iter().filter(|thread| !thread_is_feedback(app, thread)).collect();
    let said: Vec<&luxe_core::guided_review::ConversationEntry> =
        said.iter().filter(|entry| !is_feedback(app, &entry.id)).collect();
    if threads.is_empty() && more == 0 && as_feedback == 0 && said.is_empty() {
        return Vec::new();
    }
    let parts = discussion_rollup(&threads, more, as_feedback, said.len());
    let summary = format!(" {}", parts.join(" \u{b7} "));
    let mut lines = vec![Line::default(), heading(), Line::from(Span::styled(summary, styles.dim))];
    lines.extend(off_diff_thread_rows(app, &threads, width));
    lines.extend(conversation_entry_rows(app, &said, width));
    if more > 0 {
        lines.push(Line::from(Span::styled(
            format!("   {more} more on GitHub \u{b7} o"),
            styles.dim,
        )));
    }
    lines
}

/// The discussion's one-line count: what became feedback, the threads left and how many are
/// unresolved, and the comments in the conversation.
fn discussion_rollup(
    threads: &[&luxe_core::guided_review::EarlierThread],
    more: usize,
    as_feedback: usize,
    said: usize,
) -> Vec<String> {
    let unresolved = threads.iter().filter(|thread| !thread.resolved).count();
    let total = threads.len() + more;
    let noun = if total == 1 { "thread" } else { "threads" };
    let mut parts = Vec::new();
    if as_feedback > 0 {
        parts.push(format!("{as_feedback} in your feedback"));
        if total > 0 {
            parts.push(format!("{total} more {noun}"));
        }
    } else {
        parts.push(format!("{total} {noun}"));
    }
    if total > 0 {
        parts.push(format!("{unresolved} unresolved"));
    }
    // A conversation with no threads at all still says so, rather than counting zero of them
    if total == 0 && as_feedback == 0 {
        parts.clear();
    }
    match said {
        0 => {}
        1 => parts.push("1 comment".into()),
        n => parts.push(format!("{n} comments")),
    }
    parts
}

/// The threads on code the diff no longer shows, each as where it is and how it opened.
fn off_diff_thread_rows(
    app: &App,
    threads: &[&luxe_core::guided_review::EarlierThread],
    width: usize,
) -> Vec<Line<'static>> {
    let styles = &app.theme_styles;
    let mut lines = Vec::new();
    for thread in threads.iter().filter(|thread| thread.anchor.is_none()) {
        let mut place = vec![Span::styled(thread.path.clone(), styles.text)];
        let mut facts = Vec::new();
        match (thread.file_level, thread.line) {
            (true, _) => facts.push("whole file".to_owned()),
            (false, Some(line)) if !thread.outdated => {
                place.push(Span::styled(format!(":{line}"), styles.text));
            }
            _ => {}
        }
        if thread.outdated {
            facts.push("outdated".into());
        }
        if thread.resolved {
            facts.push("resolved".into());
        }
        if !facts.is_empty() {
            place.push(Span::styled(format!(" \u{b7} {}", facts.join(" \u{b7} ")), styles.dim));
        }
        let opening = thread.comments.first();
        let author = opening.map_or_else(String::new, |comment| display_author(&comment.author));
        let said = opening.and_then(|comment| comment.body.lines().next()).unwrap_or_default();
        lines.extend(discussion_item(app, &author, place, said, width));
    }
    lines
}

/// The comments in the conversation itself, each as who said it and its first line.
fn conversation_entry_rows(
    app: &App,
    said: &[&luxe_core::guided_review::ConversationEntry],
    width: usize,
) -> Vec<Line<'static>> {
    let styles = &app.theme_styles;
    let mut lines = Vec::new();
    for entry in said {
        let first = entry.body.lines().map(str::trim).find(|line| !line.is_empty());
        let author = display_author(&entry.author);
        let place = vec![Span::styled("in the conversation", styles.dim)];
        lines.extend(discussion_item(app, &author, place, first.unwrap_or_default(), width));
    }
    lines
}

/// One entry of the discussion: who and where on one line, what they said under it.
///
/// Built from spans rather than handed to the markdown renderer with the path in backticks, because
/// the quote is a stranger's text and whatever markup it carries must stay text. The name wears the
/// secondary accent and the place the brightest ink, so a reader scanning the list finds both
/// without the quote competing for either.
fn discussion_item(
    app: &App,
    author: &str,
    place: Vec<Span<'static>>,
    said: &str,
    width: usize,
) -> Vec<Line<'static>> {
    let styles = &app.theme_styles;
    let budget = width.saturating_sub(3).max(20);
    let mut head = vec![
        Span::styled(author.to_owned(), styles.accent_secondary),
        Span::styled(" \u{b7} ", styles.dim),
    ];
    head.extend(place);
    let mut lines = Vec::new();
    for (index, row) in crate::widgets::wrap_line(&Line::from(head), budget).into_iter().enumerate()
    {
        let lead =
            if index == 0 { Span::styled(" \u{2022} ", styles.dim) } else { Span::raw("   ") };
        let mut spans = vec![lead];
        spans.extend(row.spans);
        lines.push(Line::from(spans));
    }
    let said = said.trim();
    if !said.is_empty() {
        let quote = Line::from(Span::styled(said.to_owned(), styles.muted));
        for row in crate::widgets::wrap_line(&quote, budget).into_iter().take(3) {
            let mut spans = vec![Span::raw("   ")];
            spans.extend(row.spans);
            lines.push(Line::from(spans));
        }
    }
    lines
}
