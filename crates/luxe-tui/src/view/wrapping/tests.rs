use super::*;
use ratatui::style::Color;

fn rows(text: &str, width: u16, kind: MarkdownBlockKind) -> Vec<String> {
    let line = Line::from(text.to_string());
    let hang = match kind {
        MarkdownBlockKind::Prose => hanging_indent(&line, usize::from(width)),
        MarkdownBlockKind::Wide => None,
    };
    wrap_rows(line, width, hang).iter().map(Line::to_string).collect()
}

/// A line that names its own hang keeps it whatever its kind, every row, but never past half the
/// width.
#[test]
fn a_named_hang_holds_on_every_row_of_a_wide_line() {
    let line = Line::from("question  one two three four five six seven");
    let hang = named_hang(&line, 10, 22);
    let rows: Vec<String> = wrap_rows(line.clone(), 22, hang).iter().map(Line::to_string).collect();
    assert_eq!(
        rows,
        ["question  one two", "          three four", "          five six", "          seven"]
    );
    assert_eq!(named_hang(&line, 11, 22), None, "half the width or more is too far");
}

/// A wrapped list item carries on under its own first word, not back at the margin.
#[test]
fn a_wrapped_list_item_hangs_under_its_text() {
    let bullet = rows("• alpha beta gamma delta", 14, MarkdownBlockKind::Prose);
    assert_eq!(bullet, ["• alpha beta", "  gamma delta"]);
    let numbered = rows("12. alpha beta gamma", 14, MarkdownBlockKind::Prose);
    assert_eq!(numbered, ["12. alpha beta", "    gamma"]);
    let nested = rows("  - [ ] alpha beta gamma", 20, MarkdownBlockKind::Prose);
    assert_eq!(nested, ["  - [ ] alpha beta", "        gamma"]);
}

/// An indented run of names, the way `/tools` lists them, keeps its indent on every row.
#[test]
fn an_indented_run_keeps_its_indent() {
    let tools = rows("    read  write  edit  bash  grep", 21, MarkdownBlockKind::Prose);
    assert_eq!(tools, ["    read  write  edit", "    bash  grep"]);
}

/// A long word still splits at the width, and the split half hangs too.
#[test]
fn a_hard_split_word_hangs_and_fits() {
    let split = rows("• abcdefghijklmnop", 10, MarkdownBlockKind::Prose);
    assert_eq!(split, ["• abcdefgh", "  ijklmnop"]);
}

/// Code and wide rows keep exactly the columns they were drawn with.
#[test]
fn wide_rows_and_plain_prose_do_not_hang() {
    assert_eq!(rows("    alpha beta gamma", 12, MarkdownBlockKind::Wide)[1], "beta gamma");
    assert_eq!(rows("alpha beta gamma", 12, MarkdownBlockKind::Prose)[1], "gamma");
    assert_eq!(rows("-5 is alpha beta", 10, MarkdownBlockKind::Prose)[1], "alpha beta");
}

/// Past half the width a hang would leave too little row to be worth it.
#[test]
fn a_deep_indent_does_not_hang_on_a_narrow_row() {
    let deep = rows("          alpha beta", 16, MarkdownBlockKind::Prose);
    assert_eq!(deep[1], "beta");
}

/// The indent wears the style the line began in, so a card's band has no hole in it.
#[test]
fn the_indent_carries_the_leading_style() {
    let band = Style::default().bg(Color::Blue);
    let line = Line::from(Span::styled("• alpha beta gamma delta", band));
    let hang = hanging_indent(&line, 14);
    let wrapped = wrap_rows(line, 14, hang);
    assert!(wrapped[1].spans[0].content.starts_with("  gamma"), "{:?}", wrapped[1]);
    assert_eq!(wrapped[1].spans[0].style, band);
}

const PLATE: Color = Color::Rgb(30, 30, 40);

/// `line` in `depth` quotes, laid out on a 40-column canvas: its rows past the canvas indent, and
/// the prose width.
fn quoted(line: Line<'static>, depth: usize) -> (Vec<Line<'static>>, usize) {
    let canvas = super::super::conversation_canvas_widths(40);
    let mut layout = MarkdownLine::prose(line);
    let rule = Style::default().fg(Color::Gray);
    layout.quote = Some(QuoteMark { rules: vec![rule; depth], plate: Style::default().bg(PLATE) });
    let indented = canvas.prose_x > SAFE_GUTTER_WIDTH / 2;
    let rows = conversation_line(layout, canvas)
        .into_iter()
        .map(|row| {
            Line::from(row.spans.into_iter().skip(usize::from(indented)).collect::<Vec<_>>())
        })
        .collect();
    (rows, usize::from(canvas.prose_width))
}

/// A quoted line that wraps is ruled on every row, not just its first, and every row is on the
/// plate all the way out to the prose width.
///
/// The break this catches: the rule drawn once into the line before it wraps, or a row's plate
/// stopping where its words do.
#[test]
fn a_quoted_line_is_ruled_on_every_row_and_plated_to_the_width() {
    // words that leave every row short of the width, so a row's plate has to be padded out
    let (rows, width) = quoted(Line::from("alpha beta gamma delta ".repeat(5)), 1);
    assert!(rows.len() > 1, "the line wraps: {rows:?}");
    for row in &rows {
        assert!(row.to_string().starts_with("\u{258e} "), "ruled: {row:?}");
        assert!(!row.to_string().starts_with("\u{258e}  "), "no hang past the rule: {row:?}");
        assert_eq!(row.width(), width, "plated to the width: {row:?}");
        assert!(row.spans.iter().all(|span| span.style.bg == Some(PLATE)), "{row:?}");
    }
}

/// A quote in a quote draws a rule for each.
///
/// The break this catches: only the innermost quote's rule drawn.
#[test]
fn a_nested_quote_draws_a_rule_for_each_level() {
    let (rows, width) = quoted(Line::from("inner"), 2);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].to_string().starts_with("\u{258e} \u{258e} inner"), "{:?}", rows[0]);
    assert_eq!(rows[0].width(), width);
}

/// A word that brings its own background keeps it on the plate.
///
/// The break this catches: the plate painted over every span, hiding a highlighted word.
#[test]
fn a_word_with_its_own_background_keeps_it_on_the_plate() {
    let marked = Style::default().bg(Color::Yellow);
    let (rows, _) = quoted(Line::from(vec![Span::raw("see "), Span::styled("this", marked)]), 1);
    let this = rows[0].spans.iter().find(|span| span.content == "this").expect("the word");
    assert_eq!(this.style.bg, Some(Color::Yellow));
}
