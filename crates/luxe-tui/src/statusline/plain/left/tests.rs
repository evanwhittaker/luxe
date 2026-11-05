use super::super::{Kind, Surface};
use super::*;
use crate::glyphs::GlyphTier;
use luxe_core::github::{CheckRollup, PullRequestState, PullRequestStatus};

fn pr(failed: usize) -> PullRequestStatus {
    PullRequestStatus {
        number: 138,
        title: "Groundwork".into(),
        url: "https://github.com/acme/app/pull/138".into(),
        state: PullRequestState::Open,
        draft: false,
        review_decision: None,
        checks: CheckRollup { passed: 3, failed, pending: 0, skipped: 0 },
        unresolved_threads: 0,
        head_oid: "abc".into(),
        stack: None,
    }
}

fn words(facts: &Facts<'_>) -> Vec<(Kind, String, Tone, Surface)> {
    left_segments(facts).into_iter().map(|s| (s.kind, s.label, s.tone, s.surface)).collect()
}

/// The break this catches: the phone's left side worded, ordered or surfaced apart from the bar's,
/// which reads the pull request on a raised slab, then the plan and the change count bare.
#[test]
fn the_left_facts_read_as_the_bar_words_them() {
    let pr = pr(0);
    let todos = TodoProgress { completed: 3, total: 8, current: Some("Fix the parser") };
    let facts =
        Facts { pull_request: Some(&pr), todos: Some(todos), changed_files: 5, ..Facts::default() };
    let expected = [
        (Kind::PullRequest, "#138 · checks pass".into(), Tone::Plain, Surface::Raised),
        (Kind::Todo, "3/8 · Fix the parser".into(), Tone::Plain, Surface::Bare),
        (Kind::Changes, "5 files".into(), Tone::Plain, Surface::Bare),
    ];
    assert_eq!(words(&facts), expected);
    let marks: Vec<_> = left_segments(&facts).into_iter().map(|s| s.glyph).collect();
    let nerd = |icon: Icon| Some(icon.at(GlyphTier::Nerd));
    assert_eq!(marks, [nerd(Icon::PullRequest), nerd(Icon::Plan), nerd(Icon::Changes)]);
}

/// The break this catches: a failing check left unmarked, so the one thing on the pull request
/// waiting on its author reads as all clear.
#[test]
fn a_failing_check_marks_the_pull_request() {
    let pr = pr(2);
    let facts = Facts { pull_request: Some(&pr), ..Facts::default() };
    assert_eq!(left_segments(&facts)[0].tone, Tone::Warning);
}

/// The break this catches: a quiet session drawing `0/0`, `0 files` or an empty pull request
/// module, where the bar draws nothing until there's something to say.
#[test]
fn nothing_to_say_draws_nothing() {
    let none = TodoProgress { completed: 0, total: 0, current: None };
    assert!(left_segments(&Facts { todos: Some(none), ..Facts::default() }).is_empty());
    let one = Facts { changed_files: 1, ..Facts::default() };
    assert_eq!(left_segments(&one)[0].label, "1 file");
}
