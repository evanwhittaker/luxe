use super::*;

/// Three rendered lines, padded the way a transcript row is, the middle one with a wide glyph.
fn lines() -> Vec<String> {
    vec!["alpha bravo   ".into(), "ab界cd    ".into(), "x  ".into()]
}

fn at(line: usize, column: usize) -> TextPoint {
    TextPoint::new(line, column)
}

fn step(point: TextPoint, step: SelectionStep) -> TextPoint {
    step_point(&lines(), point, step)
}

#[test]
fn left_and_right_move_a_character_inside_a_line() {
    assert_eq!(step(at(0, 3), SelectionStep::Left), at(0, 2));
    assert_eq!(step(at(0, 3), SelectionStep::Right), at(0, 4));
}

#[test]
fn a_wide_glyph_is_stepped_over_whole() {
    // `界` takes cells 2 and 3
    assert_eq!(step(at(1, 2), SelectionStep::Right), at(1, 4));
    assert_eq!(step(at(1, 4), SelectionStep::Left), at(1, 2));
}

#[test]
fn left_at_a_line_start_wraps_to_the_text_end_above() {
    assert_eq!(step(at(1, 0), SelectionStep::Left), at(0, 11), "the padding is not text");
}

#[test]
fn right_at_a_text_end_wraps_to_the_next_line_start() {
    assert_eq!(step(at(0, 11), SelectionStep::Right), at(1, 0));
}

#[test]
fn the_ends_of_the_text_hold_still() {
    assert_eq!(step(at(0, 0), SelectionStep::Left), at(0, 0));
    assert_eq!(step(at(2, 1), SelectionStep::Right), at(2, 1));
    assert_eq!(step(at(0, 4), SelectionStep::Up), at(0, 4));
    assert_eq!(step(at(2, 0), SelectionStep::Down), at(2, 0));
}

#[test]
fn up_and_down_keep_the_column_or_stop_at_a_shorter_line() {
    assert_eq!(step(at(1, 1), SelectionStep::Up), at(0, 1));
    assert_eq!(step(at(0, 9), SelectionStep::Down), at(1, 6), "line two's text ends at 6");
    assert_eq!(step(at(1, 4), SelectionStep::Down), at(2, 1));
}

#[test]
fn home_and_end_take_the_line_s_text() {
    assert_eq!(step(at(0, 4), SelectionStep::LineStart), at(0, 0));
    assert_eq!(step(at(0, 4), SelectionStep::LineEnd), at(0, 11));
}

#[test]
fn an_end_past_the_text_is_brought_back_first() {
    // A triple-click parks the cursor at the far end of the line
    assert_eq!(step(at(0, usize::MAX), SelectionStep::Left), at(0, 10));
}

#[test]
fn a_point_off_the_lines_is_left_alone() {
    assert_eq!(step(at(9, 2), SelectionStep::Left), at(9, 2));
}
