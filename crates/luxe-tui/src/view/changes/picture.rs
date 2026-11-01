//! The File view's picture — an image file drawn in the pane, the way the transcript draws one.
//!
//! Painted through the transcript's own image store, so every terminal it can draw in (kitty,
//! Sixel, half blocks) and every bound it enforces come along, and an image already prepared at
//! this size is reused rather than sent again. Never larger than the image's own pixels: a preview
//! shows the file as it is, and an icon blown up to the pane's width is a blur, not the icon.
//! Painted after the rest of the pane, which draws from a shared `App`, because the store needs
//! `&mut`.

use crate::app::App;
use crate::changes::{ChangesView, PeekView, TextLoad};
use crate::graphics::ImageKey;
use crate::mouse::MouseTarget;
use ratatui::{
    layout::{Alignment, Rect},
    style::Color,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

/// The picture the File view shows instead of lines: an image's, or an SVG's while its source is
/// not what the reader asked for.
pub(crate) fn showing(view: &ChangesView) -> Option<&ImageKey> {
    let ready = matches!(view.text, TextLoad::Ready(_));
    let shown = ready && view.view == PeekView::File && !view.source;
    view.picture.as_ref().filter(|_| shown).map(|picture| &picture.key)
}

/// Draws the picture in `area` with a line under it saying how big it really is, or a line
/// saying why there is no picture.
pub(super) fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(key) = app.changes.as_ref().and_then(showing).cloned() else {
        return;
    };
    if area.width == 0 || area.height == 0 {
        return;
    }
    let caption = Rect { y: area.bottom() - 1, height: 1, ..area };
    let stage = Rect { height: area.height.saturating_sub(2), ..area };
    let (text, style) = match paint(frame, stage, &key, app) {
        true => ("click for full size", app.theme_styles.muted),
        false => (UNDRAWABLE, app.theme_styles.warning),
    };
    let (caption, alignment) = match stage.height {
        0 => (area, Alignment::Left),
        _ => (caption, Alignment::Center),
    };
    let line = Paragraph::new(Line::from(Span::styled(text, style))).alignment(alignment);
    frame.render_widget(line.wrap(Wrap { trim: true }), caption);
}

/// What the pane says where a terminal cannot draw images: why, where to look, and the way to see
/// the picture anyway.
const UNDRAWABLE: &str = "This terminal can't draw images. See /images, or press o to open it.";

/// Paints the picture centered in `stage` and publishes its click; `false` when nothing could be
/// drawn there.
fn paint(frame: &mut Frame, stage: Rect, key: &ImageKey, app: &mut App) -> bool {
    // The viewer covers the window and wants this image at its own size; preparing both every
    // frame would re-send the picture twice a frame, which the terminal shows as a flicker
    if app.overlays.image_viewer.is_some() {
        return true;
    }
    if stage.height == 0 || !app.images.tier().draws_pixels() {
        return false;
    }
    let Some(fit) = app.images.fit_unstretched(key, stage.width, stage.height) else {
        return false;
    };
    let canvas = app.theme_styles.background.bg.unwrap_or(Color::Reset);
    let buf = frame.buffer_mut();
    let at = crate::image_viewer::paint_centered(buf, stage, (key, fit), &mut app.images, canvas);
    // After painting, so the picture just drawn is never what gets evicted
    app.images.sweep();
    app.hit.borrow_mut().push(at, MouseTarget::PreviewImage(key.clone()));
    true
}
