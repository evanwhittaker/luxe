use super::*;

fn segment(kind: &'static str, label: &str, surface: StatusSurface) -> StatusSegment {
    StatusSegment { kind, glyph: None, label: label.into(), tone: StatusTone::Plain, surface }
}

/// The bar's full set: a bare reading, two recessed telemetry slabs, the raised thinking level and
/// the model as the hero.
fn full() -> Vec<StatusSegment> {
    vec![
        segment("context", "5%/200k", StatusSurface::Bare),
        segment("cost", "$0.44", StatusSurface::Recessed),
        segment("tokens", "↑12k ↓980", StatusSurface::Recessed),
        segment("thinking", "high", StatusSurface::Raised),
        segment("model", "Haiku", StatusSurface::Hero),
    ]
}

/// The left side's: the pull request on a raised slab, then the plan and the change count bare.
fn left() -> Vec<StatusSegment> {
    vec![
        segment("pull-request", "#138", StatusSurface::Raised),
        segment("todo", "3/8", StatusSurface::Bare),
        segment("changes", "5 files", StatusSurface::Bare),
    ]
}

fn kinds(html: &str) -> Vec<String> {
    html.split("class=\"seg seg-")
        .skip(1)
        .filter_map(|rest| rest.split([' ', '"']).next())
        .map(str::to_string)
        .collect()
}

/// The break this catches: the band losing its shape, so the reading gets a fill, neighbors of one
/// depth stop splitting on a dot, a change of depth stops ramping, or the band opens on a hard edge.
#[test]
fn a_band_steps_up_to_the_model() {
    let html = line(&[], &full(), None);
    let expected = "<span class=\"cluster right\"><span class=\"seg seg-context\">5%/200k</span>\
        <span class=\"band\"><span class=\"fade to-recessed\"></span>\
        <span class=\"seg seg-cost slab slab-recessed\">$0.44</span>\
        <span class=\"dot slab-recessed\">\u{b7}</span>\
        <span class=\"seg seg-tokens slab slab-recessed\">↑12k ↓980</span>\
        <span class=\"ramp from-recessed to-raised\"></span>\
        <span class=\"seg seg-thinking slab slab-raised\">high</span>\
        <span class=\"ramp from-raised to-hero\"></span>\
        <span class=\"seg seg-model slab slab-hero\">Haiku</span></span></span>";
    assert_eq!(html, expected);
}

/// The break this catches: the left side's band laid out as the right's, so it fades in from the
/// screen's edge and ends hard against the middle, instead of resting on the edge and fading out.
#[test]
fn the_left_band_rests_on_the_edge_and_fades_out() {
    let html = line(&left(), &[], None);
    let expected = "<span class=\"cluster left\"><span class=\"band\">\
        <span class=\"seg seg-pull-request slab slab-raised\">#138</span>\
        <span class=\"fade fade-out from-raised\"></span></span>\
        <span class=\"seg seg-todo\">3/8</span>\
        <span class=\"seg seg-changes\">5 files</span></span>\
        <span class=\"cluster right\"></span>";
    assert_eq!(html, expected);
}

/// The break this catches: the width sum drifting from what's drawn, so the fit drops a module the
/// line had room for or keeps one that wraps. 7 + gap + fade 3 + cost 7 + dot + tokens 11 + ramp 3
/// + thinking 6 + ramp 3 + model 7 is 49.
#[test]
fn the_width_counts_every_cell_drawn() {
    let segments = full();
    let all: Vec<_> = segments.iter().collect();
    assert_eq!(width(&all), 49);
    assert_eq!(kinds(&line(&[], &segments, Some(49))).len(), 5);
}

/// The break this catches: the left side or the gap between the sides left out of the sum, so the
/// line wraps with both. The left is slab 6 + fade 3 + gap + 3 + gap + 7, 21; the gap 2; the right
/// 49: 72.
#[test]
fn both_sides_and_the_gap_between_count() {
    let (left, right) = (left(), full());
    let (left, right): (Vec<_>, Vec<_>) = (left.iter().collect(), right.iter().collect());
    assert_eq!(line_width(&left, &right), 72);
    assert_eq!(line_width(&[], &right), 49, "no gap without a left side");
}

/// The break this catches: the fit dropping the wrong module first, squeezing the model out, or
/// dropping more than it needs to.
#[test]
fn a_narrow_line_drops_modules_in_the_bars_order() {
    let segments = full();
    let fitted = |cells| line(&[], &segments, Some(cells));
    assert_eq!(kinds(&fitted(48)), ["context", "cost", "thinking", "model"]);
    assert_eq!(kinds(&fitted(30)), ["context", "thinking", "model"]);
    assert_eq!(kinds(&fitted(3)), ["model"]);
}

/// The break this catches: the left side's modules outlasting the right's telemetry, or dropped
/// out of the bar's order: the plan goes before the change count, and the pull request after both.
#[test]
fn the_left_side_drops_in_the_bars_order_too() {
    let (left, right) = (left(), full());
    let fitted = |cells| kinds(&line(&left, &right, Some(cells)));
    let all = ["pull-request", "todo", "changes", "context", "cost", "thinking", "model"];
    assert_eq!(fitted(71), all);
    assert_eq!(fitted(49), ["pull-request", "changes", "context", "thinking", "model"]);
    assert_eq!(fitted(30), ["context", "thinking", "model"]);
}

/// The break this catches: a mark or tone lost on the way into a slab, or the words unescaped.
#[test]
fn a_slab_keeps_its_mark_tone_and_escaping() {
    let mut model = segment("model", "<no model>", StatusSurface::Hero);
    (model.glyph, model.tone) = (Some("&".into()), StatusTone::Warning);
    assert_eq!(
        segment_html(&model),
        "<span class=\"seg seg-model tone-warning slab slab-hero\">\
         <span class=\"seg-glyph\" aria-hidden=\"true\">&amp;</span> &lt;no model&gt;</span>"
    );
}
