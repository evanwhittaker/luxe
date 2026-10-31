use super::super::GuidedReviewWorkspace;

/// A workspace whose last frame drew from row `top` with the cursor on `cursor`.
fn drawn(top: usize, cursor: usize) -> GuidedReviewWorkspace {
    let mut workspace = GuidedReviewWorkspace::default();
    workspace.position.line_offset = cursor;
    workspace.record_diff_window(top);
    workspace
}

/// A cursor moved by a line move keeps the window's top while it is still on screen.
#[test]
fn a_kept_window_starts_where_the_last_frame_did() {
    let mut workspace = drawn(10, 12);
    workspace.position.line_offset = 15;
    workspace.keep_diff_window();
    assert_eq!(workspace.diff_window_start(20), 10);
}

/// Kept, but moved past the bottom: the start comes down only as far as a row a line could reach.
#[test]
fn a_kept_window_comes_down_only_as_far_as_the_cursor_needs() {
    let mut workspace = drawn(10, 12);
    workspace.position.line_offset = 40;
    workspace.keep_diff_window();
    assert_eq!(workspace.diff_window_start(20), 21, "rows 21 to 40 are twenty");
}

/// Kept, and moved above the top: the window starts at the cursor.
#[test]
fn a_kept_window_comes_up_to_a_cursor_above_it() {
    let mut workspace = drawn(10, 12);
    workspace.position.line_offset = 4;
    workspace.keep_diff_window();
    assert_eq!(workspace.diff_window_start(20), 4);
}

/// Any other move leaves the window behind, so the cursor's row is the top, as a jump always was.
#[test]
fn a_jump_starts_the_window_at_the_cursor() {
    let mut workspace = drawn(10, 12);
    workspace.position.line_offset = 14;
    assert_eq!(workspace.diff_window_start(20), 14);
}

/// The wheel carries the window by what the cursor moved, either way, and never above row 0.
#[test]
fn the_wheel_carries_the_window_with_the_cursor() {
    let mut workspace = drawn(10, 12);
    workspace.position.line_offset = 15;
    workspace.carry_diff_window(12);
    assert_eq!(workspace.diff_window_start(20), 13, "down three");
    workspace.position.line_offset = 1;
    workspace.carry_diff_window(15);
    assert_eq!(workspace.diff_window_start(20), 0, "up past the top");
}
