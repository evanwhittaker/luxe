//! Cursor — the review diff's line cursor, apart from the rows on screen.

use super::*;
use crate::mouse::MouseTarget;

const WIDTH: u16 = 140;
const HEIGHT: u16 = 30;

/// A large review drawn unified, so its first file runs past one screen.
fn unified_review() -> crate::app::App {
    let mut app = seeded_app("nord", FixtureScenario::GuidedReviewLarge).expect("fixture");
    app.guided_review_workspace.layout = luxe_core::config::DiffLayout::Unified;
    app.update(Msg::TerminalResized { width: WIDTH, height: HEIGHT });
    app
}

/// Draws the app, returning each row's text.
fn draw(app: &mut crate::app::App) -> Vec<String> {
    let buffer = draw_buffer(app);
    (0..HEIGHT).map(|y| (0..WIDTH).map(|x| buffer[(x, y)].symbol()).collect()).collect()
}

fn draw_buffer(app: &mut crate::app::App) -> ratatui::buffer::Buffer {
    use ratatui::{backend::TestBackend, Terminal};
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("terminal");
    terminal.draw(|frame| crate::view::render(frame, app)).expect("drawn");
    terminal.backend().buffer().clone()
}

/// The screen row the diff drew unified row `row` on, by the click regions it published.
fn screen_row_of(app: &crate::app::App, row: usize) -> Option<usize> {
    let hit = app.hit.borrow();
    (0..HEIGHT).position(|y| hit.at(WIDTH - 3, y) == Some(&MouseTarget::ReviewDiffRow(row)))
}

fn press(app: &mut crate::app::App, msg: impl Fn() -> Msg, times: usize) {
    for _ in 0..times {
        app.update(msg());
    }
}

/// `j` walks the cursor down the screen while the code stays put, and its row wears the mark.
#[test]
fn line_moves_walk_the_cursor_down_a_still_screen() {
    let mut app = unified_review();
    let before = draw_buffer(&mut app);
    let x = WIDTH - 4;
    let tint = before[(x, screen_row_of(&app, 3).expect("row 3 is drawn") as u16)].bg;
    press(&mut app, || Msg::GuidedReviewLineNext, 3);
    let drawn = draw(&mut app);

    assert_eq!(app.guided_review_workspace.diff_top(), 0, "the code did not move");
    let y = screen_row_of(&app, 3).expect("the cursor's row is on screen");
    assert!(drawn[y].contains('›'), "the cursor's row is marked: {:?}", drawn[y]);
    let other = screen_row_of(&app, 2).expect("the row above is drawn");
    assert!(!drawn[other].contains('›'), "and the row above is not: {:?}", drawn[other]);

    let banded = draw_buffer(&mut app)[(x, y as u16)].bg;
    assert_ne!(banded, tint, "the cursor's band lifts the row's own tint");
    assert_ne!(Some(banded), app.theme_styles.cursor_line_bg().bg, "rather than replacing it");
}

/// Walked past the bottom, the screen scrolls only as far as keeps the cursor's row on it.
#[test]
fn a_cursor_walked_off_the_bottom_scrolls_the_screen_with_it() {
    let mut app = unified_review();
    draw(&mut app);
    press(&mut app, || Msg::GuidedReviewLineNext, 40);
    draw(&mut app);

    let top = app.guided_review_workspace.diff_top();
    assert!(top > 0, "the screen moved down");
    assert!(top < 40, "but not so far the cursor's row went to the top");
    assert!(screen_row_of(&app, 40).is_some(), "and the cursor's row is on it");
    assert!(
        screen_row_of(&app, 40 + 1).is_none_or(|y| y + 1 >= HEIGHT as usize - 2),
        "at the foot"
    );
}

/// A jump lands with the cursor's row at the top, as it always did.
#[test]
fn a_hunk_jump_puts_the_cursor_at_the_top() {
    let mut app = unified_review();
    draw(&mut app);
    press(&mut app, || Msg::GuidedReviewHunkNext, 2);
    draw(&mut app);

    let workspace = &app.guided_review_workspace;
    assert!(workspace.position.line_offset > 0, "the jump moved the cursor");
    assert_eq!(workspace.diff_top(), workspace.position.line_offset);
}

/// The wheel scrolls the code and the cursor rides along on its screen row.
#[test]
fn the_wheel_scrolls_the_code_with_the_cursor_on_its_row() {
    let mut app = unified_review();
    draw(&mut app);
    press(&mut app, || Msg::GuidedReviewLineNext, 3);
    draw(&mut app);
    press(&mut app, || Msg::GuidedReviewWheel { forward: true }, 2);
    draw(&mut app);

    let workspace = &app.guided_review_workspace;
    assert_eq!((workspace.diff_top(), workspace.position.line_offset), (2, 5));
}

/// A click puts the cursor on the row clicked, and the screen holds still under the pointer.
#[test]
fn a_click_moves_the_cursor_and_not_the_screen() {
    let mut app = unified_review();
    draw(&mut app);
    let y = screen_row_of(&app, 6).expect("row 6 is on screen");
    app.update(Msg::GuidedReviewSelectDiffRow(6));
    draw(&mut app);

    assert_eq!(app.guided_review_workspace.diff_top(), 0);
    assert_eq!(screen_row_of(&app, 6), Some(y), "the row stayed where it was clicked");
}

/// Side by side, the cursor on a pair's addition bands the deletion beside it too, over its tint.
#[test]
fn the_split_diff_bands_both_halves_of_the_cursors_pair() {
    use ratatui::{backend::TestBackend, Terminal};
    let mut app = seeded_app("nord", FixtureScenario::GuidedReviewSplit).expect("fixture");
    app.update(Msg::TerminalResized { width: 200, height: 40 });
    app.guided_review_workspace.position.line_offset = 3;
    let mut terminal = Terminal::new(TestBackend::new(200, 40)).expect("terminal");
    terminal.draw(|frame| crate::view::render(frame, &mut app)).expect("drawn");
    let buffer = terminal.backend().buffer();

    let text: Vec<String> =
        (0..40).map(|y| (0..200).map(|x| buffer[(x, y)].symbol()).collect()).collect();
    let y = text.iter().position(|row| row.contains("pub category: String")).expect("drawn");
    let x =
        unicode_width::UnicodeWidthStr::width(&text[y][..text[y].find("pub category").unwrap()]);
    let removed = buffer[(x as u16, y as u16)].bg;
    assert_ne!(Some(removed), app.theme_styles.diff_del_bg.bg, "the deletion is lifted");
    assert_ne!(Some(removed), app.theme_styles.cursor_line_bg().bg, "over its own tint");
}
