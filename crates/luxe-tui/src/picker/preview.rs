//! Picker preview — a pane that shows the row under the cursor in its context.
//!
//! The detail pane beside the palette's list says a few lines *about* a row; this one *shows* it:
//! the file around a search hit, the turns of a session. It fills the picker's height, so a picker
//! with one asks for every row it is given, and the list keeps only the width its rows need. Side
//! by side where there is width, the pane under the list where there is only height, and left out
//! where there is neither, so the rows have to say enough on their own.

use super::layout::PANE_GUTTER;
use crate::theme::styles::ThemeStyles;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Fewest columns a pane beside the list is worth drawing in.
const BESIDE_MIN_WIDTH: u16 = 40;

/// Fewest rows a pane under the list is worth drawing in.
const BELOW_MIN_HEIGHT: u16 = 6;

/// Fewest rows the list keeps when the pane sits under it.
const BELOW_LIST_ROWS: u16 = 3;

/// The largest a picker with a preview grows: most of a big screen, short of the edges.
const MODAL_MAX: (u16, u16) = (180, 46);

/// The list's share of the width beside a preview, in fifths, and the columns it keeps either way.
const LIST_FIFTHS: u16 = 2;
const LIST_COLUMNS: (u16, u16) = (32, 56);

/// Where a picker with a preview draws over `area`: most of it, centered both ways, since the pane
/// shows as much of its row's context as it is given.
pub(crate) fn preview_modal(area: Rect) -> Rect {
    let width = area.width.saturating_sub(4).min(MODAL_MAX.0);
    let height = area.height.saturating_sub(2).min(MODAL_MAX.1);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

/// The columns a list keeps beside the preview in `modal`: enough for a name or a path and a
/// short note, the rest left to the pane.
pub(crate) fn preview_list_width(modal: Rect) -> u16 {
    let inner = modal.width.saturating_sub(2);
    (inner * LIST_FIFTHS / 5).clamp(LIST_COLUMNS.0, LIST_COLUMNS.1)
}

/// The pane, as a picker that has one describes it.
pub struct PreviewPane<'a> {
    /// Columns the list keeps when the pane sits beside it; the pane takes the rest
    pub list_width: u16,
    /// The pane's lines for the row under the cursor, for a pane `width` by `height` cells. They
    /// are drawn as given, clipped and never wrapped, so code keeps its lines.
    pub lines: &'a dyn Fn(u16, u16) -> Vec<Line<'static>>,
}

/// Where the pane goes, and the rule that sets it off from the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PanePlace {
    /// Right of the list, a rule down the column between
    Beside { rule_x: u16, pane: Rect },
    /// Under the list, a rule across the row between
    Below { rule_y: u16, pane: Rect },
}

/// Divides a picker's `body` between its list and a preview pane, the list `list_width` wide
/// when the pane fits beside it.
pub(crate) fn place(body: Rect, list_width: u16) -> (Rect, Option<PanePlace>) {
    if body.width >= list_width + PANE_GUTTER + 1 + BESIDE_MIN_WIDTH {
        let rule_x = body.x + list_width;
        let pane_x = rule_x + PANE_GUTTER;
        // a cell short of the frame, as the detail pane keeps
        let pane = Rect { x: pane_x, width: body.right() - pane_x - 1, ..body };
        return (Rect { width: list_width, ..body }, Some(PanePlace::Beside { rule_x, pane }));
    }
    if body.height >= BELOW_LIST_ROWS + 1 + BELOW_MIN_HEIGHT {
        // a third never crowds the pane under its floor once the body is this tall
        let list_rows = (body.height / 3).max(BELOW_LIST_ROWS);
        let rule_y = body.y + list_rows;
        let pane = Rect { y: rule_y + 1, height: body.bottom() - rule_y - 1, ..body };
        let list = Rect { height: list_rows, ..body };
        return (list, Some(PanePlace::Below { rule_y, pane }));
    }
    (body, None)
}

/// Whether a picker whose body is `body` will show its preview, so its rows know whether they
/// can leave the reading to the pane.
pub(crate) fn fits(body: Rect, list_width: u16) -> bool {
    place(body, list_width).1.is_some()
}

/// Draws the rule and the pane's lines at `place`.
pub(crate) fn render_preview(
    f: &mut Frame,
    place: PanePlace,
    lines: &dyn Fn(u16, u16) -> Vec<Line<'static>>,
    theme: &ThemeStyles,
) {
    let pane = render_rule(f, place, theme);
    f.render_widget(Paragraph::new(lines(pane.width, pane.height)), pane);
}

/// Draws the rule between the list and the pane, and returns the pane's area.
fn render_rule(f: &mut Frame, place: PanePlace, theme: &ThemeStyles) -> Rect {
    match place {
        PanePlace::Beside { rule_x, pane } => {
            let rule: Vec<Line> = (0..pane.height)
                .map(|_| Line::from(Span::styled("│", theme.border_muted)))
                .collect();
            f.render_widget(Paragraph::new(rule), Rect::new(rule_x, pane.y, 1, pane.height));
            pane
        }
        PanePlace::Below { rule_y, pane } => {
            let rule =
                Line::from(Span::styled("─".repeat(usize::from(pane.width)), theme.border_muted));
            f.render_widget(Paragraph::new(rule), Rect::new(pane.x, rule_y, pane.width, 1));
            pane
        }
    }
}

#[cfg(test)]
mod tests;
