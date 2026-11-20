//! Find rendering — the overlay as a centered picker with its own search field.
//!
//! Drawn through [`crate::picker::render_picker`], so its tabs, rows, scrollbar and click targets
//! are the `@` picker's. With a preview pane beside or under the list, a text hit's row is just
//! its `path:line` and the pane shows the line in its file; without room for one, the row carries
//! the line too, its matches lit. A file row is the `@` picker's own. The frame is the same size
//! with rows or without, so it holds still as a search fills it; an empty tab says what it needs
//! across the whole of it, with no pane beside.

use super::{FindState, FindTab};
use crate::glyphs::Icon;
use crate::picker::{render_picker, Edge, PickerRow, PickerView, PreviewPane};
use crate::theme::styles::ThemeStyles;
use luxe_core::project_search::SearchHit;
use ratatui::layout::Rect;

/// Renders Find centered over `area`.
pub(crate) fn render_find(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    state: &FindState,
    theme: &ThemeStyles,
    hit: &std::cell::RefCell<crate::mouse::HitRegions>,
) {
    let words = Words::of(state);
    let tabs: Vec<&str> = words.tabs.iter().map(String::as_str).collect();
    let modal = crate::picker::preview_modal(area);
    let body = list_body(modal, theme);
    // An empty list asks for the body's rows, which holds the frame at the modal's size
    let fill = usize::from(body.height);
    if state.row_count() == 0 {
        let view = picker_view(state, (&words, &tabs), false, fill, theme);
        return render_picker(frame, modal, Edge::Below, theme, hit, view);
    }
    let lines = |width, height| super::preview::preview_lines(state, width, height, theme);
    let list_width = crate::picker::preview_list_width(modal);
    let bare = crate::picker::preview_fits(body, list_width);
    let preview = Some(PreviewPane { list_width, lines: &lines });
    let view = PickerView { preview, ..picker_view(state, (&words, &tabs), bare, fill, theme) };
    render_picker(frame, modal, Edge::Below, theme, hit, view);
}

/// The body under the overlay's search field and tabs, where the list and the preview go.
fn list_body(modal: Rect, theme: &ThemeStyles) -> Rect {
    let block = crate::widgets::titled_panel(Icon::Search.glyph(), "Find", theme);
    let body = crate::widgets::footed_body(&block, modal);
    let header = crate::picker::header_rows(true, true).min(body.height);
    Rect { y: body.y + header, height: body.height - header, ..body }
}

/// Everything the overlay says that depends on its state, made once per frame.
struct Words {
    tabs: Vec<String>,
    empty: String,
    hint: String,
    count: Option<String>,
}

impl Words {
    fn of(state: &FindState) -> Self {
        let (empty, hint, count) = (empty_message(state), hint(state), count(state));
        Self { tabs: tab_labels(), empty, hint, count }
    }
}

/// Find as a picker, without its pane. Args: `(state, (words, tabs), bare, rows, theme)`, where
/// `bare` drops the line from a hit's row and `rows` is the list's height.
fn picker_view<'a>(
    state: &'a FindState,
    (words, tabs): (&'a Words, &'a [&'a str]),
    bare: bool,
    rows: usize,
    theme: &'a ThemeStyles,
) -> PickerView<'a, impl Fn(usize) -> PickerRow + 'a, fn(usize) -> crate::mouse::MouseTarget> {
    let open = FindTab::ALL.iter().position(|tab| *tab == state.tab()).unwrap_or(0);
    PickerView {
        glyph: Icon::Search.glyph(),
        title: "Find",
        empty_message: &words.empty,
        item_count: state.row_count(),
        selected: state.selected(),
        row: move |position| row(state, position, bare, theme),
        hit_target: crate::mouse::MouseTarget::FindRow,
        hint: Some(&words.hint),
        query: Some(state.query()),
        count: words.count.as_deref(),
        tabs: Some((tabs, open)),
        divider: None,
        detail: None,
        rows: Some(rows),
        list: crate::mouse::OverlayList::Find,
        preview: None,
    }
}

fn tab_labels() -> Vec<String> {
    FindTab::ALL
        .iter()
        .map(|tab| {
            let icon = match tab {
                FindTab::Text => Icon::Grep,
                FindTab::Files => Icon::FileMenu,
                FindTab::Conversation => Icon::Conversation,
            };
            crate::glyphs::icon_label(icon.glyph(), tab.label())
        })
        .collect()
}

/// The row at `position`: `bare` when a preview pane shows the line, so the row needn't.
fn row(state: &FindState, position: usize, bare: bool, theme: &ThemeStyles) -> PickerRow {
    match state.tab() {
        FindTab::Text => state.hits().get(position).map(|hit| match bare {
            true => location_row(hit),
            false => hit_row(hit, theme),
        }),
        FindTab::Files => state.files().get(position).map(|file| file_row(file, theme)),
        FindTab::Conversation => state.messages().get(position).map(message_row),
    }
    .unwrap_or_default()
}

/// A text hit as where it is, `path:line` and the scopes around it, for a list beside a pane that
/// shows the line but not which function holds it.
fn location_row(hit: &SearchHit) -> PickerRow {
    PickerRow {
        label: format!("{}:{}", hit.path, hit.line),
        detail: hit.scope.clone(),
        path: true,
        ..PickerRow::default()
    }
}

/// A file row, as the `@` picker draws it: the path, its matches lit, and its git mark.
fn file_row(file: &crate::ref_provider::Candidate, theme: &ThemeStyles) -> PickerRow {
    let status = file.status.map(|status| crate::view::status_mark(status, theme));
    PickerRow {
        label: file.label.clone(),
        detail: status.map(|(mark, _)| mark.to_string()),
        label_matched: file.label_matched.clone(),
        path: true,
        detail_style: status.map(|(_, style)| style),
        ..PickerRow::default()
    }
}

/// A message: who said it, then the line of it that matched, lit.
fn message_row(hit: &super::messages::MessageHit) -> PickerRow {
    PickerRow {
        label: hit.role.to_string(),
        detail: Some(hit.line.clone()),
        detail_matched: hit.matched.clone(),
        ..PickerRow::default()
    }
}

/// A text hit: `path:line`, then the line with its indent dropped and its matches lit.
fn hit_row(hit: &SearchHit, theme: &ThemeStyles) -> PickerRow {
    let (detail, detail_matched) = hit_line(hit);
    PickerRow {
        label: format!("{}:{}", hit.path, hit.line),
        detail: Some(detail),
        label_matched: Vec::new(),
        detail_matched,
        path: true,
        // The line is code to read, not a note beside the path
        detail_style: Some(theme.text),
        detail_right: false,
    }
}

/// The hit's line without its indent, tabs drawn as spaces, and which characters of it matched.
fn hit_line(hit: &SearchHit) -> (String, Vec<usize>) {
    let trimmed = hit.text.trim_start();
    let indent = hit.text.len() - trimmed.len();
    let inside = |byte: usize| {
        hit.matches.iter().any(|range| (range.start..range.end).contains(&(indent + byte)))
    };
    let matched = trimmed
        .char_indices()
        .enumerate()
        .filter(|(_, (byte, _))| inside(*byte))
        .map(|(index, _)| index)
        .collect();
    (trimmed.replace('\t', " "), matched)
}

/// What the list says when it has no rows, which on each tab means something different.
fn empty_message(state: &FindState) -> String {
    let query = state.query();
    match state.tab() {
        FindTab::Text if query.is_empty() => "Type to search every file in the project".into(),
        FindTab::Text => text_empty(state),
        FindTab::Files => "No matching files".into(),
        FindTab::Conversation if query.trim().is_empty() => {
            "Type to search what was said in the conversation".into()
        }
        FindTab::Conversation => "No messages say that".into(),
    }
}

fn text_empty(state: &FindState) -> String {
    if let Some(error) = state.error() {
        return capitalized(error);
    }
    match state.results() {
        Some(results) if results.walk_capped => {
            format!("No matches in the first {} files", results.files_searched)
        }
        Some(_) if !state.is_searching() => "No matches".into(),
        _ => "Searching…".into(),
    }
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| first.to_uppercase().chain(chars).collect())
}

/// The keys the open tab answers, with the text search's options showing their settings.
///
/// The options lead because a narrow footer drops what comes last, and a hidden setting is one
/// that changes the rows without saying so; Enter opening a row needs no telling.
fn hint(state: &FindState) -> String {
    match state.tab() {
        FindTab::Text => {
            let options = state.options();
            let on = |flag: bool| if flag { "on" } else { "off" };
            let case = match options.case_sensitive {
                None => "smart",
                Some(true) => "exact",
                Some(false) => "any",
            };
            format!(
                "alt+r regex {} · alt+c case {case} · alt+i ignored {} · enter open · alt+m mention",
                on(options.regex),
                on(options.include_ignored),
            )
        }
        FindTab::Files => "enter open · alt+m mention · tab switch · esc close".into(),
        FindTab::Conversation => "enter go to · tab switch · esc close".into(),
    }
}

/// Where the cursor is among the rows, how many there were past the ones listed, or that a
/// search is still out.
fn count(state: &FindState) -> Option<String> {
    if state.tab() == FindTab::Text && state.is_searching() {
        return Some("searching…".into());
    }
    let rows = state.row_count();
    if rows == 0 {
        return None;
    }
    let at = state.selected() + 1;
    let total = state.results().map_or(rows, |results| results.total);
    Some(match state.tab() == FindTab::Text && total > rows {
        true => format!("{at}/{rows} of {total}"),
        false => format!("{at}/{rows}"),
    })
}

#[cfg(test)]
mod tests;
