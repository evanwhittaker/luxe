//! Picture — telling an image file by its first bytes, and reading it whole for the preview.
//!
//! Content rather than extension, the way the `read` tool decides: a `.txt` holding a PNG is a PNG.
//! Only the formats luxe links a decoder for are recognized, and each by a signature long enough
//! that a text file cannot start with it by accident, so a false match never costs a reader the
//! text they asked for.

use super::{unreadable, PreviewImage, ProjectFileFailure};
use std::io::Read;
use std::path::Path;

/// Bytes an image may hold and still be previewed: the front-end's own image store refuses
/// anything larger, so reading more would only be thrown away.
const MAX_IMAGE_BYTES: u64 = 24 * 1024 * 1024;

/// Bytes the longest signature needs.
const SIGNATURE: usize = 12;

/// Extensions of the pictures a preview draws.
const PICTURE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "webp", "svg"];

/// Whether `path` names a picture, by its extension.
///
/// What a front-end goes by before the file is read, to open it on its picture rather than on a
/// diff of bytes nobody can read. The read itself still goes by content.
pub fn names_picture(path: &str) -> bool {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.rsplit_once('.').is_some_and(|(stem, extension)| {
        !stem.is_empty() && PICTURE_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
    })
}

/// The image at `path`, when it is one luxe can draw and small enough to preview; `None` for
/// anything else, which the caller then reads as text or binary.
pub(super) fn read(path: &Path, size: u64) -> Result<Option<PreviewImage>, ProjectFileFailure> {
    if size > MAX_IMAGE_BYTES {
        return Ok(None);
    }
    let mut file = std::fs::File::open(path).map_err(unreadable)?;
    let mut bytes = Vec::new();
    (&mut file).take(SIGNATURE as u64).read_to_end(&mut bytes).map_err(unreadable)?;
    let Some(mime) = mime(&bytes) else {
        return Ok(None);
    };
    file.take(MAX_IMAGE_BYTES).read_to_end(&mut bytes).map_err(unreadable)?;
    use base64::Engine as _;
    let data = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(Some(PreviewImage { mime: mime.to_string(), data, declared_size: None }))
}

/// The MIME type `header` announces, among PNG, JPEG, GIF and WebP.
fn mime(header: &[u8]) -> Option<&'static str> {
    if header.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if header.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    if header.starts_with(b"GIF87a") || header.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    let webp = header.len() >= SIGNATURE && &header[..4] == b"RIFF" && &header[8..12] == b"WEBP";
    webp.then_some("image/webp")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// A picture is named by its extension in any case; a dotfile, a folder and a lookalike are not.
    #[test]
    fn a_picture_is_named_by_its_extension() {
        for path in ["logo.png", "a/b/Photo.JPEG", "x.jpg", "anim.gif", "s.webp", "icons/arrow.svg"]
        {
            assert!(names_picture(path), "{path}");
        }
        for path in ["png", ".png", "a.png/notes.md", "logo.pngx", "main.rs", "img/"] {
            assert!(!names_picture(path), "{path}");
        }
    }

    /// One signature per format, and the near misses that must stay text or binary: a RIFF that
    /// is a WAV, a GIF prefix with no version, and text that merely mentions PNG.
    #[test]
    fn each_format_is_known_by_its_signature_and_nothing_else_is() {
        assert_eq!(mime(b"\x89PNG\r\n\x1a\n...."), Some("image/png"));
        assert_eq!(mime(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
        assert_eq!(mime(b"GIF89a.."), Some("image/gif"));
        assert_eq!(mime(b"GIF87a.."), Some("image/gif"));
        assert_eq!(mime(b"RIFF\0\0\0\0WEBP"), Some("image/webp"));
        assert_eq!(mime(b"RIFF\0\0\0\0WAVE"), None);
        assert_eq!(mime(b"RIFF\0\0\0\0WE"), None, "too short to say");
        assert_eq!(mime(b"GIF is a format"), None);
        assert_eq!(mime(b"PNG files are"), None);
        assert_eq!(mime(&[0xFF, 0xD8]), None);
    }

    /// An image reads whole, as base64 the front-end decodes; one past the cap is left to the
    /// binary notice rather than read only to be refused.
    #[test]
    fn an_image_reads_whole_and_an_oversized_one_does_not() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dot.png");
        let bytes = b"\x89PNG\r\n\x1a\nrest-of-the-file".to_vec();
        std::fs::write(&path, &bytes).unwrap();

        let image = read(&path, bytes.len() as u64).unwrap().unwrap();
        use base64::Engine as _;
        let decoded = base64::engine::general_purpose::STANDARD.decode(&image.data).unwrap();
        assert_eq!((image.mime.as_str(), decoded), ("image/png", bytes));
        assert_eq!(read(&path, MAX_IMAGE_BYTES + 1).unwrap(), None);
    }
}
