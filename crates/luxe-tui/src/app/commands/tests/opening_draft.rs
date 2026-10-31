//! The pull request preview's `d`: the agent drafts the title and description in place.

use super::opening::{branch_review, prepared_event};
use super::*;
use luxe_core::engine::DraftTarget;
use luxe_core::event::Event;

/// A preview with its draft read, and the request the engine was asked.
fn preview() -> (App, mpsc::Receiver<Action>) {
    let (mut app, mut engine_rx) = branch_review(luxe_core::workspace::PullRequestPresence::None);
    app.update(crate::app::Msg::GuidedReviewOpenPullRequest);
    let Ok(Action::PreparePullRequest { request_id, .. }) = engine_rx.try_recv() else {
        panic!("expected a prepare");
    };
    app.update(prepared_event(request_id, &[]));
    (app, engine_rx)
}

fn drafted(request_id: &str, body: Option<&str>, error: Option<&str>) -> crate::app::Msg {
    crate::app::Msg::Engine(Event::GithubReplyDrafted {
        request_id: request_id.into(),
        body: body.map(str::to_string),
        sources: vec!["this session".into()],
        error: error.map(str::to_string),
        note: Some("I didn't run the tests this turn.".into()),
    })
}

/// `d` asks for a draft of this pull request, carrying both branches and what the reader has, and
/// what comes back becomes the title and the description. The break this catches: `d` toggling
/// GitHub's draft flag, as it used to, or the answer landing nowhere.
#[test]
fn d_has_the_agent_draft_the_title_and_description() {
    let (mut app, mut engine_rx) = preview();
    app.update(key(crossterm::event::KeyCode::Char('d')));
    let Ok(Action::DraftGithubReply(request)) = engine_rx.try_recv() else {
        panic!("expected a draft request");
    };
    let DraftTarget::PullRequest(ask) = &request.target else { panic!("{:?}", request.target) };
    assert_eq!((ask.base.as_str(), ask.head.as_str()), ("main", "7-fix"));
    let current = request.current.as_deref().expect("the reader's draft goes with it");
    assert!(current.starts_with("Fix the total\n\n### Sum every element"), "{current}");

    // While it drafts, nothing the draft would overwrite can change, and it isn't asked twice
    app.update(key(crossterm::event::KeyCode::Char('t')));
    app.update(key(crossterm::event::KeyCode::Char('d')));
    assert!(engine_rx.try_recv().is_err(), "once at a time");
    let opening = app.guided_review_workspace.pull_request_opening.as_ref().expect("preview");
    assert!(opening.editing.is_none() && !opening.prepared.as_ref().unwrap().pull_request.draft);

    let answer = "Sum every element of the total\n\nThe loop started at 1.\n\nCloses #7";
    app.update(drafted(&request.request_id, Some(answer), None));
    let opening = app.guided_review_workspace.pull_request_opening.as_ref().expect("preview");
    let pull_request = &opening.prepared.as_ref().expect("prepared").pull_request;
    assert_eq!(pull_request.title, "Sum every element of the total");
    assert_eq!(pull_request.body, "The loop started at 1.\n\nCloses #7");
    assert!(opening.drafting.is_none());
    assert_eq!(opening.draft_note.as_deref(), Some("I didn't run the tests this turn."));
}

/// A failed draft says why and leaves what was there; another preview's answer is not this one's.
#[test]
fn a_failed_or_foreign_draft_leaves_the_preview_as_it_was() {
    let (mut app, mut engine_rx) = preview();
    app.update(key(crossterm::event::KeyCode::Char('d')));
    let Ok(Action::DraftGithubReply(request)) = engine_rx.try_recv() else { panic!("asked") };

    app.update(drafted("draft:review:7:1", Some("Elsewhere\n\nNot this."), None));
    let opening = app.guided_review_workspace.pull_request_opening.as_ref().expect("preview");
    assert!(opening.drafting.is_some(), "still waiting on its own");

    app.update(drafted(&request.request_id, None, Some("The agent answered without a draft.")));
    let opening = app.guided_review_workspace.pull_request_opening.as_ref().expect("preview");
    assert!(opening.drafting.is_none(), "ready to ask again");
    assert_eq!(opening.prepared.as_ref().unwrap().pull_request.title, "Fix the total");
}
