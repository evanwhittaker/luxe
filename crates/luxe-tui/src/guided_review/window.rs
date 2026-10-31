//! Review window — which rows of the diff are on screen, apart from the row the cursor is on.
//!
//! `position.line_offset` is the cursor. The window is where the last frame started drawing, and
//! the next frame keeps it while the cursor is where that frame left it, so a line move or a click
//! moves the cursor on the screen rather than dragging the code under a fixed top row. Any other
//! move (a hunk, file or finding jump, a search hit) leaves the window behind, and the frame after
//! it starts at the cursor's row, as it always has.

use super::GuidedReviewWorkspace;

/// The diff window a frame drew.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct DiffWindow {
    /// The first row drawn
    pub(crate) top: usize,
    /// The cursor's row when it was drawn
    pub(crate) cursor: usize,
}

impl GuidedReviewWorkspace {
    /// Keeps the window where it is for the cursor's new row, so the next frame scrolls only as
    /// far as it must to show it.
    pub(crate) fn keep_diff_window(&self) {
        let window = self.diff_window.get();
        self.diff_window.set(DiffWindow { cursor: self.position.line_offset, ..window });
    }

    /// Moves the window as many rows as the cursor moved from `before`, so the wheel scrolls the
    /// code and the cursor rides along on its screen row.
    pub(crate) fn carry_diff_window(&self, before: usize) {
        let cursor = self.position.line_offset;
        let top = (self.diff_window.get().top + cursor).saturating_sub(before);
        self.diff_window.set(DiffWindow { top, cursor });
    }

    /// Where the next frame starts drawing, before it checks the cursor fits: the window's top
    /// while the cursor is where the last frame left it, else the cursor's own row. Never below
    /// the cursor, nor so far above it that `height` rows could not reach it.
    pub(crate) fn diff_window_start(&self, height: usize) -> usize {
        let cursor = self.position.line_offset;
        let window = self.diff_window.get();
        let top = if window.cursor == cursor { window.top.min(cursor) } else { cursor };
        top.max((cursor + 1).saturating_sub(height.max(1)))
    }

    /// Records the window a frame drew from `top`.
    pub(crate) fn record_diff_window(&self, top: usize) {
        self.diff_window.set(DiffWindow { top, cursor: self.position.line_offset });
    }

    /// The first row the last frame drew.
    pub(crate) fn diff_top(&self) -> usize {
        self.diff_window.get().top
    }
}

#[cfg(test)]
mod tests;
