//! sixel — an image as DEC sixel data, for terminals that draw pixels over blank cells.
//!
//! Sixel has no placeholder cells the way the kitty protocol does: the picture is painted where the
//! cursor stands and the terminal keeps it with the cells underneath. [`Indexed`] is the half of
//! that which is pure — an image reduced to a palette, and the bytes that draw any band of rows of
//! it. Where those bytes go, and when, is the store's business.
//!
//! Sixel carries at most a palette's worth of colors, so quantizing is the one lossy step. Median
//! cut over the colors that occur, weighted by how often they do: a screenshot is mostly a few
//! flat colors, and the background has to come out as exactly its color, or the picture shows as
//! a rectangle a shade off the cells around it.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::ops::Range;

/// Colors a picture is reduced to. Every sixel terminal measured offers at least this many.
const PALETTE_SIZE: usize = 256;

/// Pixel rows one sixel band carries.
const BAND: u32 = 6;

/// A run is written as `!n` and a character only when that is shorter than repeating it.
const MIN_RUN: usize = 4;

/// An image reduced to a palette: one byte a pixel, naming an entry of `palette`.
pub(super) struct Indexed {
    width: u32,
    height: u32,
    palette: Vec<[u8; 3]>,
    pixels: Vec<u8>,
}

impl Indexed {
    /// Reduces `image` to at most [`PALETTE_SIZE`] colors, ignoring alpha.
    ///
    /// The caller composites over the canvas first; a sixel is opaque, and a transparent pixel
    /// left in would show whatever the terminal last drew there.
    pub(super) fn quantize(image: &image::RgbaImage) -> Self {
        let packed: Vec<u32> = image.pixels().map(|pixel| pack(pixel.0)).collect();
        let mut counts: HashMap<u32, u32> = HashMap::new();
        for color in &packed {
            *counts.entry(*color).or_default() += 1;
        }
        let (palette, lookup) = build_palette(counts);
        let pixels = index_pixels(&packed, &lookup);
        Self { width: image.width(), height: image.height(), palette, pixels }
    }

    /// The sixel stream that draws pixel rows `rows`, as one complete DCS sequence.
    ///
    /// Rows outside the image are dropped. An empty range gives an empty string, never a sequence
    /// that draws nothing and still moves the cursor.
    pub(super) fn encode(&self, rows: Range<u32>) -> String {
        let rows = rows.start.min(self.height)..rows.end.min(self.height);
        if self.width == 0 || rows.is_empty() {
            return String::new();
        }
        // P1=9 asks for square pixels: the default is 2:1, which stretches a picture to twice the
        // height it was scaled for
        let mut out = format!("\x1bP9q\"1;1;{};{}", self.width, rows.len());
        self.push_palette(&mut out);
        let mut bands = Bands::new(self.width as usize);
        let tops: Vec<u32> = rows.clone().step_by(BAND as usize).collect();
        for (position, top) in tops.iter().enumerate() {
            self.encode_band(&mut bands, *top..(*top + BAND).min(rows.end), &mut out);
            if position + 1 < tops.len() {
                out.push('-');
            }
        }
        out.push_str("\x1b\\");
        out
    }

    fn push_palette(&self, out: &mut String) {
        for (index, [r, g, b]) in self.palette.iter().enumerate() {
            let _ = write!(out, "#{index};2;{};{};{}", percent(*r), percent(*g), percent(*b));
        }
    }

    fn encode_band(&self, bands: &mut Bands, rows: Range<u32>, out: &mut String) {
        bands.clear();
        let width = self.width as usize;
        for (bit, y) in rows.enumerate() {
            let line = &self.pixels[y as usize * width..(y as usize + 1) * width];
            for (x, color) in line.iter().enumerate() {
                bands.mark(*color, x, 1 << bit);
            }
        }
        bands.write(out);
    }
}

/// Which pixels of the band being written each color covers, one 6-bit mask a column.
struct Bands {
    width: usize,
    masks: Vec<Vec<u8>>,
    used: Vec<bool>,
    touched: Vec<u8>,
}

impl Bands {
    fn new(width: usize) -> Self {
        Self {
            width,
            masks: vec![Vec::new(); PALETTE_SIZE],
            used: vec![false; PALETTE_SIZE],
            touched: Vec::new(),
        }
    }

    fn clear(&mut self) {
        for color in self.touched.drain(..) {
            self.masks[usize::from(color)].fill(0);
            self.used[usize::from(color)] = false;
        }
    }

    fn mark(&mut self, color: u8, x: usize, bit: u8) {
        let masks = &mut self.masks[usize::from(color)];
        if masks.is_empty() {
            masks.resize(self.width, 0);
        }
        masks[x] |= bit;
        if !self.used[usize::from(color)] {
            self.used[usize::from(color)] = true;
            self.touched.push(color);
        }
    }

    /// Writes each color's pass over the band, `$` returning to the left edge between them.
    fn write(&mut self, out: &mut String) {
        self.touched.sort_unstable();
        for (position, color) in self.touched.iter().enumerate() {
            if position > 0 {
                out.push('$');
            }
            let masks = &self.masks[usize::from(*color)];
            let end = masks.iter().rposition(|mask| *mask != 0).map_or(0, |last| last + 1);
            let _ = write!(out, "#{color}");
            push_runs(out, &masks[..end]);
        }
    }
}

/// Writes `masks` as sixel characters, run-length encoding the repeats.
fn push_runs(out: &mut String, masks: &[u8]) {
    let mut at = 0;
    while at < masks.len() {
        let value = masks[at];
        let run = masks[at..].iter().take_while(|mask| **mask == value).count();
        let character = char::from(b'?' + value);
        if run >= MIN_RUN {
            let _ = write!(out, "!{run}{character}");
        } else {
            out.extend(std::iter::repeat_n(character, run));
        }
        at += run;
    }
}

/// An 8-bit channel as the 0–100 a sixel palette entry takes.
fn percent(channel: u8) -> u32 {
    (u32::from(channel) * 100 + 127) / 255
}

fn pack([r, g, b, _]: [u8; 4]) -> u32 {
    u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
}

fn unpack(color: u32) -> [u8; 3] {
    [(color >> 16) as u8, (color >> 8) as u8, color as u8]
}

/// Each pixel's palette entry.
///
/// Flat regions repeat a color for a long way, so the map is asked once per run; that is most of
/// what keeps a large screenshot cheap.
fn index_pixels(packed: &[u32], lookup: &HashMap<u32, u8>) -> Vec<u8> {
    let mut last = (u32::MAX, 0u8);
    packed
        .iter()
        .map(|color| {
            if last.0 != *color {
                last = (*color, lookup[color]);
            }
            last.1
        })
        .collect()
}

/// The palette for the colors in `counts`, and which entry each color maps to.
fn build_palette(counts: HashMap<u32, u32>) -> (Vec<[u8; 3]>, HashMap<u32, u8>) {
    let mut colors: Vec<(u32, u32)> = counts.into_iter().collect();
    // A fixed order, so the same picture always quantizes to the same bytes
    colors.sort_unstable();
    let buckets =
        if colors.len() <= PALETTE_SIZE { singletons(colors) } else { median_cut(colors) };
    let mut palette = Vec::with_capacity(buckets.len());
    let mut lookup = HashMap::new();
    for (index, bucket) in buckets.iter().enumerate() {
        palette.push(bucket.mean());
        for (color, _) in &bucket.colors {
            lookup.insert(*color, index as u8);
        }
    }
    (palette, lookup)
}

fn singletons(colors: Vec<(u32, u32)>) -> Vec<Bucket> {
    colors.into_iter().map(|color| Bucket::new(vec![color])).collect()
}

/// A set of colors that will share one palette entry.
struct Bucket {
    colors: Vec<(u32, u32)>,
    axis: usize,
    /// How much splitting this one would improve the picture, or zero when it cannot be split.
    score: u64,
}

impl Bucket {
    fn new(colors: Vec<(u32, u32)>) -> Self {
        let (mut low, mut high) = ([u8::MAX; 3], [0u8; 3]);
        let mut population = 0u64;
        for (color, count) in &colors {
            let rgb = unpack(*color);
            for channel in 0..3 {
                low[channel] = low[channel].min(rgb[channel]);
                high[channel] = high[channel].max(rgb[channel]);
            }
            population += u64::from(*count);
        }
        let (axis, range) = (0..3)
            .map(|channel| (channel, high[channel] - low[channel]))
            .max_by_key(|(_, range)| *range)
            .unwrap_or((0, 0));
        let score = if colors.len() < 2 { 0 } else { population * u64::from(range) };
        Self { colors, axis, score }
    }

    /// The average color, weighted by how many pixels each one covers.
    fn mean(&self) -> [u8; 3] {
        let (mut sum, mut total) = ([0u64; 3], 0u64);
        for (color, count) in &self.colors {
            let rgb = unpack(*color);
            for channel in 0..3 {
                sum[channel] += u64::from(rgb[channel]) * u64::from(*count);
            }
            total += u64::from(*count);
        }
        let total = total.max(1);
        sum.map(|channel| ((channel + total / 2) / total) as u8)
    }

    /// Splits along the widest channel at the pixel-count median.
    fn split(mut self) -> (Bucket, Bucket) {
        let axis = self.axis;
        self.colors.sort_unstable_by_key(|(color, _)| unpack(*color)[axis]);
        let total: u64 = self.colors.iter().map(|(_, count)| u64::from(*count)).sum();
        let mut seen = 0u64;
        let mut cut = self.colors.len() - 1;
        for (index, (_, count)) in self.colors.iter().enumerate() {
            seen += u64::from(*count);
            if seen * 2 >= total {
                cut = (index + 1).min(self.colors.len() - 1);
                break;
            }
        }
        let high = self.colors.split_off(cut);
        (Bucket::new(self.colors), Bucket::new(high))
    }
}

fn median_cut(colors: Vec<(u32, u32)>) -> Vec<Bucket> {
    let mut buckets = vec![Bucket::new(colors)];
    while buckets.len() < PALETTE_SIZE {
        let Some(widest) = (0..buckets.len())
            .filter(|index| buckets[*index].score > 0)
            .max_by_key(|index| buckets[*index].score)
        else {
            break;
        };
        let (low, high) = buckets.swap_remove(widest).split();
        buckets.push(low);
        buckets.push(high);
    }
    buckets
}

#[cfg(test)]
mod tests;
