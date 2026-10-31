use super::*;
use image::{Rgba, RgbaImage};

/// A decoded sixel stream: the palette it defined and the palette entry of each pixel.
struct Decoded {
    width: usize,
    height: usize,
    palette: HashMap<usize, [u32; 3]>,
    grid: Vec<Vec<Option<usize>>>,
}

/// Reads sixel data the way a terminal does, so a round trip checks the encoder against the format
/// rather than against itself.
fn decode(stream: &str) -> Decoded {
    let body = stream.strip_prefix("\x1bP9q").expect("DCS header").strip_suffix("\x1b\\");
    let body = body.expect("string terminator");
    let (raster, data) = body.split_once('#').map_or((body, ""), |(raster, rest)| (raster, rest));
    let dims: Vec<usize> = raster
        .trim_start_matches('"')
        .split(';')
        .map(|part| part.parse().expect("number"))
        .collect();
    let mut decoded = Decoded {
        width: dims[2],
        height: dims[3],
        palette: HashMap::new(),
        grid: vec![vec![None; dims[2]]; dims[3]],
    };
    paint(&mut decoded, &format!("#{data}"));
    decoded
}

fn paint(decoded: &mut Decoded, data: &str) {
    let (mut color, mut x, mut band) = (0usize, 0usize, 0usize);
    let mut chars = data.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '#' => {
                let mut number = String::new();
                while let Some(d) = chars.next_if(|d| d.is_ascii_digit() || *d == ';') {
                    number.push(d);
                }
                let parts: Vec<usize> = number.split(';').map(|p| p.parse().unwrap()).collect();
                color = parts[0];
                if let [_, _, r, g, b] = parts[..] {
                    decoded.palette.insert(color, [r as u32, g as u32, b as u32]);
                }
            }
            '$' => x = 0,
            '-' => (band, x) = (band + 1, 0),
            '!' => {
                let mut run = String::new();
                while let Some(d) = chars.next_if(char::is_ascii_digit) {
                    run.push(d);
                }
                let sixel = chars.next().expect("a character after a repeat");
                for _ in 0..run.parse::<usize>().unwrap() {
                    x = put(decoded, (x, band), color, sixel);
                }
            }
            sixel => x = put(decoded, (x, band), color, sixel),
        }
    }
}

fn put(decoded: &mut Decoded, (x, band): (usize, usize), color: usize, sixel: char) -> usize {
    let bits = sixel as u8 - b'?';
    for bit in 0..6 {
        let y = band * 6 + bit;
        if bits & (1 << bit) != 0 && y < decoded.height && x < decoded.width {
            decoded.grid[y][x] = Some(color);
        }
    }
    x + 1
}

fn solid(width: u32, height: u32, color: [u8; 3]) -> RgbaImage {
    RgbaImage::from_pixel(width, height, Rgba([color[0], color[1], color[2], 255]))
}

/// The smallest picture there is, byte for byte: a header with the size, the palette, one band of
/// one color, and the string terminator. Pinned as a literal so a change to any of those is seen.
#[test]
fn a_solid_picture_is_one_band_of_one_color() {
    let indexed = Indexed::quantize(&solid(3, 6, [255, 0, 0]));
    assert_eq!(indexed.encode(0..6), "\x1bP9q\"1;1;3;6#0;2;100;0;0#0~~~\x1b\\");
}

/// A run longer than three is written as `!n` and the character, because that is shorter.
#[test]
fn long_runs_are_run_length_encoded() {
    let indexed = Indexed::quantize(&solid(10, 6, [0, 0, 255]));
    assert!(indexed.encode(0..6).ends_with("#0!10~\x1b\\"));
}

/// Every pixel comes back as the palette entry it was given, across bands, with colors sharing a
/// band, and with a last band that is only partly there.
#[test]
fn a_picture_decodes_to_the_pixels_it_was_made_from() {
    let mut image = RgbaImage::new(7, 8);
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let shade = if (x + y) % 3 == 0 {
            [200, 10, 10]
        } else if x < 3 {
            [10, 200, 10]
        } else {
            [10, 10, 200]
        };
        *pixel = Rgba([shade[0], shade[1], shade[2], 255]);
    }
    let indexed = Indexed::quantize(&image);
    let decoded = decode(&indexed.encode(0..8));
    assert_eq!((decoded.width, decoded.height), (7, 8));
    for (x, y, pixel) in image.enumerate_pixels() {
        let entry = decoded.grid[y as usize][x as usize].expect("every pixel is painted");
        let [r, g, b] = decoded.palette[&entry];
        let want = pixel.0;
        assert_eq!([r, g, b], [percent(want[0]), percent(want[1]), percent(want[2])], "({x},{y})");
    }
}

/// A crop draws the rows asked for and only those, which is what a half-scrolled image needs. The
/// break this catches: a crop that kept encoding from row zero, so the visible half showed the top.
#[test]
fn a_crop_encodes_only_the_rows_asked_for() {
    let mut image = solid(4, 12, [255, 0, 0]);
    for y in 6..12 {
        for x in 0..4 {
            image.put_pixel(x, y, Rgba([0, 0, 255, 255]));
        }
    }
    let indexed = Indexed::quantize(&image);
    let decoded = decode(&indexed.encode(6..12));
    assert_eq!(decoded.height, 6);
    let [r, _, b] = decoded.palette[&decoded.grid[0][0].expect("painted")];
    assert_eq!((r, b), (0, 100), "the lower half is blue");
}

/// Out-of-range and empty crops draw nothing rather than a sequence that moves the cursor.
#[test]
fn an_empty_crop_is_empty() {
    let indexed = Indexed::quantize(&solid(4, 12, [1, 2, 3]));
    assert_eq!(indexed.encode(6..6), "");
    assert_eq!(indexed.encode(40..50), "");
    assert_eq!(decode(&indexed.encode(6..99)).height, 6, "a crop past the end is clamped");
}

/// More colors than the palette holds are reduced to the palette, and a background that covers
/// most of the picture keeps exactly its color.
///
/// The break this catches: a mean taken over unweighted colors, which drags a background toward
/// the few noisy pixels on it and leaves a visible rectangle against the cells around it.
#[test]
fn many_colors_reduce_to_a_palette_that_keeps_the_background() {
    let background = [30u8, 31, 36];
    let mut image = solid(100, 100, background);
    for i in 0..1_000u32 {
        let (x, y) = (i % 100, i / 100);
        let v = (i % 251) as u8;
        image.put_pixel(x, y, Rgba([v, 255 - v, (i % 97) as u8 * 2, 255]));
    }
    let indexed = Indexed::quantize(&image);
    assert!(indexed.palette.len() <= PALETTE_SIZE);
    let at = usize::from(indexed.pixels[99 * 100 + 99]);
    assert_eq!(indexed.palette[at], background);
}

/// A picture with few colors keeps all of them.
#[test]
fn a_picture_within_the_palette_is_lossless() {
    let mut image = solid(4, 1, [10, 20, 30]);
    image.put_pixel(1, 0, Rgba([40, 50, 60, 255]));
    image.put_pixel(2, 0, Rgba([70, 80, 90, 255]));
    let indexed = Indexed::quantize(&image);
    assert_eq!(indexed.palette.len(), 4 - 1);
    for (x, pixel) in image.pixels().enumerate() {
        assert_eq!(
            indexed.palette[usize::from(indexed.pixels[x])],
            [pixel.0[0], pixel.0[1], pixel.0[2]]
        );
    }
}
