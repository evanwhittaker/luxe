//! Vector — an SVG file drawn to pixels for the preview.
//!
//! Drawn here, during the read, so every front-end gets a PNG it already knows how to show, the
//! terminal and the remote web view alike, and none of them needs an SVG renderer of its own.
//!
//! Nothing outside the file is loaded. An `<image>` that names a path or a URL draws nothing;
//! only the `data:` pictures an SVG carries inside itself draw. Text uses the fonts installed on
//! the machine, found once per process since looking costs a scan of every font folder.

use super::PreviewImage;
use resvg::{tiny_skia, usvg};
use std::sync::{Arc, OnceLock};

/// Pixels on the drawing's longest side.
///
/// A vector's declared size is a suggestion (icons say 24×24), and drawing at it would leave the
/// preview a speck; this is enough to fill a wide pane on a high-density screen, and small enough
/// to draw quickly and send cheaply.
const LONGEST_SIDE: f32 = 1600.0;

/// Whether `path` names an SVG.
pub(super) fn names_svg(path: &str) -> bool {
    path.rsplit_once('.').is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("svg"))
}

/// `source` drawn as a PNG, or `None` when it is not an SVG luxe can draw, in which case the file
/// still shows as the text it is.
pub(super) fn draw(source: &str) -> Option<PreviewImage> {
    let tree = match usvg::Tree::from_str(source, &options(source)) {
        Ok(tree) => tree,
        Err(error) => {
            tracing::debug!(%error, "svg not drawn");
            return None;
        }
    };
    let size = tree.size();
    let scale = LONGEST_SIDE / size.width().max(size.height());
    let pixels = |side: f32| ((side * scale).ceil() as u32).max(1);
    let mut pixmap = tiny_skia::Pixmap::new(pixels(size.width()), pixels(size.height()))?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    let png = pixmap.encode_png().ok()?;
    use base64::Engine as _;
    let data = base64::engine::general_purpose::STANDARD.encode(png);
    let declared = (size.width().round() as u32, size.height().round() as u32);
    Some(PreviewImage { mime: "image/png".to_string(), data, declared_size: Some(declared) })
}

/// How `source` is read: loading nothing a path or URL names, and with the machine's fonts when it
/// has text to set, which most icons do not and so skip the scan for.
fn options(source: &str) -> usvg::Options<'static> {
    let mut options = usvg::Options::default();
    if source.contains("<text") {
        options.fontdb = system_fonts();
    }
    options.image_href_resolver.resolve_string = Box::new(|_, _| None);
    options
}

/// The fonts installed on this machine, found on first use and shared after.
fn system_fonts() -> Arc<usvg::fontdb::Database> {
    static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    let fonts = FONTS.get_or_init(|| {
        let mut fonts = usvg::fontdb::Database::new();
        fonts.load_system_fonts();
        Arc::new(fonts)
    });
    Arc::clone(fonts)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// The pixel at `(x, y)` of a drawn picture, as RGBA.
    fn pixel(image: &PreviewImage, x: u32, y: u32) -> [u8; 4] {
        use base64::Engine as _;
        let png = base64::engine::general_purpose::STANDARD.decode(&image.data).unwrap();
        let pixmap = tiny_skia::Pixmap::decode_png(&png).unwrap();
        let color = pixmap.pixel(x, y).unwrap();
        [color.red(), color.green(), color.blue(), color.alpha()]
    }

    /// A 24×12 drawing comes out 1600×800 with its declared size kept, and drawn: red where the
    /// rectangle is, nothing where it is not.
    #[test]
    fn an_svg_is_drawn_large_with_its_own_size_kept() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="12">
            <rect x="0" y="0" width="12" height="12" fill="red"/></svg>"#;
        let image = draw(svg).unwrap();
        assert_eq!((image.mime.as_str(), image.declared_size), ("image/png", Some((24, 12))));
        assert_eq!(pixel(&image, 10, 10), [255, 0, 0, 255], "the rectangle");
        assert_eq!(pixel(&image, 1590, 790)[3], 0, "and the empty half is clear");
    }

    /// An `<image>` naming a file on disk draws nothing: a preview must not reach past the file.
    ///
    /// The break this catches: the default resolver, which reads the named path.
    #[test]
    fn an_svg_cannot_pull_in_a_file_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let red = image_png(dir.path());
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
                width="10" height="10"><image width="10" height="10" xlink:href="{}"/></svg>"#,
            red.display()
        );
        let image = draw(&svg).unwrap();
        assert_eq!(pixel(&image, 800, 800)[3], 0, "the named file was not drawn");
    }

    /// A 1×1 red PNG on disk, for an SVG to name.
    fn image_png(dir: &std::path::Path) -> std::path::PathBuf {
        let mut pixmap = tiny_skia::Pixmap::new(1, 1).unwrap();
        pixmap.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
        let path = dir.join("red.png");
        std::fs::write(&path, pixmap.encode_png().unwrap()).unwrap();
        path
    }

    /// What is not an SVG is left to show as text.
    #[test]
    fn what_is_not_an_svg_is_not_drawn() {
        assert!(draw("fn main() {}").is_none());
        assert!(draw("<svg").is_none(), "cut short");
        assert!(names_svg("a/icon.SVG") && !names_svg("svg") && !names_svg("a.svgz"));
    }
}
