//! Status line — the TUI's bar under the composer, cut into the same slabs.
//!
//! The facts come worded from the [`Renderer`](super::Renderer) with the surface the TUI gives each.
//! On the right, the context reading bare, then a band that steps up from the canvas through the
//! recessed telemetry and the raised thinking level to the model, lit in the accent and flush
//! against the screen's edge. On the left, the mirror: the pull request's slab against the edge,
//! fading out toward the middle, then the plan and the change count bare. Same-depth neighbors
//! split on a dim `·`; a change of depth ramps across three cells. Fitted to the cells the app
//! says it has, dropping modules in the bar's order rather than squeezing them, so a phone shows
//! fewer facts, never cut ones. The colors are the stylesheet's, from the theme's derived
//! properties; a flat `statusline.style` draws it all bare there.

use super::escape;

/// One fact on the status line, as words; the bridge escapes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSegment {
    /// Which fact, as the class the app styles it by: `context`, `cost`, `tokens`, `thinking` or
    /// `model`
    pub kind: &'static str,
    /// The mark in front of it, if any
    pub glyph: Option<String>,
    /// The words
    pub label: String,
    /// How loudly to color it
    pub tone: StatusTone,
    /// What it sits on
    pub surface: StatusSurface,
}

/// How loudly a status segment is colored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTone {
    /// Body text
    Plain,
    /// The theme's warning color
    Warning,
    /// The theme's error color
    Error,
}

/// What a status segment sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusSurface {
    /// Straight on the bar
    Bare,
    /// The accent, inked in the canvas
    Hero,
    /// One step back from the accent
    Raised,
    /// Two steps back
    Recessed,
}

impl StatusSurface {
    fn class(self) -> Option<&'static str> {
        match self {
            Self::Bare => None,
            Self::Hero => Some("hero"),
            Self::Raised => Some("raised"),
            Self::Recessed => Some("recessed"),
        }
    }
}

/// Cells each change of color takes, as the TUI's bar spends them.
const FADE_CELLS: usize = 3;

/// Cells at least between the two sides, as the TUI keeps its clusters apart.
const CLUSTER_GAP: usize = 2;

/// The kinds the fit drops, first to last, as the TUI's bar drops them: trivia before the reading
/// that says whether the window is about to run out, and the plan before the change count, the
/// pull request after both. The model always stays.
const DROP_ORDER: [&str; 7] =
    ["tokens", "cost", "todo", "changes", "pull-request", "context", "thinking"];

/// Which edge a side's band rests against: the left side's band starts at the screen's edge and
/// fades out toward the middle, the right side's fades in from the middle and ends at the edge.
#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
}

/// The line for `left` and `right`, fitted to `cells` when the app said how many it has.
pub(super) fn line(left: &[StatusSegment], right: &[StatusSegment], cells: Option<u16>) -> String {
    let mut left: Vec<&StatusSegment> = left.iter().collect();
    let mut right: Vec<&StatusSegment> = right.iter().collect();
    if let Some(cells) = cells {
        fit(&mut left, &mut right, usize::from(cells));
    }
    let left = (!left.is_empty()).then(|| cluster_html(&left, Side::Left));
    left.unwrap_or_default() + &cluster_html(&right, Side::Right)
}

fn fit(left: &mut Vec<&StatusSegment>, right: &mut Vec<&StatusSegment>, cells: usize) {
    for kind in DROP_ORDER {
        if line_width(left, right) <= cells {
            return;
        }
        left.retain(|segment| segment.kind != kind);
        right.retain(|segment| segment.kind != kind);
    }
}

fn line_width(left: &[&StatusSegment], right: &[&StatusSegment]) -> usize {
    let gap = if left.is_empty() || right.is_empty() { 0 } else { CLUSTER_GAP };
    width(left) + gap + width(right)
}

/// What the line is made of: bare segments, and bands of slabs running together.
enum Run<'a> {
    Bare(&'a StatusSegment),
    Band(Vec<&'a StatusSegment>),
}

fn runs<'a>(segments: &[&'a StatusSegment]) -> Vec<Run<'a>> {
    let mut runs = Vec::new();
    for &segment in segments {
        match (segment.surface, runs.last_mut()) {
            (StatusSurface::Bare, _) => runs.push(Run::Bare(segment)),
            (_, Some(Run::Band(band))) => band.push(segment),
            _ => runs.push(Run::Band(vec![segment])),
        }
    }
    runs
}

/// The cells `segments` take laid out: a cell between runs, and in a band its fade, each slab's
/// pads, and a dot or a ramp between neighbors. A side's fade is the same three cells whichever
/// end it's at.
fn width(segments: &[&StatusSegment]) -> usize {
    let runs = runs(segments);
    let gaps = runs.len().saturating_sub(1);
    gaps + runs.iter().map(run_width).sum::<usize>()
}

fn run_width(run: &Run<'_>) -> usize {
    let band = match run {
        Run::Bare(segment) => return words_width(segment),
        Run::Band(band) => band,
    };
    let slabs: usize = band.iter().map(|segment| words_width(segment) + 2).sum();
    let joins: usize = band.windows(2).map(|pair| join_width(pair[0], pair[1])).sum();
    FADE_CELLS + slabs + joins
}

fn words_width(segment: &StatusSegment) -> usize {
    segment.label.chars().count() + segment.glyph.as_ref().map_or(0, |_| 2)
}

fn join_width(before: &StatusSegment, after: &StatusSegment) -> usize {
    if before.surface == after.surface {
        1
    } else {
        FADE_CELLS
    }
}

fn cluster_html(segments: &[&StatusSegment], side: Side) -> String {
    let class = match side {
        Side::Left => "left",
        Side::Right => "right",
    };
    let runs: String = runs(segments).iter().map(|run| run_html(run, side)).collect();
    format!("<span class=\"cluster {class}\">{runs}</span>")
}

fn run_html(run: &Run<'_>, side: Side) -> String {
    let band = match run {
        Run::Bare(segment) => return segment_html(segment),
        Run::Band(band) => band,
    };
    let mut slabs = segment_html(band[0]);
    for pair in band.windows(2) {
        slabs += &join_html(pair[0], pair[1]);
        slabs += &segment_html(pair[1]);
    }
    let fade = |end: &StatusSegment| end.surface.class().unwrap_or_default();
    let band = match side {
        Side::Left => {
            let last = band.last().map_or("", |end| fade(end));
            format!("{slabs}<span class=\"fade fade-out from-{last}\"></span>")
        }
        Side::Right => format!("<span class=\"fade to-{}\"></span>{slabs}", fade(band[0])),
    };
    format!("<span class=\"band\">{band}</span>")
}

fn join_html(before: &StatusSegment, after: &StatusSegment) -> String {
    let (from, to) =
        (before.surface.class().unwrap_or_default(), after.surface.class().unwrap_or_default());
    if from == to {
        // on the slab it splits, so the band runs unbroken under it
        return format!("<span class=\"dot slab-{to}\">\u{b7}</span>");
    }
    format!("<span class=\"ramp from-{from} to-{to}\"></span>")
}

fn segment_html(segment: &StatusSegment) -> String {
    let tone = match segment.tone {
        StatusTone::Plain => "",
        StatusTone::Warning => " tone-warning",
        StatusTone::Error => " tone-error",
    };
    let slab = segment.surface.class().map(|depth| format!(" slab slab-{depth}"));
    let glyph = segment.glyph.as_deref().map(|glyph| {
        format!("<span class=\"seg-glyph\" aria-hidden=\"true\">{}</span> ", escape(glyph))
    });
    format!(
        "<span class=\"seg seg-{}{tone}{}\">{}{}</span>",
        segment.kind,
        slab.unwrap_or_default(),
        glyph.unwrap_or_default(),
        escape(&segment.label)
    )
}

#[cfg(test)]
mod tests;
