//! Selection steps — Shift and an arrow moving a selection's free end over rendered lines.
//!
//! The anchor stays where the drag began; each step moves the cursor a character or a line, the
//! way Shift and an arrow grow a selection in any editor. Columns are terminal cells, like the rest
//! of [`crate::selection`], so a step over a wide glyph moves past all of it.

use super::{cell_width, TextPoint};
use unicode_width::UnicodeWidthChar;

/// One Shift and arrow press, naming where a selection's free end goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionStep {
    /// A character back, onto the end of the line above at a line's start
    Left,
    /// A character on, onto the start of the next line at a line's end
    Right,
    /// The line above, at the same column or its end if shorter
    Up,
    /// The line below, at the same column or its end if shorter
    Down,
    /// The start of the line
    LineStart,
    /// The end of the line's text
    LineEnd,
}

/// Where `point` lands after `step`, over `lines` of plain text.
///
/// A line ends at its last character: the padding a transcript row is drawn with is not text, so
/// a step never walks into it, and an end parked past the text (a triple-click's whole line) is
/// brought back to it first.
pub fn step_point(lines: &[String], point: TextPoint, step: SelectionStep) -> TextPoint {
    let Some(line) = lines.get(point.line) else {
        return point;
    };
    let point = TextPoint::new(point.line, point.column.min(text_end(line)));
    let last = lines.len() - 1;
    match step {
        SelectionStep::Left | SelectionStep::Right => sideways(lines, point, step),
        SelectionStep::Up if point.line > 0 => to_line(lines, point.line - 1, point.column),
        SelectionStep::Down if point.line < last => to_line(lines, point.line + 1, point.column),
        SelectionStep::Up | SelectionStep::Down => point,
        SelectionStep::LineStart => TextPoint::new(point.line, 0),
        SelectionStep::LineEnd => TextPoint::new(point.line, text_end(line)),
    }
}

/// A character back or on from `point`, already inside its line's text, wrapping between lines.
fn sideways(lines: &[String], point: TextPoint, step: SelectionStep) -> TextPoint {
    let line = &lines[point.line];
    match step {
        SelectionStep::Left if point.column == 0 && point.line > 0 => {
            TextPoint::new(point.line - 1, text_end(&lines[point.line - 1]))
        }
        SelectionStep::Left => TextPoint::new(point.line, column_before(line, point.column)),
        _ if point.column == text_end(line) && point.line + 1 < lines.len() => {
            TextPoint::new(point.line + 1, 0)
        }
        _ => TextPoint::new(point.line, column_after(line, point.column)),
    }
}

/// `column` on line `line`, or that line's end when it is shorter.
fn to_line(lines: &[String], line: usize, column: usize) -> TextPoint {
    TextPoint::new(line, column.min(text_end(&lines[line])))
}

/// The cell just past a line's last character.
fn text_end(line: &str) -> usize {
    cell_width(line.trim_end())
}

/// Where the character before `column` starts.
fn column_before(line: &str, column: usize) -> usize {
    let mut start = 0;
    for width in line.chars().map(|character| character.width().unwrap_or(0)) {
        if start + width >= column {
            return start;
        }
        start += width;
    }
    start
}

/// Where the character at `column` ends, or `column` at the end of the text.
fn column_after(line: &str, column: usize) -> usize {
    let mut end = 0;
    for width in line.chars().map(|character| character.width().unwrap_or(0)) {
        end += width;
        if end > column {
            return end.min(text_end(line));
        }
    }
    column
}

#[cfg(test)]
mod tests;
