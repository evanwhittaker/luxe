use super::*;
use luxe_core::project_search::SearchOptions;
use ratatui::style::{Color, Style};

fn lit() -> Style {
    Style::default().bg(Color::Yellow)
}

fn red() -> Style {
    Style::default().fg(Color::Red)
}

/// Each span as `(text, lit)`, to read what a result paints.
fn shape(spans: &[Span<'static>]) -> Vec<(String, bool)> {
    spans
        .iter()
        .map(|span| (span.content.to_string(), span.style.bg == Some(Color::Yellow)))
        .collect()
}

fn matcher(query: &str) -> QueryMatcher {
    QueryMatcher::new(query, SearchOptions::default()).expect("a literal compiles")
}

/// The break this catches: lighting the whole span a match touches rather than the match, so a
/// one-word hit paints the token around it.
#[test]
fn a_match_inside_a_span_lights_only_itself() {
    let spans = vec![Span::styled("let total = 1;", red())];
    let out = light_ranges(spans, &[MatchRange { start: 4, end: 9 }], lit());
    assert_eq!(
        shape(&out),
        [("let ".into(), false), ("total".into(), true), (" = 1;".into(), false)]
    );
    assert_eq!(out[1].style.fg, Some(Color::Red), "the syntax color stays under the band");
}

/// The break this catches: a match across two highlighted tokens lit in only the first, since
/// ranges are offsets into the whole line and not into any one span.
#[test]
fn a_match_across_spans_lights_each_part() {
    let spans = vec![Span::styled("foo", red()), Span::raw("bar baz")];
    let out = light_ranges(spans, &[MatchRange { start: 2, end: 5 }], lit());
    assert_eq!(
        shape(&out),
        [("fo".into(), false), ("o".into(), true), ("ba".into(), true), ("r baz".into(), false)]
    );
}

/// The break this catches: only the first of several matches lit.
#[test]
fn every_range_is_lit() {
    let spans = vec![Span::raw("a x a")];
    let ranges = [MatchRange { start: 0, end: 1 }, MatchRange { start: 4, end: 5 }];
    let out = light_ranges(spans, &ranges, lit());
    assert_eq!(shape(&out), [("a".into(), true), (" x ".into(), false), ("a".into(), true)]);
}

/// The break this catches: a range that ends inside a character splitting it and panicking,
/// which a match computed on other text (a line with its tabs expanded) can do.
#[test]
fn a_range_off_a_character_boundary_is_left_alone() {
    let spans = vec![Span::raw("é!")];
    let out = light_ranges(spans.clone(), &[MatchRange { start: 1, end: 2 }], lit());
    assert_eq!(shape(&out), shape(&spans));
}

/// The break this catches: an empty range taken as a match, which rebuilds the spans for
/// nothing and drops an empty one a caller placed on purpose.
#[test]
fn an_empty_range_leaves_the_spans_as_they_were() {
    let spans = vec![Span::raw(""), Span::styled("abc", red())];
    let out = light_ranges(spans.clone(), &[MatchRange { start: 1, end: 1 }], lit());
    assert_eq!(out, spans);
}

fn numbered(count: usize) -> Vec<Line<'static>> {
    (0..count).map(|index| Line::from(index.to_string())).collect()
}

fn first_shown(lines: &[Line<'static>]) -> String {
    lines[0].spans.iter().map(|span| span.content.as_ref()).collect()
}

/// The break this catches: a window that scrolls for a match it could already show, hiding the
/// lines above it for nothing, or one that never scrolls at all.
#[test]
fn the_window_scrolls_only_for_a_match_past_its_bottom() {
    let within = window_on(numbered(30), Some(9), 10);
    assert_eq!((first_shown(&within).as_str(), within.len()), ("0", 10));
    let past = window_on(numbered(30), Some(10), 10);
    assert_eq!(first_shown(&past), "7", "the match a third of the way down");
    assert_eq!(window_on(numbered(30), None, 10).len(), 10);
}

/// The break this catches: a match on a later line missed, or a line with none taken for one.
#[test]
fn the_first_match_is_the_first_line_that_has_one() {
    let lines = vec![Line::from("none"), Line::from(vec![Span::raw("a "), Span::raw("Foo")])];
    assert_eq!(first_match(&lines, &matcher("foo")), Some(1));
    assert_eq!(first_match(&lines, &matcher("bar")), None);
}

/// The break this catches: lines without a match rebuilt or restyled, or a match on a later line
/// missed because only the first line was looked at.
#[test]
fn light_lines_lights_each_line_on_its_own() {
    let lines = vec![Line::from("nothing here"), Line::from(vec![Span::raw("a Foo b")])];
    let out = light_lines(lines, &matcher("foo"), lit());
    assert_eq!(shape(&out[0].spans), [("nothing here".into(), false)]);
    assert_eq!(
        shape(&out[1].spans),
        [("a ".into(), false), ("Foo".into(), true), (" b".into(), false)]
    );
}
