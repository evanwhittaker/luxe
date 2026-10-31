//! Sixel frame — where Sixel images go once a frame's cells are out.
//!
//! A Sixel image is not part of the cell buffer, so the store paints blank, marked cells where one
//! belongs and draws the picture over them afterwards. The terminal keeps the picture with those
//! cells: writing a character into one wipes the picture from it. That is the only fact this
//! relies on, and it is what makes the rest small — an image whose cells ratatui did not write is
//! still showing, one whose cells it did write is gone, and an image that moves is drawn at its
//! new place while the cells it left are written blank by the diff.
//!
//! So a frame redraws exactly the images whose placement differs from the one the last frame left
//! behind. A frame that changes nothing about an image costs the terminal nothing for it.

use super::{kitty, ImageStore, ImageTier, LiveKind};
use crate::graphics::sixel::Indexed;
use crate::graphics::{CellSize, ImageKey};
use ratatui::buffer::Buffer;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::ops::Range;
use std::sync::Arc;

/// Encodings kept per image. Scrolling a half-visible image asks for a new crop each step.
const CACHED_CROPS: usize = 8;

/// A picture quantized for Sixel, with the streams already encoded from it.
pub(super) struct SixelLive {
    id: u32,
    cell: CellSize,
    image: Indexed,
    streams: HashMap<(u16, u16), Arc<str>>,
}

impl SixelLive {
    /// Quantizes `page`, which is already scaled to exactly the cells it will cover.
    pub(super) fn new(id: u32, cell: CellSize, page: &image::RgbaImage) -> Self {
        Self { id, cell, image: Indexed::quantize(page), streams: HashMap::new() }
    }

    /// The id marking this picture's cells.
    pub(super) fn id(&self) -> u32 {
        self.id
    }

    /// The stream that draws image rows `rows`, encoded the first time it is asked for.
    fn stream(&mut self, rows: &Range<u16>) -> Arc<str> {
        let crop = (rows.start, rows.end);
        if let Some(stream) = self.streams.get(&crop) {
            return Arc::clone(stream);
        }
        if self.streams.len() >= CACHED_CROPS {
            self.streams.clear();
        }
        let height = u32::from(self.cell.height);
        let pixels = u32::from(rows.start) * height..u32::from(rows.end) * height;
        let stream: Arc<str> = Arc::from(self.image.encode(pixels));
        self.streams.insert(crop, Arc::clone(&stream));
        stream
    }
}

/// One row of an image as the transcript painted it this frame.
pub(super) struct PaintedRow {
    key: ImageKey,
    id: u32,
    x: u16,
    y: u16,
    image_row: u16,
    cols: u16,
}

impl PaintedRow {
    pub(super) fn new(
        key: &ImageKey,
        id: u32,
        (x, y): (u16, u16),
        cols: u16,
        image_row: u16,
    ) -> Self {
        Self { key: key.clone(), id, x, y, image_row, cols }
    }

    pub(super) fn at(&self) -> (u16, u16) {
        (self.x, self.y)
    }
}

/// Where one image sits on screen, and which rows of it show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Placement {
    key: ImageKey,
    id: u32,
    x: u16,
    y: u16,
    cols: u16,
    rows: Range<u16>,
}

impl Placement {
    fn starting_at(row: &PaintedRow) -> Self {
        let rows = row.image_row..row.image_row + 1;
        Self { key: row.key.clone(), id: row.id, x: row.x, y: row.y, cols: row.cols, rows }
    }

    /// Whether `row` is the next one down of the same image in the same column.
    fn continues_with(&self, row: &PaintedRow) -> bool {
        self.key == row.key
            && self.id == row.id
            && (self.x, self.cols) == (row.x, row.cols)
            && row.image_row == self.rows.end
            && row.y == self.y + (self.rows.end - self.rows.start)
    }

    /// Whether every cell it covers is still blank and carrying its mark.
    fn is_intact(&self, buf: &Buffer) -> bool {
        let mark = kitty::id_color(self.id);
        let height = self.rows.end - self.rows.start;
        (0..height).all(|dy| {
            (0..self.cols).all(|dx| {
                buf.cell((self.x + dx, self.y + dy))
                    .is_some_and(|cell| cell.symbol() == " " && cell.fg == mark)
            })
        })
    }

    /// Drops the rows that would put the cursor off the bottom of the screen.
    ///
    /// A Sixel leaves the cursor on the line below its last row, and a cursor pushed past the last
    /// line scrolls the whole screen — measured on Windows Terminal, which scrolls by exactly the
    /// overhang. The last line is never one an image needs.
    fn clipped_to(mut self, screen_rows: u16) -> Option<Self> {
        let room = screen_rows.saturating_sub(1).checked_sub(self.y)?;
        self.rows.end = self.rows.end.min(self.rows.start + room);
        (!self.rows.is_empty()).then_some(self)
    }
}

/// Folds painted rows into one placement per image.
fn placements(rows: &[PaintedRow], buf: &Buffer) -> Vec<Placement> {
    let mut found: Vec<Placement> = Vec::new();
    for row in rows {
        match found.last_mut() {
            Some(last) if last.continues_with(row) => last.rows.end += 1,
            _ => found.push(Placement::starting_at(row)),
        }
    }
    found
        .into_iter()
        .filter_map(|placement| placement.clipped_to(buf.area.height))
        .filter(|placement| placement.is_intact(buf))
        .collect()
}

impl ImageStore {
    /// Starts a frame: nothing is painted yet.
    pub fn begin_frame(&mut self) {
        self.painted_rows.clear();
    }

    /// Ends a frame, working out what the terminal is owed for its Sixel images.
    ///
    /// Called with the finished buffer and before it is written; what is owed comes out of
    /// [`Self::drain_control`] after, so the pictures land on top of the cells they belong to.
    pub fn finish_frame(&mut self, buf: &Buffer) {
        let rows = std::mem::take(&mut self.painted_rows);
        if self.tier != ImageTier::Sixel {
            return;
        }
        let wanted = placements(&rows, buf);
        self.overlay = self.draw_changed(&wanted);
        self.shown = wanted;
    }

    /// Forgets what the terminal is showing, so every image is drawn again.
    ///
    /// For when the screen was cleared out from under the images: a resize, or a clear.
    pub fn invalidate_overlay(&mut self) {
        self.shown.clear();
    }

    /// The Sixel for each placement the last frame did not leave on screen.
    fn draw_changed(&mut self, wanted: &[Placement]) -> String {
        let mut out = String::new();
        let changed: Vec<&Placement> =
            wanted.iter().filter(|placement| !self.shown.contains(placement)).collect();
        for placement in changed {
            let Some(stream) = self.stream_for(placement) else {
                continue;
            };
            let _ = write!(out, "\x1b[{};{}H{stream}", placement.y + 1, placement.x + 1);
        }
        // The cursor goes back where the frame left it: the composer's caret is in the cells
        if out.is_empty() {
            return out;
        }
        format!("\x1b7{out}\x1b8")
    }

    fn stream_for(&mut self, placement: &Placement) -> Option<Arc<str>> {
        let live = self.live.get_mut(&placement.key)?;
        let LiveKind::Sixel(sixel) = &mut live.kind else {
            return None;
        };
        (sixel.id == placement.id).then(|| sixel.stream(&placement.rows))
    }
}

#[cfg(test)]
mod tests;
