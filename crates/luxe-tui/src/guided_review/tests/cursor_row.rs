//! The comment under the diff cursor: which comments sit on its row, and which one acts.

use super::super::*;
use super::text_file;
use luxe_core::guided_review::{ReviewLine, ReviewLineKind};

fn comment(anchor: FindingAnchor) -> ReviewFinding {
    ReviewFinding {
        posted: None,
        github_thread: None,
        id: FindingId::new(),
        label: String::new(),
        anchor,
        severity: FindingSeverity::Issue,
        title: "Fixture".into(),
        body: "Fixture comment.".into(),
        proposed_fix: None,
        state: FindingState::Draft,
        thread: Vec::new(),
    }
}

/// A hunk of a removed line 7 and an added line 7, so only the side tells them apart.
fn replaced_line_file() -> ReviewFile {
    let mut file = text_file("src/cursor.rs");
    let line = |old_line, new_line, kind| ReviewLine { old_line, new_line, kind, text: "x".into() };
    file.hunks[0].lines = vec![
        line(Some(7), None, ReviewLineKind::Deletion),
        line(None, Some(7), ReviewLineKind::Addition),
    ];
    file
}

fn line_anchor(file: &ReviewFile, side: DiffSide) -> FindingAnchor {
    FindingAnchor::Line {
        file_id: file.id.clone(),
        hunk_id: file.hunks[0].id.clone(),
        side,
        line: 7,
        context_hash: String::new(),
    }
}

fn workspace_at(file: &ReviewFile, line_offset: usize) -> GuidedReviewWorkspace {
    let mut workspace = GuidedReviewWorkspace::default();
    workspace.position.file_id = Some(file.id.clone());
    workspace.position.hunk_id = Some(file.hunks[0].id.clone());
    workspace.position.line_offset = line_offset;
    workspace
}

/// The break this catches: matching a line by number alone, which would put the comment on the
/// removed line 7 under the added line 7 too, and let `x` there delete it.
#[test]
fn only_the_comments_drawn_on_the_cursor_row_are_at_the_cursor() {
    let file = replaced_line_file();
    let hunk = FindingAnchor::Hunk { file_id: file.id.clone(), hunk_id: file.hunks[0].id.clone() };
    let findings = vec![
        comment(line_anchor(&file, DiffSide::Old)),
        comment(line_anchor(&file, DiffSide::New)),
        comment(hunk),
    ];
    let files = std::slice::from_ref(&file);
    let ids_at = |offset| {
        let workspace = workspace_at(&file, offset);
        let here = workspace.comments_at_cursor(files, &findings);
        here.into_iter().map(|finding| finding.id.clone()).collect::<Vec<_>>()
    };

    assert_eq!(ids_at(0), vec![findings[2].id.clone()], "the header holds the hunk's comment");
    assert_eq!(ids_at(1), vec![findings[0].id.clone()], "the removed line holds the old side's");
    assert_eq!(ids_at(2), vec![findings[1].id.clone()], "the added line holds the new side's");
    assert!(ids_at(3).is_empty(), "past the last row nothing is under the cursor");
}

/// The break this catches: acting on the feedback window's old selection when it is not on this
/// row, or on the oldest comment here rather than the one just written.
#[test]
fn the_comment_at_the_cursor_is_the_selected_one_there_else_the_newest() {
    let file = replaced_line_file();
    let findings = vec![
        comment(line_anchor(&file, DiffSide::New)),
        comment(line_anchor(&file, DiffSide::New)),
        comment(line_anchor(&file, DiffSide::Old)),
    ];
    let files = std::slice::from_ref(&file);
    let mut workspace = workspace_at(&file, 2);

    workspace.selected_finding_id = Some(findings[2].id.clone());
    assert_eq!(workspace.comment_at_cursor(files, &findings), Some(findings[1].id.clone()));

    workspace.selected_finding_id = Some(findings[0].id.clone());
    assert_eq!(workspace.comment_at_cursor(files, &findings), Some(findings[0].id.clone()));

    workspace.position.line_offset = 3;
    assert_eq!(workspace.comment_at_cursor(files, &findings), None);
}
