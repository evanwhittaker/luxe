use super::*;

const LIST: u16 = 36;

/// The break this catches: a wide picker stacking its pane under a list stretched across the
/// whole width, or a pane that runs into the frame.
#[test]
fn a_wide_body_puts_the_pane_beside_a_list_of_its_own_width() {
    let body = Rect::new(1, 2, 120, 30);
    let (list, place) = place(body, LIST);
    assert_eq!(list, Rect::new(1, 2, LIST, 30));
    let Some(PanePlace::Beside { rule_x, pane }) = place else { panic!("beside: {place:?}") };
    assert_eq!(rule_x, 1 + LIST);
    assert_eq!(pane.x, rule_x + PANE_GUTTER);
    assert_eq!(pane.right(), body.right() - 1, "a cell short of the frame");
    assert_eq!((pane.y, pane.height), (2, 30), "the whole height");
}

/// The break this catches: a terminal too narrow for a pane beside the list losing the preview
/// altogether, though it has the height to show it under the list.
#[test]
fn a_narrow_tall_body_puts_the_pane_under_the_list() {
    let body = Rect::new(0, 0, 60, 30);
    let (list, place) = place(body, LIST);
    assert_eq!(list, Rect::new(0, 0, 60, 10), "a third of the height, the whole width");
    let Some(PanePlace::Below { rule_y, pane }) = place else { panic!("below: {place:?}") };
    assert_eq!(rule_y, 10);
    assert_eq!(pane, Rect::new(0, 11, 60, 19));
}

/// The break this catches: a pane squeezed into a body with no room for one, too small to read,
/// taking rows the list needed; the rows then have to carry the text themselves.
#[test]
fn a_small_body_has_no_pane() {
    let body = Rect::new(0, 0, 60, 9);
    assert_eq!(place(body, LIST), (body, None));
    assert!(!fits(body, LIST));
    assert!(fits(Rect::new(0, 0, 60, 10), LIST), "one row more and it fits under the list");
}

/// The break this catches: a short body under the list starving the list below its floor.
#[test]
fn the_list_keeps_its_floor_when_the_pane_is_under_it() {
    let (list, _) = place(Rect::new(0, 0, 60, 11), LIST);
    assert_eq!(list.height, BELOW_LIST_ROWS);
}
