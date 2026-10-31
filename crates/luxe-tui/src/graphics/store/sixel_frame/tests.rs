use super::super::tests::{origin, png};
use super::*;
use crate::graphics::{CellSize, Fit};
use ratatui::layout::Rect;
use ratatui::style::Color;

const SCREEN: (u16, u16) = (40, 30);

fn sixel_store() -> (ImageStore, ImageKey, Fit) {
    let mut store = ImageStore::new(ImageTier::Sixel, CellSize { width: 10, height: 20 });
    let key = ImageKey::new("shot");
    assert!(store.insert(key.clone(), png(100, 100), origin("shot.png")));
    // Square, so ten columns of 10px are five rows of 20px: a 100x100 picture
    let fit = store.fit(&key, 10, 10).expect("fits");
    assert_eq!(fit, Fit { cols: 10, rows: 5 });
    (store, key, fit)
}

/// One whole frame: paints `shown` of the image's rows from `top`, finishes it, and takes what the
/// terminal is owed.
fn frame(
    store: &mut ImageStore,
    (key, fit): (&ImageKey, Fit),
    at: (u16, u16),
    shown: std::ops::Range<u16>,
) -> (String, Buffer) {
    let mut buf = Buffer::empty(Rect::new(0, 0, SCREEN.0, SCREEN.1));
    store.begin_frame();
    for (offset, image_row) in shown.enumerate() {
        let painted = store.paint_row(
            &mut buf,
            (at.0, at.1 + offset as u16),
            key,
            fit,
            image_row,
            Color::Rgb(1, 2, 3),
        );
        assert!(painted);
    }
    store.finish_frame(&buf);
    (store.drain_control(), buf)
}

/// The picture is drawn once, after its cells, at its cell, with the cursor put back — and a frame
/// that changes nothing about it costs the terminal nothing.
///
/// The break this catches: redrawing every frame, which at animation pace re-sends the whole
/// picture thirty times a second.
#[test]
fn an_image_is_drawn_once_and_then_left_alone() {
    let (mut store, key, fit) = sixel_store();
    let (first, _) = frame(&mut store, (&key, fit), (3, 4), 0..5);
    assert!(first.starts_with("\x1b7\x1b[5;4H\x1bP9q\"1;1;100;100"), "{:?}", &first[..40]);
    assert!(first.as_bytes().ends_with(&[0x1b, b'\\', 0x1b, b'8']), "the cursor goes back");
    let (second, _) = frame(&mut store, (&key, fit), (3, 4), 0..5);
    assert_eq!(second, "", "nothing about the image changed");
}

/// An image that moves is drawn at its new place; its old place is the diff's to wipe.
///
/// The break this catches: remembering only that an image was drawn, so a scrolled transcript
/// leaves the picture where it was while the text moves.
#[test]
fn an_image_that_moved_is_drawn_again() {
    let (mut store, key, fit) = sixel_store();
    frame(&mut store, (&key, fit), (3, 4), 0..5);
    let (moved, _) = frame(&mut store, (&key, fit), (3, 6), 0..5);
    assert!(moved.contains("\x1b[7;4H\x1bP9q"), "{:?}", &moved[..30]);
}

/// A half-scrolled image is cropped to the rows that show, from the right end of it.
///
/// The break this catches: drawing the whole picture at the first visible row, which shows the top
/// of an image whose top has scrolled away, and runs past the rows reserved for it.
#[test]
fn a_half_scrolled_image_is_cropped_to_its_visible_rows() {
    let (mut store, key, fit) = sixel_store();
    let (top_cut, _) = frame(&mut store, (&key, fit), (0, 2), 2..5);
    assert!(top_cut.contains("\x1b[3;1H\x1bP9q\"1;1;100;60"), "three rows of 20px");
    let (bottom_cut, _) = frame(&mut store, (&key, fit), (0, 2), 0..2);
    assert!(bottom_cut.contains("\x1b[3;1H\x1bP9q\"1;1;100;40"), "two rows of 20px");
}

/// Something drawn over the image's cells stops it being drawn, and uncovering it draws it again.
///
/// The break this catches: a picture painted over a menu that is meant to be on top of it.
#[test]
fn an_image_under_something_else_is_not_drawn_over_it() {
    let (mut store, key, fit) = sixel_store();
    frame(&mut store, (&key, fit), (3, 4), 0..5);

    let mut buf = Buffer::empty(Rect::new(0, 0, SCREEN.0, SCREEN.1));
    store.begin_frame();
    for row in 0..5 {
        store.paint_row(&mut buf, (3, 4 + row), &key, fit, row, Color::Reset);
    }
    buf[(5, 6)].set_symbol("x");
    store.finish_frame(&buf);
    assert_eq!(store.drain_control(), "", "a menu is on top");

    let (uncovered, _) = frame(&mut store, (&key, fit), (3, 4), 0..5);
    assert!(uncovered.contains("\x1bP9q"), "and when it goes, the picture is back");
}

/// A Sixel that ends on the last line of the screen scrolls it. The last line is never drawn on.
///
/// The break this catches: the whole screen jumping up by the overhang when an image is scrolled to
/// the bottom of the transcript.
#[test]
fn an_image_never_reaches_the_last_screen_row() {
    let (mut store, key, fit) = sixel_store();
    // Rows 27..32 on a 30-row screen: only 27, 28 may be drawn, leaving 29 for the cursor
    let (clipped, _) = frame(&mut store, (&key, fit), (0, 27), 0..3);
    assert!(clipped.contains("\x1b[28;1H\x1bP9q\"1;1;100;40"), "two rows, not three");

    let (none, _) = frame(&mut store, (&key, fit), (0, 29), 0..1);
    assert_eq!(none, "", "a picture on the last line has nowhere to go");
}

/// After the screen is cleared out from under it, every image is drawn again.
#[test]
fn invalidating_the_overlay_draws_everything_again() {
    let (mut store, key, fit) = sixel_store();
    frame(&mut store, (&key, fit), (3, 4), 0..5);
    store.invalidate_overlay();
    let (again, _) = frame(&mut store, (&key, fit), (3, 4), 0..5);
    assert!(again.contains("\x1bP9q"));
}

/// A new size is a new picture.
#[test]
fn an_image_at_a_new_size_is_drawn_at_that_size() {
    let (mut store, key, fit) = sixel_store();
    frame(&mut store, (&key, fit), (0, 0), 0..5);
    let smaller = store.fit(&key, 4, 4).expect("fits");
    let (redrawn, _) = frame(&mut store, (&key, smaller), (0, 0), 0..smaller.rows);
    let width = u32::from(smaller.cols) * 10;
    assert!(redrawn.contains(&format!("\"1;1;{width};")), "{:?}", &redrawn[..40]);
}

/// The cells under a picture are blank, marked, and carry nothing the terminal could mistake for a
/// command, so ratatui's diff can write and wipe them like any other cell.
#[test]
fn the_cells_under_a_picture_are_blank_and_marked() {
    let (mut store, key, fit) = sixel_store();
    let (_, buf) = frame(&mut store, (&key, fit), (3, 4), 0..5);
    let cell = &buf[(3, 4)];
    assert_eq!(cell.symbol(), " ");
    assert_eq!(cell.bg, Color::Rgb(1, 2, 3));
    assert!(matches!(cell.fg, Color::Rgb(..)), "marked with the picture's id");
    assert_eq!(buf[(2, 4)].symbol(), " ", "and no wider than the picture");
    assert_eq!(buf[(2, 4)].bg, Color::Reset);
}

/// Only the Sixel tier owes the terminal anything at the end of a frame.
#[test]
fn other_tiers_owe_nothing_at_the_end_of_a_frame() {
    let (mut store, key, fit) = sixel_store();
    store.set_tier(ImageTier::Blocks);
    let (owed, _) = frame(&mut store, (&key, fit), (0, 0), 0..5);
    assert_eq!(owed, "");
}
