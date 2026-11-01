//! Peek picture — an image file in the preview, drawn rather than refused as not text.

use super::files::next_action_but_listings;
use super::*;
use luxe_tui::graphics::ImageTier;

/// A pane at `tier` showing `art/logo.png`, read by the engine as a `width`×`height` PNG.
fn previewing_picture(tier: ImageTier, size: (u32, u32)) -> (App, TestBackend) {
    previewing(tier, read_picture("art/logo.png", size))
}

/// A pane at `tier` showing `file`, as the engine read it.
fn previewing(tier: ImageTier, file: luxe_core::project_files::ProjectFile) -> (App, TestBackend) {
    let (columns, rows) = (160, 40);
    let backend = test_backend(columns, rows);
    let (mut app, mut rx) = create_app_with_engine();
    app.images = pinned_store(tier);
    app.update(Msg::TerminalResized { width: columns, height: rows });
    app.workspace = Some(changed_workspace("peek-picture", 1));
    app.update(Msg::OpenProjectFile(file.path.clone()));
    let luxe_core::engine::Action::ReadProjectFile { request_id, .. } =
        next_action_but_listings(&mut rx)
    else {
        panic!("an unchanged file is read");
    };
    let file = Box::new(file);
    app.update(Msg::Engine(luxe_core::event::Event::ProjectFileRead { request_id, file }));
    (app, backend)
}

/// What the engine answers for a PNG of this size at `path`.
fn read_picture(path: &str, (width, height): (u32, u32)) -> luxe_core::project_files::ProjectFile {
    let data = split_png(width, height);
    let image = luxe_core::project_files::PreviewImage {
        mime: "image/png".into(),
        data,
        declared_size: None,
    };
    luxe_core::project_files::ProjectFile {
        path: path.to_string(),
        text: String::new(),
        bytes: 1234,
        truncated: false,
        binary: true,
        image: Some(image),
    }
}

/// Clicks the footer's `key` button, as the frame now on screen draws it.
fn click_footer_key(app: &mut App, backend: &TestBackend, key: &str) {
    let buffer = render_app(app, backend);
    let footer = row_containing(&buffer, key, 160, 40);
    let text = row_text(&buffer, footer, 160);
    let action = left_click(app, column_of(&text, key), footer);
    apply(app, action);
}

/// The cells the half-block tier painted.
fn painted(buffer: &Buffer) -> usize {
    buffer.content.iter().filter(|cell| cell.symbol() == "\u{2580}").count()
}

/// An image file shows as its picture at its own size, named by its real dimensions, with no
/// claim that there is nothing to show.
///
/// The break this catches: the picture never drawn (the binary notice instead), or stretched to
/// fill the pane. 60×80 pixels in 10×20 cells is 6×4 cells and not one more.
#[test]
fn an_image_file_previews_as_its_picture_at_its_own_size() {
    let (mut app, backend) = previewing_picture(ImageTier::Blocks, (60, 80));
    let buffer = render_app(&mut app, &backend);
    let content = buffer_to_string(&buffer);
    assert_eq!(painted(&buffer), 6 * 4, "the picture, unstretched: {content}");
    assert!(content.contains("60\u{d7}80 image"), "the header gives its size: {content}");
    assert!(content.contains("click for full size"), "and says how to see more: {content}");
    assert!(!content.contains("not text"), "an image is not refused as binary: {content}");
    assert!(!content.contains("wrap"), "a picture has no lines to wrap: {content}");
}

/// A picture bigger than the pane shrinks into it rather than spilling past it.
#[test]
fn a_big_picture_shrinks_into_the_pane() {
    let (mut app, backend) = previewing_picture(ImageTier::Blocks, (4000, 3000));
    let buffer = render_app(&mut app, &backend);
    let rows = rows_of(&buffer);
    let drawn = rows.iter().filter(|row| row.contains('\u{2580}')).count();
    assert!(drawn > 4 && drawn < 40, "inside the pane's height: {drawn} rows");
}

/// Where the terminal cannot draw images, the pane says so and how to see it anyway.
#[test]
fn an_undrawable_picture_says_why_and_offers_the_way_out() {
    let (mut app, backend) = previewing_picture(ImageTier::Text, (60, 80));
    let buffer = render_app(&mut app, &backend);
    let content = buffer_to_string(&buffer);
    assert_eq!(painted(&buffer), 0);
    assert!(content.contains("can't draw images"), "{content}");
    assert!(content.contains("/images") && content.contains("o to open"), "{content}");
}

/// A click on the picture opens it full-window, the same viewer the transcript's images use.
#[test]
fn a_click_on_the_picture_opens_it_full_window() {
    let (mut app, backend) = previewing_picture(ImageTier::Blocks, (60, 80));
    let buffer = render_app(&mut app, &backend);
    let (col, row) = buffer
        .content
        .iter()
        .position(|cell| cell.symbol() == "\u{2580}")
        .map(|at| (at as u16 % buffer.area.width, at as u16 / buffer.area.width))
        .expect("the picture is on screen");
    let action = left_click(&mut app, col, row);
    apply(&mut app, action);
    assert!(app.overlays.image_viewer.is_some(), "the viewer is open on it");
}

/// With the keyboard in the pane, the footer offers what a picture can do, nothing a picture
/// cannot (no lines to move or pick), and `Enter` and its button open the viewer.
///
/// The break this catches: the text keys offered over a picture, or `Enter` asking the agent
/// about a line that does not exist.
#[test]
fn the_focused_footer_offers_a_pictures_keys_and_enter_enlarges_it() {
    let (mut app, backend) = previewing_picture(ImageTier::Blocks, (60, 80));
    app.update(Msg::FocusPeek);
    let buffer = render_app(&mut app, &backend);
    let content = buffer_to_string(&buffer);
    assert!(content.contains("enter full size") && content.contains("o open"), "{content}");
    assert!(!content.contains("j k line") && !content.contains("v select"), "{content}");
    assert!(!content.contains("s source"), "an image file has no source to see: {content}");

    app.update(Msg::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)));
    assert!(app.overlays.image_viewer.is_some(), "Enter opens the viewer");
    assert_eq!(app.editor.current_input(), "", "and asks the agent nothing");

    app.overlays.image_viewer = None;
    click_footer_key(&mut app, &backend, "enter full size");
    assert!(app.overlays.image_viewer.is_some(), "and so does its button");
}

/// What the engine answers for an SVG declaring 24×12, drawn as a 240×120 PNG.
fn read_svg() -> luxe_core::project_files::ProjectFile {
    let mut file = read_picture("art/icon.svg", (240, 120));
    file.text = "<svg width=\"24\" height=\"12\"/>\n".into();
    file.binary = false;
    file.image.as_mut().expect("read_picture carries a picture").declared_size = Some((24, 12));
    file
}

/// An SVG previews as its picture, named by the size it declares, and `s` and its button swap
/// between that and its source, the footer naming the face it goes to.
///
/// The break this catches: the source drawn under the picture, the header giving the size the
/// picture was drawn at rather than the SVG's own, or no way to the source.
#[test]
fn an_svg_previews_as_its_picture_and_s_shows_its_source() {
    let (mut app, backend) = previewing(ImageTier::Blocks, read_svg());
    app.update(Msg::FocusPeek);
    let content = buffer_to_string(&render_app(&mut app, &backend));
    assert!(content.contains("24\u{d7}12 SVG"), "the size it declares: {content}");
    assert!(!content.contains("<svg"), "no source under the picture: {content}");
    assert!(content.contains("s source") && content.contains("o editor"), "{content}");

    app.update(Msg::Key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE)));
    let buffer = render_app(&mut app, &backend);
    let content = buffer_to_string(&buffer);
    assert_eq!(painted(&buffer), 0, "the picture gives way to the source: {content}");
    assert!(content.contains("<svg width") && content.contains("s picture"), "{content}");
    assert_eq!(app.editor.current_input(), "", "the s is not typed");

    click_footer_key(&mut app, &backend, "s picture");
    assert!(painted(&render_app(&mut app, &backend)) > 0, "its button brings the picture back");
}

/// A workspace where a source file and a picture both changed.
fn source_and_picture_changed() -> luxe_core::workspace::WorkspaceSnapshot {
    let mut snapshot = changed_workspace("picture-changed", 2);
    snapshot.changes.files[0].path = "src/a.rs".into();
    snapshot.changes.files[1].path = "art/logo.png".into();
    snapshot.changes.files[1].binary = true;
    snapshot
}

/// What the pane asked the engine for: `(diffs, reads)` by path, folder listings aside.
fn asked_for(
    rx: &mut tokio::sync::mpsc::Receiver<luxe_core::engine::Action>,
) -> (Vec<String>, Vec<String>) {
    use luxe_core::engine::Action;
    let (mut diffs, mut reads) = (Vec::new(), Vec::new());
    while let Ok(action) = rx.try_recv() {
        match action {
            Action::LoadFileDiff { path, .. } => diffs.push(path),
            Action::ReadProjectFile { path, .. } => reads.push(path),
            _ => {}
        }
    }
    (diffs, reads)
}

/// A changed picture opens on its picture, not on a diff of its bytes; stepping off it goes back
/// to diffs, since that File view was the picture's own and not the reader's choice.
///
/// The break this catches: a changed image opened on DIFF (no read asked for), or the forced File
/// view carried on to the next source file.
#[test]
fn a_changed_picture_opens_on_its_picture_and_stepping_off_it_returns_to_diffs() {
    let (mut app, mut rx) = create_app_with_engine();
    app.workspace = Some(source_and_picture_changed());
    app.update(Msg::OpenProjectFile("art/logo.png".to_string()));
    let (diffs, reads) = asked_for(&mut rx);
    assert_eq!(reads, ["art/logo.png"], "the picture is read to be shown");
    assert_eq!(diffs, ["art/logo.png"], "and its diff still loads, for the DIFF switch");

    app.update(Msg::ChangesStepFile(true));
    let (diffs, reads) = asked_for(&mut rx);
    assert_eq!((diffs.as_slice(), reads.len()), (["src/a.rs".to_string()].as_slice(), 0));

    app.update(Msg::ChangesStepFile(true));
    let (_, reads) = asked_for(&mut rx);
    assert_eq!(reads, ["art/logo.png"], "stepping onto a picture shows it too");
}
