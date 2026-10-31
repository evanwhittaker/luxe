//! The diff row under the cursor, and the comments that sit on it.
//!
//! One walk of the visible rows answers every "what is the cursor on" question: where a new
//! comment anchors, which line an editor opens at, and which comment `e`, `x` and `r` act on
//! while the feedback window is shut.

use super::*;
use luxe_core::guided_review::ReviewLine;

/// What one visible row of the diff is.
#[derive(Clone, Copy)]
pub(crate) enum CursorRow<'a> {
    /// A collapsed run of mechanical hunks
    Folded,
    /// A hunk's header row
    Header(&'a ReviewHunk),
    /// A source line, by its index in the hunk
    Line(&'a ReviewHunk, usize),
}

impl GuidedReviewWorkspace {
    /// The row the cursor is on in `file`, or `None` when the cursor is past its last row.
    pub(crate) fn cursor_row<'a>(&self, file: &'a ReviewFile) -> Option<CursorRow<'a>> {
        let target = self.position.line_offset;
        let mut row = 0usize;
        for group in visible_groups(file, &self.fold) {
            let height = group.rows();
            if let Some(offset) = target.checked_sub(row).filter(|at| *at < height) {
                return Some(match group {
                    VisibleGroup::Hunk(hunk) if offset > 0 => CursorRow::Line(hunk, offset - 1),
                    VisibleGroup::Hunk(hunk) => CursorRow::Header(hunk),
                    VisibleGroup::Folded(_) => CursorRow::Folded,
                });
            }
            row = row.saturating_add(height);
        }
        None
    }

    /// The comments drawn under the cursor's row, in the order the session holds them.
    pub(crate) fn comments_at_cursor<'a>(
        &self,
        files: &[ReviewFile],
        findings: &'a [ReviewFinding],
    ) -> Vec<&'a ReviewFinding> {
        let Some(file) = self.selected_file(files) else { return Vec::new() };
        let Some(row) = self.cursor_row(file) else { return Vec::new() };
        findings.iter().filter(|finding| anchor_on_row(&finding.anchor, file, row)).collect()
    }

    /// The comment `e`, `x` and `r` act on from the diff: the window's selection when it sits on
    /// the cursor's row, else the newest comment there.
    pub(crate) fn comment_at_cursor(
        &self,
        files: &[ReviewFile],
        findings: &[ReviewFinding],
    ) -> Option<FindingId> {
        let here = self.comments_at_cursor(files, findings);
        let selected = self.selected_finding_id.as_ref();
        let kept = here.iter().find(|finding| Some(&finding.id) == selected);
        kept.or(here.last()).map(|finding| finding.id.clone())
    }
}

/// Whether a comment anchored at `anchor` is drawn under `row` of `file`.
fn anchor_on_row(anchor: &FindingAnchor, file: &ReviewFile, row: CursorRow<'_>) -> bool {
    match row {
        CursorRow::Header(hunk) => hunk_anchor_matches(anchor, &hunk.id),
        CursorRow::Line(hunk, index) => {
            hunk.lines.get(index).is_some_and(|line| line_anchor_matches(anchor, &file.id, line))
        }
        CursorRow::Folded => false,
    }
}

/// Whether a line anchor points at this exact source line, on the side it names.
pub(crate) fn line_anchor_matches(
    anchor: &FindingAnchor,
    file_id: &FileId,
    line: &ReviewLine,
) -> bool {
    let FindingAnchor::Line { file_id: anchor_file, side, line: anchor_line, .. } = anchor else {
        return false;
    };
    anchor_file == file_id
        && match side {
            DiffSide::Old => line.old_line == Some(*anchor_line),
            DiffSide::New => line.new_line == Some(*anchor_line),
        }
}

/// Whether an anchor is the whole of the hunk `hunk_id`.
pub(crate) fn hunk_anchor_matches(anchor: &FindingAnchor, hunk_id: &HunkId) -> bool {
    matches!(anchor, FindingAnchor::Hunk { hunk_id: at, .. } if at == hunk_id)
}
