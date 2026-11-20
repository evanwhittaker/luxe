//! Cursor — where the keyboard stands in the pane, on either face.
//!
//! The File view stands on a line of the file and the diff on a row of its own row space (a hunk's
//! header, then its lines). Each face keeps its own, so switching faces never lands the cursor on a
//! number that meant something else on the other one.

use super::cells::diff_row_lines;
use super::{ChangesView, PeekView};

/// Lines shown above a line the pane lands on, so a hit is read with what leads up to it.
pub(super) const LANDING_LEAD: usize = 5;

impl ChangesView {
    /// Gives the pane the keyboard, with a cursor on the first row showing on a face that has none.
    pub(crate) fn focus(&mut self) {
        self.focused = true;
        let (line, row) = (self.top_line(), self.scroll);
        self.cursor.get_or_insert(line);
        self.diff_cursor.get_or_insert(row);
    }

    /// Gives the pane the keyboard with the showing face's cursor on `row`, where a press landed.
    pub(crate) fn focus_on_row(&mut self, row: usize) {
        self.focused = true;
        self.stand_on_row(row);
    }

    /// Puts the showing face's cursor on `row`, scrolling the least that keeps it on screen.
    pub(crate) fn stand_on_row(&mut self, row: usize) {
        match self.view {
            PeekView::File => {
                self.cursor = Some(u32::try_from(row + 1).unwrap_or(u32::MAX));
                self.keep_cursor_in_view();
            }
            PeekView::Diff => {
                self.diff_cursor = Some(row);
                self.keep_diff_cursor_in_view();
            }
        }
    }

    /// Moves the File view's cursor by `delta` lines, to a line that exists, scrolling to keep it
    /// on screen. With `extend`, or in visual-line mode, the pick grows from where it began to
    /// where the cursor lands; otherwise a pick is dropped, the way moving the caret drops a
    /// selection everywhere else.
    pub(crate) fn move_cursor(&mut self, delta: i64, extend: bool) {
        let Some(lines) = self.text_lines() else { return };
        let extend = extend || self.visual;
        let from = self.cursor.unwrap_or_else(|| self.top_line());
        let to = (i64::from(from) + delta).clamp(1, i64::from(lines));
        let to = u32::try_from(to).unwrap_or(1);
        self.cursor = Some(to);
        self.selection = match (extend, self.selection) {
            (true, Some((anchor, _))) => Some((anchor, to)),
            (true, None) => Some((from, to)),
            (false, _) => None,
        };
        self.keep_cursor_in_view();
    }

    /// Enters visual-line mode on the cursor's line, or leaves it and drops the pick.
    pub(crate) fn toggle_visual(&mut self) {
        if self.visual {
            self.visual = false;
            self.selection = None;
            return;
        }
        let at = self.cursor.unwrap_or_else(|| self.top_line());
        self.cursor = Some(at);
        self.visual = true;
        self.selection = Some((at, at));
    }

    /// Puts the cursor at the pick's other end, as `o` does in vim's visual mode, so either end can
    /// be moved.
    pub(crate) fn swap_pick_ends(&mut self) {
        if let Some((anchor, head)) = self.selection {
            self.selection = Some((head, anchor));
            self.cursor = Some(anchor);
            self.keep_cursor_in_view();
        }
    }

    /// The first line on screen, where a cursor starts when the pane is first entered.
    pub(crate) fn top_line(&self) -> u32 {
        u32::try_from(self.scroll + 1).unwrap_or(u32::MAX)
    }

    /// Puts the File view's cursor on `line`, with a few lines above it so what leads up to it
    /// shows.
    ///
    /// Called before the text arrives, so it can't check the line exists. The frame clamps the
    /// offset, and moving clamps the cursor, once there is text to clamp against.
    pub(crate) fn land_on(&mut self, line: u32) {
        let line = line.max(1);
        self.cursor = Some(line);
        self.selection = None;
        self.visual = false;
        self.scroll = (line as usize - 1).saturating_sub(LANDING_LEAD);
    }

    /// Scrolls the least that puts the cursor's line on screen.
    fn keep_cursor_in_view(&mut self) {
        let Some(cursor) = self.cursor else { return };
        let line = cursor.saturating_sub(1) as usize;
        let rows = self.viewport_rows.get().max(1);
        if line < self.scroll {
            self.scroll = line;
        } else if line >= self.scroll + rows {
            self.scroll = line + 1 - rows;
        }
    }

    /// Moves the diff's cursor by `delta` rows, to one that exists, scrolling the least that keeps
    /// it on screen.
    pub(crate) fn move_diff_cursor(&mut self, delta: i64) {
        let Some(rows) = self.file().map(|file| diff_row_lines(file).count()) else { return };
        let Some(last) = rows.checked_sub(1) else { return };
        let from = i64::try_from(self.diff_cursor.unwrap_or(self.scroll)).unwrap_or(i64::MAX);
        let to = from.saturating_add(delta).clamp(0, i64::try_from(last).unwrap_or(i64::MAX));
        self.diff_cursor = Some(usize::try_from(to).unwrap_or(0));
        self.keep_diff_cursor_in_view();
    }

    /// Scrolls the least that puts the diff cursor's whole row on screen.
    fn keep_diff_cursor_in_view(&mut self) {
        let Some(cursor) = self.diff_cursor else { return };
        if cursor < self.scroll {
            self.scroll = cursor;
            return;
        }
        self.scroll = self.scroll.max(self.lowest_top_showing(cursor));
    }

    /// The first row the view can start at and still show all of `row`, by the heights the last
    /// frame drew. A row no frame has measured yet counts as one line.
    fn lowest_top_showing(&self, row: usize) -> usize {
        let heights = self.diff_row_heights.borrow();
        let room = self.viewport_rows.get().max(1);
        let mut used = 0usize;
        let mut top = row;
        for at in (0..=row).rev() {
            used += heights.get(at).copied().unwrap_or(1);
            if used > room {
                break;
            }
            top = at;
        }
        top
    }
}

#[cfg(test)]
mod tests;
