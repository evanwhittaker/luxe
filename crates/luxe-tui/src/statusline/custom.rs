//! Statusline custom modules — the segment a `statusline.custom` program feeds.
//!
//! The program's first row, label then spans, cut to a fixed width so one program can't push the
//! anchors off the bar. Rows arrive from core already sanitized; this only fits them. A program
//! that prints nothing hides its module, which is how one says "nothing to report".

use super::{Ink, Segment, SegmentKind};
use crate::app::App;
use crate::widgets::truncate_end;
use luxe_core::custom_dock::{DockInk, DockRow};
use ratatui::style::Color;
use unicode_width::UnicodeWidthStr;

/// Most cells a custom module takes: about a context reading and a cost side by side.
pub(crate) const MAX_CUSTOM_WIDTH: usize = 32;

/// The segment for the custom module `name` at `index` in its side's list, or nothing until its
/// program first answers. A failure keeps the last row; with none to keep, the bar says why, since
/// a module that silently never appears is the hardest thing to debug.
pub(super) fn custom_segment(app: &App, name: &str, index: usize) -> Option<Segment> {
    let answer = app.status.custom_statusline.get(name)?;
    let parts = match answer.rows.first() {
        Some(row) => row_parts(row, app),
        None => vec![(format!("{name}: {}", answer.failure.as_deref()?), Ink::Neutral)],
    };
    let parts = fit(parts, MAX_CUSTOM_WIDTH);
    Some(Segment { kind: SegmentKind::Custom(index), parts, emphasized: false })
}

/// A row as the bar's parts: the label, then each span in its ink.
fn row_parts(row: &DockRow, app: &App) -> Vec<(String, Ink)> {
    let label = row.label.iter().map(|label| (format!("{label} "), Ink::Neutral));
    let spans = row.spans.iter().map(|span| (span.text.clone(), ink(span.ink, app)));
    label.chain(spans).collect()
}

/// Body text follows its surface like every other neutral part on the bar; any other ink is a
/// signal the program chose, so it keeps its color.
fn ink(ink: DockInk, app: &App) -> Ink {
    match ink {
        DockInk::Text | DockInk::Unknown => Ink::Neutral,
        other => Ink::Signal(app.theme_styles.custom_ink(other).fg.unwrap_or(Color::Reset)),
    }
}

/// `parts` cut where they pass `room` cells, with the cut marked.
fn fit(parts: Vec<(String, Ink)>, room: usize) -> Vec<(String, Ink)> {
    let mut left = room;
    let mut fitted = Vec::new();
    for (text, ink) in parts {
        if left == 0 {
            break;
        }
        let text = truncate_end(&text, left);
        left = left.saturating_sub(text.width());
        fitted.push((text, ink));
    }
    fitted
}
