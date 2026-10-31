//! Cursor — where the keyboard stands in the pane, on either face.
//!
//! The File view stands on a line of the file and the diff on a row of its own row space (a hunk's
//! header, then its lines). Each face keeps its own, so switching faces never lands the cursor on a
//! number that meant something else on the other one.

use super::cells::diff_row_lines;
use super::{ChangesView, PeekView};

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
