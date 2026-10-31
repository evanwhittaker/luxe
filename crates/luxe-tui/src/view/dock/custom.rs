//! Dock custom sections — the blocks a `dock.custom` program feeds, drawn like the built-ins.
//!
//! Rows come from core already sanitized and capped, so this only fits them to the rail: a label
//! in the telemetry column, the value cut to the room left, and every row exactly one line, since
//! a row that wrapped would move whatever is pinned under it.

use super::anchors::DockSection;
use super::chrome::custom_heading_line;
use super::telemetry::{telemetry_label_width, telemetry_value_width, WIDEST_TELEMETRY_LABEL};
use crate::app::App;
use crate::theme::styles::Depth;
use crate::widgets::truncate_end;
use luxe_core::config::{CustomDockSection, DockPlace, DockSectionName};
use luxe_core::custom_dock::{DockRow, DockSpan};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

/// The custom sections placed in the middle, in list order, each after a blank as the built-ins
/// there are. One with nothing to show yet is left out.
pub(super) fn middle_sections(
    app: &App,
    sections: &[DockSectionName],
    row: usize,
) -> Vec<(DockSectionName, DockSection)> {
    let column = telemetry_label_width(row, WIDEST_TELEMETRY_LABEL);
    let built = |section: &DockSectionName| {
        let DockSectionName::Other(name) = section else { return None };
        let lines = placed_lines(app, name, DockPlace::Middle, row, column);
        let lines = (!lines.is_empty()).then(|| [vec![Line::default()], lines].concat())?;
        Some((section.clone(), DockSection { lines, ..DockSection::default() }))
    };
    sections.iter().filter_map(built).collect()
}

/// The block for `name` when it's defined and placed at `place`, else nothing.
pub(super) fn placed_lines(
    app: &App,
    name: &str,
    place: DockPlace,
    row: usize,
    column: usize,
) -> Vec<Line<'static>> {
    match app.config.dock.custom_section(name) {
        Some(definition) if definition.place() == place => {
            custom_lines(name, definition, app, row, column)
        }
        _ => Vec::new(),
    }
}

/// The block for the custom section `name`, or nothing until its program first answers. A program
/// that printed nothing has nothing to show, so its section goes rather than leave a bare heading.
fn custom_lines(
    name: &str,
    definition: &CustomDockSection,
    app: &App,
    row: usize,
    column: usize,
) -> Vec<Line<'static>> {
    let Some(answer) = app.status.custom_dock.get(name) else { return Vec::new() };
    if answer.rows.is_empty() && answer.failure.is_none() {
        return Vec::new();
    }
    let title = truncate_end(&definition.heading(name), row.saturating_sub(4));
    let mut lines = vec![custom_heading_line(title, depth(definition.place()), app)];
    lines.extend(answer.rows.iter().map(|dock_row| row_line(dock_row, row, column, app)));
    if let Some(failure) = &answer.failure {
        lines.push(failure_line(failure, row, app));
    }
    lines
}

/// A middle section sits with the work in flight, a bottom one with the ambient accounting.
fn depth(place: DockPlace) -> Depth {
    match place {
        DockPlace::Middle => Depth::Raised,
        _ => Depth::Recessed,
    }
}

/// One row: the label padded to the telemetry column when there is one, then the spans.
fn row_line(dock_row: &DockRow, row: usize, column: usize, app: &App) -> Line<'static> {
    let styles = &app.theme_styles;
    let Some(label) = &dock_row.label else {
        return Line::from(fit_spans(&dock_row.spans, row, app));
    };
    let label = truncate_end(label, column.saturating_sub(1));
    let pad = " ".repeat(column.saturating_sub(label.width()));
    let mut spans = vec![Span::styled(format!("{label}{pad}"), styles.muted.patch(styles.tool_bg))];
    spans.extend(fit_spans(&dock_row.spans, telemetry_value_width(row, column), app));
    Line::from(spans)
}

/// The spans in their inks, cut where they pass `room` cells.
fn fit_spans(spans: &[DockSpan], room: usize, app: &App) -> Vec<Span<'static>> {
    let mut left = room;
    let mut fitted = Vec::new();
    for span in spans {
        if left == 0 {
            break;
        }
        let text = truncate_end(&span.text, left);
        left = left.saturating_sub(text.width());
        let style = app.theme_styles.custom_ink(span.ink).patch(app.theme_styles.tool_bg);
        fitted.push(Span::styled(text, style));
    }
    fitted
}

/// The reason the last run drew nothing, dim under whatever rows are kept.
fn failure_line(failure: &str, row: usize, app: &App) -> Line<'static> {
    let text = truncate_end(&format!("last run: {failure}"), row);
    Line::from(Span::styled(text, app.theme_styles.dim.patch(app.theme_styles.tool_bg)))
}
