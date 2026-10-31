//! Peek cursor — the line the keyboard stands on in the Changes pane's diff, and how a press puts
//! it there.

use super::*;
use ratatui::style::Color;

/// A pane on `one_hunk_file`, `width` cells wide, drawn once. Rows: the header, `fn sum() {`, the
/// deletion, the addition.
fn diff_pane(width: u16, height: u16) -> (App, TestBackend, Buffer) {
    let backend = test_backend(width, height);
    let (mut app, mut rx) = create_app_with_engine();
    app.update(Msg::TerminalResized { width, height });
    app.dock.hide();
    app.workspace = Some(changed_workspace("peek-cursor", 2));
    let path = "src/module00/file00.rs";
    open_changes_with(&mut app, &mut rx, path, Some(one_hunk_file(path)));
    let buffer = render_app(&mut app, &backend);
    (app, backend, buffer)
}

fn press(app: &mut App, code: KeyCode) {
    app.update(Msg::Key(KeyEvent::new(code, KeyModifiers::NONE)));
}

/// The background under `needle`'s first cell.
fn background_at(buffer: &Buffer, needle: &str, width: u16, height: u16) -> Option<Color> {
    let row = row_containing(buffer, needle, width, height);
    buffer[(column_of(&row_text(buffer, row, width), needle), row)].bg.into()
}

/// With the keyboard in the pane the diff has a cursor: `j` walks it down, its row's numbers wear
/// the File view's `›`, and its band lifts the deletion's tint without losing it to a plain band.
#[test]
fn the_unified_diff_marks_and_bands_the_cursor_row() {
    let (width, height) = (90, 24);
    let (mut app, backend, before) = diff_pane(width, height);
    assert!(!buffer_to_string(&before).contains('›'), "no cursor while the keyboard is away");

    app.update(Msg::FocusPeek);
    press(&mut app, KeyCode::Char('j'));
    press(&mut app, KeyCode::Char('j'));
    let buffer = render_app(&mut app, &backend);
    let row = row_containing(&buffer, "a + b", width, height);
    assert!(peek_diff_text(&buffer, row, &app, width).contains('›'), "the deletion is marked");
    let other = row_containing(&buffer, "a - b", width, height);
    assert!(!peek_diff_text(&buffer, other, &app, width).contains('›'), "and only it");

    let deletion = app.theme_styles.diff_del_bg.bg;
    let band = background_at(&buffer, "a + b", width, height);
    assert_eq!(background_at(&before, "a + b", width, height), deletion, "tinted before");
    assert_ne!(band, deletion, "the cursor lifts the tint");
    assert_ne!(band, app.theme_styles.cursor_line_bg().bg, "rather than replacing it");
    assert!(content_has(&buffer, "j k line"), "the footer says the keys move a line");
}

/// With the keyboard in the pane, the layout chords still work, so the footer still names them
/// where there's room: after the movement keys and before the way back. The break this catches:
/// the focused footer listing only the pane's own keys while wrap and side by side went unsaid.
#[test]
fn the_focused_footer_keeps_the_layout_keys_where_they_fit() {
    let footer = |width: u16| {
        let (mut app, backend, _) = diff_pane(width, 24);
        app.update(Msg::FocusPeek);
        let buffer = render_app(&mut app, &backend);
        let row = row_containing(&buffer, "esc back", width, 24);
        peek_diff_text(&buffer, row, &app, width).trim().to_string()
    };
    let wide = footer(200);
    assert!(wide.contains("ctrl+b unified diff · alt+w stop wrapping · tab files"), "{wide}");
    assert!(wide.starts_with("j k line") && wide.ends_with("esc back"), "{wide}");
    let narrow = footer(60);
    assert!(
        !narrow.contains("alt+w") && narrow.contains("j k line"),
        "layout goes first: {narrow}"
    );
}

/// Side by side, the cursor on either half of a pair bands the whole pair.
#[test]
fn the_split_diff_bands_the_pair_the_cursor_is_on() {
    let (width, height) = (200, 24);
    let (mut app, backend, before) = diff_pane(width, height);
    assert!(split_row(&before, "fn sum() {", &app, width, height), "the diff is side by side");

    app.update(Msg::FocusPeek);
    for _ in 0..3 {
        press(&mut app, KeyCode::Char('j'));
    }
    let buffer = render_app(&mut app, &backend);
    let added = background_at(&buffer, "a - b", width, height);
    let removed = background_at(&buffer, "a + b", width, height);
    assert_ne!(added, app.theme_styles.diff_add_bg.bg, "the addition, under the cursor");
    assert_ne!(removed, app.theme_styles.diff_del_bg.bg, "and the deletion beside it");
    let context = background_at(&buffer, "fn sum", width, height);
    assert_eq!(context, background_at(&before, "fn sum", width, height), "the row above is not");
}

/// A press on a line of the diff turns the reader to the pane: the keyboard follows with the cursor
/// on the line pressed, so the next `j` moves from there.
#[test]
fn a_press_on_a_diff_line_gives_the_pane_the_keyboard_there() {
    let (width, height) = (90, 24);
    let (mut app, backend, buffer) = diff_pane(width, height);
    let row = row_containing(&buffer, "fn sum", width, height);
    let column = column_of(&row_text(&buffer, row, width), "fn sum");
    click_cell(&mut app, column, row, std::time::Instant::now());
    press(&mut app, KeyCode::Char('j'));

    let buffer = render_app(&mut app, &backend);
    let marked = row_containing(&buffer, "a + b", width, height);
    assert!(peek_diff_text(&buffer, marked, &app, width).contains('›'), "one on from the press");
    assert_eq!(app.editor.current_input(), "", "the j went to the pane");
}

fn content_has(buffer: &Buffer, needle: &str) -> bool {
    buffer_to_string(buffer).contains(needle)
}
