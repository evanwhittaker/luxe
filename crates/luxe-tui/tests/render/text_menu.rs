//! Text menu — what a right-click on the composer or the transcript's text offers.

use super::*;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use luxe_tui::mouse::TextSurface;

/// Right-clicks `(column, row)` the way the event loop does.
fn right_click(app: &mut App, column: u16, row: u16) {
    let kind = MouseEventKind::Down(MouseButton::Right);
    let action = route_mouse(app, &MouseEvent { kind, column, row, modifiers: KeyModifiers::NONE });
    apply(app, action);
}

/// The open menu's labels, top to bottom; empty when none is open.
fn menu_labels(app: &App) -> Vec<String> {
    let items = app.overlays.context_menu.as_ref().map(|menu| menu.items.clone());
    items.unwrap_or_default().into_iter().map(|item| item.label).collect()
}

/// Chooses the open menu's item named `label`.
fn choose(app: &mut App, label: &str) {
    let index = menu_labels(app).iter().position(|it| it == label);
    let index = index.unwrap_or_else(|| panic!("no {label:?} in {:?}", menu_labels(app)));
    app.update(Msg::ChooseMenuItem(index));
}

/// A cell of the composer's first drawn row, found from the regions the last frame published.
fn composer_cell(app: &App, width: u16, height: u16) -> (u16, u16) {
    let hit = app.hit.borrow();
    let cells = (0..height).flat_map(|row| (0..width).map(move |column| (column, row)));
    let mut cells = cells;
    let found = cells.find(|&(column, row)| {
        hit.text_at(column, row).is_some_and(|it| it.surface == TextSurface::Composer)
    });
    found.expect("the composer publishes its rows")
}

#[test]
fn a_right_click_on_an_empty_composer_offers_paste() {
    let backend = test_backend(60, 12);
    let mut app = create_app();
    render_app(&mut app, &backend);
    let (column, row) = composer_cell(&app, 60, 12);

    right_click(&mut app, column, row);
    assert_eq!(menu_labels(&app), ["Paste"], "nothing selected, so nothing to copy");
    let drawn = buffer_to_string(&render_app(&mut app, &backend));
    assert!(drawn.contains("Paste") && drawn.contains("ctrl+v"), "drawn with its key: {drawn}");

    choose(&mut app, "Paste");
    assert!(app.outbox.clipboard_paste, "the clipboard is read, as Ctrl+V reads it");
    assert!(app.overlays.context_menu.is_none(), "and the menu closes");
}

#[test]
fn a_right_click_on_a_composer_selection_offers_copy_too() {
    let backend = test_backend(60, 12);
    let mut app = create_app();
    app.editor.insert_text("draft");
    app.update(Msg::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT)));
    render_app(&mut app, &backend);
    let (column, row) = composer_cell(&app, 60, 12);

    right_click(&mut app, column, row);
    assert_eq!(menu_labels(&app), ["Copy", "Paste"]);
    choose(&mut app, "Copy");
    assert_eq!(app.outbox.copy.as_deref(), Some("t"));
}

#[test]
fn a_right_click_on_a_transcript_selection_offers_copy() {
    let (width, height) = (60, 12);
    let backend = test_backend(width, height);
    let mut app = create_app();
    app.push_convo(ConvoBlock::Assistant("alpha bravo charlie".into()));
    let buffer = render_app(&mut app, &backend);
    let row = row_containing(&buffer, "alpha bravo charlie", width, height);
    let start = column_of(&row_text(&buffer, row, width), "bravo");

    right_click(&mut app, start, row);
    assert!(menu_labels(&app).is_empty(), "no selection, nothing to offer");

    drag_along(&mut app, row, start, start + 5);
    right_click(&mut app, start + 2, row);
    assert_eq!(menu_labels(&app), ["Copy", "Clear the selection"]);
    choose(&mut app, "Copy");
    render_app(&mut app, &backend);
    assert_eq!(app.outbox.copy.as_deref(), Some("bravo"));
    assert!(app.text_selection_is_active(), "copying keeps the selection");
}

#[test]
fn the_menu_can_clear_a_transcript_selection() {
    let (width, height) = (60, 12);
    let backend = test_backend(width, height);
    let mut app = create_app();
    app.push_convo(ConvoBlock::Assistant("alpha bravo charlie".into()));
    let buffer = render_app(&mut app, &backend);
    let row = row_containing(&buffer, "alpha bravo charlie", width, height);
    let start = column_of(&row_text(&buffer, row, width), "bravo");
    let now = std::time::Instant::now();
    click_cell(&mut app, start, row, now);
    click_cell(&mut app, start, row, now);
    render_app(&mut app, &backend);

    right_click(&mut app, start, row);
    choose(&mut app, "Clear the selection");
    assert!(!app.text_selection_is_active());
}
