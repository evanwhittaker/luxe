use crate::app::Msg;
use crate::fixture::{seed_file_view, seeded_app, FixtureScenario};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn press(app: &mut crate::app::App, code: KeyCode, modifiers: KeyModifiers) {
    app.update(Msg::Key(KeyEvent::new(code, modifiers)));
}

fn previewing() -> crate::app::App {
    let mut app = seeded_app("nord", FixtureScenario::ChatIdle).expect("chat fixture");
    let text: String = (1..=40).map(|n| format!("line {n}\n")).collect();
    seed_file_view(&mut app, "src/lib.rs", &text, None);
    app.changes.as_ref().expect("a preview").viewport_rows.set(10);
    app
}

/// The walk: Ctrl+T's message takes the keyboard from the composer to the tree to the preview,
/// Tab trades it back to the tree, and Esc gives it to the composer.
#[test]
fn the_keyboard_walks_from_the_composer_to_the_tree_to_the_preview() {
    let mut app = previewing();
    app.update(Msg::FocusFiles);
    assert!(app.files.focused);
    app.update(Msg::FocusFiles);
    let view = app.changes.as_ref().expect("a preview");
    assert!(view.focused && !app.files.focused);
    assert_eq!(view.cursor, Some(1), "the cursor starts on the first line showing");

    press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    assert!(app.files.focused && !app.changes.as_ref().expect("a preview").focused);
    press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    assert!(app.changes.as_ref().expect("a preview").focused);
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert!(!app.changes.as_ref().expect("still open").focused && !app.files.focused);
}

/// The cursor moves line by line and the view follows it; Shift, or the capitals, grow a pick
/// from where it stood; a plain move drops the pick.
#[test]
fn the_cursor_moves_scrolls_and_picks() {
    let mut app = previewing();
    app.update(Msg::FocusPeek);
    for _ in 0..12 {
        press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
    }
    let view = app.changes.as_ref().expect("a preview");
    assert_eq!(view.cursor, Some(13));
    assert_eq!(view.scroll, 3, "scrolled the least that keeps line 13 on a 10-row screen");

    press(&mut app, KeyCode::Char('J'), KeyModifiers::SHIFT);
    press(&mut app, KeyCode::Down, KeyModifiers::SHIFT);
    assert_eq!(app.changes.as_ref().expect("a preview").selected_lines(), Some((13, 15)));
    press(&mut app, KeyCode::Char('k'), KeyModifiers::NONE);
    assert_eq!(app.changes.as_ref().expect("a preview").selection, None);

    press(&mut app, KeyCode::Char('G'), KeyModifiers::SHIFT);
    assert_eq!(app.changes.as_ref().expect("a preview").cursor, Some(40));
    press(&mut app, KeyCode::Char('g'), KeyModifiers::NONE);
    assert_eq!(app.changes.as_ref().expect("a preview").cursor, Some(1));
}

/// Enter asks about the pick, or the line the cursor is on, and hands the keyboard to the
/// composer, where the question is typed; a letter the pane does not know goes there too.
#[test]
fn enter_asks_about_the_cursor_line_and_typing_goes_to_the_composer() {
    let mut app = previewing();
    app.update(Msg::FocusPeek);
    press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.editor.current_input().contains("src/lib.rs:2"), "{}", app.editor.current_input());
    assert!(!app.changes.as_ref().expect("still open").focused);

    app.update(Msg::FocusPeek);
    press(&mut app, KeyCode::Char('w'), KeyModifiers::NONE);
    assert!(!app.changes.as_ref().expect("still open").focused);
    assert!(app.editor.current_input().ends_with('w'), "{}", app.editor.current_input());
}

/// `v` is visual-line mode: every move after it grows the pick, `o` swaps which end moves,
/// `y` copies and leaves the mode, and `Esc` drops it. Before, a move after `v` dropped the
/// pick, which is what a plain move does and not what anyone pressing `v` meant.
#[test]
fn v_keeps_growing_the_pick_until_it_is_used_or_dropped() {
    let mut app = previewing();
    app.update(Msg::FocusPeek);
    press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('v'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Down, KeyModifiers::NONE);
    let view = app.changes.as_ref().expect("a preview");
    assert!(view.visual);
    assert_eq!(view.selected_lines(), Some((2, 4)));

    press(&mut app, KeyCode::Char('o'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('k'), KeyModifiers::NONE);
    assert_eq!(app.changes.as_ref().expect("a preview").selected_lines(), Some((1, 4)));

    press(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
    let view = app.changes.as_ref().expect("a preview");
    assert!(!view.visual && view.selection.is_none(), "a yank ends the mode");
    press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(app.changes.as_ref().expect("a preview").selection, None, "plain moves again");

    press(&mut app, KeyCode::Char('V'), KeyModifiers::SHIFT);
    press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    let view = app.changes.as_ref().expect("a preview");
    assert!(!view.visual && view.selection.is_none() && view.focused, "Esc drops it, stays");
}

/// Esc drops dragged-over text before it gives up the keyboard, as it drops a line pick.
#[test]
fn esc_drops_a_character_selection_and_keeps_the_keyboard() {
    use crate::selection::{SelectionSpan, TextPoint};
    let mut app = previewing();
    app.update(Msg::FocusPeek);
    let view = app.changes.as_mut().expect("a preview");
    view.begin_cells(TextPoint::new(0, 0), crate::mouse::PeekSide::Whole, SelectionSpan::Caret);
    view.extend_cells(TextPoint::new(0, 4));
    assert!(view.cells_picked());

    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    let view = app.changes.as_ref().expect("a preview");
    assert!(view.cells.is_none() && view.focused, "the text goes, the keyboard stays");
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert!(!app.changes.as_ref().expect("a preview").focused, "then Esc gives it back");
}

/// `Ctrl+D` and `Ctrl+U` move the cursor half a page, and in visual mode grow the pick by it.
#[test]
fn ctrl_d_and_ctrl_u_move_half_a_page() {
    let mut app = previewing();
    app.update(Msg::FocusPeek);
    press(&mut app, KeyCode::Char('d'), KeyModifiers::CONTROL);
    // ten rows on screen: a page is nine, half of it four
    assert_eq!(app.changes.as_ref().expect("a preview").cursor, Some(5));
    press(&mut app, KeyCode::Char('v'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('d'), KeyModifiers::CONTROL);
    assert_eq!(app.changes.as_ref().expect("a preview").selected_lines(), Some((5, 9)));
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('u'), KeyModifiers::CONTROL);
    let view = app.changes.as_ref().expect("a preview");
    assert_eq!(view.cursor, Some(5));
    assert!(view.focused, "the keyboard stays in the pane");
}
