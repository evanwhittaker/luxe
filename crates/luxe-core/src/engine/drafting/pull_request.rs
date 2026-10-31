//! Pull request drafting — the agent writes the title and description of a pull request it opens.
//!
//! The work under review is this session's, so the agent that did it is the one to describe it,
//! from the transcript it already holds. What the brief adds is what only the preview knows: where
//! the pull request merges, the issue it closes, and the guide's reading of the change. The answer
//! comes back in the same `luxe-draft` fence as any draft, its first line the title and the rest
//! the description, split by [`split_pull_request_draft`].

use serde::{Deserialize, Serialize};

use crate::github::untrusted;

/// The pull request being drafted, before it exists: what the preview says about it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestAsk {
    /// Branch it merges into
    pub base: String,
    /// Branch it carries, as GitHub names it (`owner:branch` from a fork)
    pub head: String,
    /// Issue it closes, when the session took one up
    pub closes: Option<u64>,
    /// The guide's reading of the change, when the review has one
    pub guide: Option<String>,
}

/// How a pull request draft's brief opens, which the transcript reads to fold it to one line.
pub(crate) const OPENER: &str = "Draft the title and description of the pull request";

/// The opening of a pull request's brief: where it merges, what it closes, and the guide's
/// reading. What the work was and why is already in the session drafting it.
pub(super) fn pull_request_brief(ask: &PullRequestAsk) -> (String, Vec<String>) {
    let mut sources = vec!["this session".to_string()];
    let mut message = instructions(ask);
    if let Some(number) = ask.closes {
        sources.push(format!("issue #{number}"));
        message.push_str(&format!("\nIt closes issue #{number}: end with `Closes #{number}`.\n"));
    }
    if let Some(guide) = ask.guide.as_deref().filter(|guide| !guide.trim().is_empty()) {
        sources.push("the guide".into());
        message.push_str(&format!(
            "\n## The guide's reading of the change\n\n{}\n",
            untrusted(guide, 4_000)
        ));
    }
    (message, sources)
}

/// What the agent is asked for, and the shape its answer comes back in.
fn instructions(ask: &PullRequestAsk) -> String {
    format!(
        "{OPENER} that will merge `{}` into `{}`, for me to read, change and open. You are not \
         opening anything, and this turn changes nothing: read what you need, then answer with \
         the draft inside one ```luxe-draft fenced block and nothing else of substance. Its first \
         line is the title, plain text under 72 characters; then a blank line; then the \
         description in markdown.\n\n\
         Write it for a reviewer who has not seen this session: what the change does and why, \
         and anything they should look at closely. Describe the change as it is in the diff \
         against `{}`, not the conversation that led to it, and say only what that evidence \
         supports. Keep it as short as the change allows, with no greeting or sign-off.\n\n\
         The shell is off for this turn, and so is anything that changes a file: read with \
         `read`, `grep` and `glob`, and the `git` tool's `log`, `diff` and `show`.\n",
        ask.head, ask.base, ask.base,
    )
}

/// A drafted pull request's title and description: the first line that says anything, without a
/// heading's marks, and everything after it.
///
/// A model asked for "a title, then a description" still writes `# Title` or `Title: …` often
/// enough that both are taken off; a title can't carry markdown on GitHub anyway.
pub fn split_pull_request_draft(draft: &str) -> (String, String) {
    let mut lines = draft.lines();
    let title = lines.by_ref().map(str::trim).find(|line| !line.is_empty()).unwrap_or("");
    let title = title.trim_start_matches('#').trim();
    let title = title.strip_prefix("Title:").unwrap_or(title).trim();
    let body = lines.collect::<Vec<_>>().join("\n");
    (title.to_string(), body.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(closes: Option<u64>, guide: Option<&str>) -> PullRequestAsk {
        PullRequestAsk {
            base: "main".into(),
            head: "todo-overdue".into(),
            closes,
            guide: guide.map(str::to_string),
        }
    }

    /// The brief names both branches and the title-then-description shape it reads back, and the
    /// transcript folds it to one line.
    #[test]
    fn the_brief_says_where_it_merges_and_how_to_answer() {
        let (message, sources) = pull_request_brief(&ask(None, None));
        assert!(message.contains("merge `todo-overdue` into `main`"), "{message}");
        assert!(message.contains("first line is the title"));
        assert!(message.contains("```luxe-draft"));
        assert!(!message.contains("Closes #"), "no issue, no closing line");
        assert_eq!(sources, ["this session"]);
        assert_eq!(crate::briefs::label(&message), Some("Drafted the pull request"));
    }

    /// An issue the session took up is closed by the description, and the guide's reading goes
    /// with it as data.
    #[test]
    fn the_brief_carries_the_issue_and_the_guide() {
        let (message, sources) = pull_request_brief(&ask(Some(42), Some("Adds `todo overdue`.")));
        assert!(message.contains("end with `Closes #42`"), "{message}");
        assert!(message.contains("## The guide's reading of the change\n\nAdds `todo overdue`."));
        assert_eq!(sources, ["this session", "issue #42", "the guide"]);
        let (blank_guide, _) = pull_request_brief(&ask(None, Some("   ")));
        assert!(!blank_guide.contains("guide's reading"), "a blank guide is no guide");
    }

    /// What `luxe --rpc` clients send for it, so a renamed field or tag fails here first.
    #[test]
    fn the_target_reads_from_its_wire_form() {
        let wire = r#"{"kind":"pull_request","base":"main","head":"todo-overdue","closes":42,"guide":null}"#;
        let target: super::super::DraftTarget = serde_json::from_str(wire).expect("it reads");
        assert_eq!(target, super::super::DraftTarget::PullRequest(ask(Some(42), None)));
    }

    /// The first line that says anything is the title, stripped of a heading's marks or a
    /// `Title:` label; the rest, trimmed, is the description.
    #[test]
    fn a_draft_splits_into_its_title_and_description() {
        let split = |draft: &str| split_pull_request_draft(draft);
        assert_eq!(
            split("Add `todo overdue`\n\nLists open tasks past due.\n\nCloses #42\n"),
            ("Add `todo overdue`".into(), "Lists open tasks past due.\n\nCloses #42".into())
        );
        assert_eq!(split("\n## Add it\nBody").0, "Add it");
        assert_eq!(split("Title: Add it\n\nBody").0, "Add it");
        assert_eq!(split("Only a title"), ("Only a title".into(), String::new()));
        assert_eq!(split(""), (String::new(), String::new()));
    }
}
