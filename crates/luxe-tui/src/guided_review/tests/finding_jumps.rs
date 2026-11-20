//! Jumps to each kind of finding anchor, editing a finding that's no longer a draft, and the new
//! line a deleted file's hunk header names.

use super::super::*;
use super::text_file;
use crate::fixture::{seeded_app, FixtureScenario};

fn review() -> GuidedReview {
    let app = seeded_app("nord", FixtureScenario::GuidedReviewReady).expect("fixture app");
    app.guided_review.as_deref().expect("fixture review").clone()
}

fn finding(anchor: FindingAnchor, state: FindingState) -> ReviewFinding {
    ReviewFinding {
        posted: None,
        github_thread: None,
        id: FindingId::new(),
        label: String::new(),
        anchor,
        severity: FindingSeverity::Issue,
        title: "Jump target".into(),
        body: "Look here.".into(),
        proposed_fix: None,
        state,
        thread: Vec::new(),
    }
}

/// The last file's last hunk, so a match on "any other" file or hunk lands elsewhere.
fn last_hunk(review: &GuidedReview) -> (&ReviewFile, &ReviewHunk) {
    let file = review.files.iter().rev().find(|file| !file.hunks.is_empty()).expect("a hunk");
    (file, file.hunks.last().expect("a hunk"))
}

/// The break this catches: an anchor on a removed line matched against the wrong line, landing
/// the cursor on the hunk's first row instead of the line the comment is about.
#[test]
fn an_old_side_anchor_lands_on_its_removed_line() {
    let review = review();
    let (file, hunk, index, line) = removed_line(&review);
    let anchor = FindingAnchor::Line {
        file_id: file.id.clone(),
        hunk_id: hunk.id.clone(),
        side: DiffSide::Old,
        line,
        context_hash: hunk.lines[index].context_hash(),
    };
    let mut workspace = GuidedReviewWorkspace::default();
    assert!(workspace.jump_to_finding(&finding(anchor, FindingState::Open), &review));
    assert_eq!(workspace.position.hunk_id, Some(hunk.id.clone()));
    assert_eq!(workspace.position.line_offset, index + 1, "the header is row 0");
}

/// A removed line past a hunk's first row, as its file, hunk, row and old line number.
fn removed_line(review: &GuidedReview) -> (&ReviewFile, &ReviewHunk, usize, u32) {
    let rows = review.files.iter().flat_map(|file| file.hunks.iter().map(move |hunk| (file, hunk)));
    rows.flat_map(|(file, hunk)| {
        hunk.lines.iter().enumerate().skip(1).filter_map(move |(index, source)| {
            let removed = source.new_line.is_none();
            removed.then_some((file, hunk, index, source.old_line?))
        })
    })
    .next()
    .expect("the fixture removes a line past a hunk's first row")
}

/// The break this catches: a hunk anchor matched against the wrong file or hunk, so the jump
/// fails or lands on another hunk.
#[test]
fn a_hunk_anchor_lands_on_its_hunk() {
    let review = review();
    let (file, hunk) = last_hunk(&review);
    let anchor = FindingAnchor::Hunk { file_id: file.id.clone(), hunk_id: hunk.id.clone() };
    let mut workspace = GuidedReviewWorkspace::default();
    assert!(workspace.jump_to_finding(&finding(anchor, FindingState::Open), &review));
    assert_eq!(workspace.position.file_id, Some(file.id.clone()));
    assert_eq!(workspace.position.hunk_id, Some(hunk.id.clone()));
}

/// The break this catches: a chapter anchor counted as known when any other chapter exists, so a
/// comment on a chapter the review no longer has moves the reader anyway.
#[test]
fn a_chapter_anchor_opens_its_chapter_and_only_a_known_one() {
    let review = review();
    let chapter_id = review.chapters.last().expect("a chapter").id.clone();
    let comment =
        finding(FindingAnchor::Chapter { chapter_id: chapter_id.clone() }, FindingState::Open);
    let mut workspace = GuidedReviewWorkspace::default();
    assert!(workspace.jump_to_finding(&comment, &review));
    assert_eq!(workspace.position.chapter_id, Some(chapter_id.clone()));

    let gone = luxe_core::guided_review::ChapterId::from_item_keys("gone", &["nothing"]);
    let stale = finding(FindingAnchor::Chapter { chapter_id: gone }, FindingState::Open);
    let mut workspace = GuidedReviewWorkspace::default();
    assert!(!workspace.jump_to_finding(&stale, &review), "a chapter the review doesn't have");
    assert_eq!(workspace.position.chapter_id, None);
}

/// The break this catches: reopening a sent comment in the draft editor, where saving would
/// rewrite feedback the agent already has.
#[test]
fn only_a_draft_reopens_in_the_editor() {
    let mut workspace = GuidedReviewWorkspace::default();
    let sent = finding(FindingAnchor::Review, FindingState::Open);
    assert!(!workspace.begin_edit_finding_draft(&sent));
    assert!(workspace.finding_draft.is_none());
}

/// The break this catches: a deleted file's hunk header (its new side starts at line 0) naming
/// line 0, which an editor can't open.
#[test]
fn a_deleted_files_hunk_header_names_no_new_line() {
    let mut file = text_file("src/gone.rs");
    file.hunks[0].new_range = luxe_core::guided_review::LineRange::new(0, 0);
    let mut workspace = GuidedReviewWorkspace::default();
    workspace.position.file_id = Some(file.id.clone());
    workspace.position.hunk_id = Some(file.hunks[0].id.clone());
    workspace.position.line_offset = 0;
    let files = [file];
    let (_, line) = workspace.selected_new_line(&files).expect("the file");
    assert_eq!(line, None);
}
