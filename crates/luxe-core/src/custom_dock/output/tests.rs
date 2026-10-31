use super::*;

fn span(text: &str, ink: DockInk) -> DockSpan {
    DockSpan { text: text.into(), ink }
}

#[test]
fn json_rows_keep_their_labels_and_inks() {
    let rows = parse_output(
        r#"{"rows": [{"label": "weekly", "spans": [{"text": "▰▰▱", "ink": "accent"}, {"text": " 30%"}]}]}"#,
    )
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].label.as_deref(), Some("weekly"));
    assert_eq!(rows[0].spans, [span("▰▰▱", DockInk::Accent), span(" 30%", DockInk::Text)]);
}

#[test]
fn plain_text_is_a_row_per_line() {
    let rows = parse_output("30% of the week\n5h: 80%\n").unwrap();
    let texts: Vec<&str> = rows.iter().map(|row| row.spans[0].text.as_str()).collect();
    assert_eq!(texts, ["30% of the week", "5h: 80%"]);
    assert!(rows.iter().all(|row| row.label.is_none()));
}

#[test]
fn blank_lines_draw_nothing() {
    assert_eq!(parse_output("one\n\n   \ntwo").unwrap().len(), 2);
    assert!(parse_output("").unwrap().is_empty());
}

#[test]
fn json_that_is_not_the_shape_is_a_failure() {
    let failure = parse_output(r#"{"rows": "nope"}"#).unwrap_err();
    assert!(matches!(failure, DockOutputError::Invalid(_)), "{failure:?}");
    assert!(parse_output("{ not json").is_err());
}

#[test]
fn an_unknown_ink_is_kept_as_unknown_not_refused() {
    let rows =
        parse_output(r#"{"rows": [{"spans": [{"text": "x", "ink": "chartreuse"}]}]}"#).unwrap();
    assert_eq!(rows[0].spans[0].ink, DockInk::Unknown);
}

#[test]
fn rows_past_the_cap_are_dropped() {
    let many: String = (0..20).map(|n| format!("row {n}\n")).collect();
    assert_eq!(parse_output(&many).unwrap().len(), MAX_ROWS);
}

/// A program must not be able to write to the terminal through luxe: a color, a cursor move, a
/// title change, a bell and a carriage return all come out as nothing.
#[test]
fn escape_sequences_and_control_characters_are_stripped() {
    assert_eq!(clean_text("\u{1b}[31mred\u{1b}[0m"), "red");
    assert_eq!(clean_text("a\u{1b}[2Jb"), "ab");
    assert_eq!(clean_text("\u{1b}]0;pwned\u{7}title"), "title");
    assert_eq!(clean_text("ring\u{7}\r\n"), "ring");
    assert_eq!(clean_text("lone\u{1b}"), "lone");
    assert_eq!(clean_text("c1\u{9b}31m"), "c131m", "a C1 control is dropped too");
}

#[test]
fn invisible_marks_are_stripped_and_wide_glyphs_kept() {
    assert_eq!(clean_text("abc\u{202E}def"), "abcdef", "a bidi override would reorder the row");
    assert_eq!(clean_text("zero\u{200B}width"), "zerowidth");
    assert_eq!(clean_text("▰▰▱ 漢字 🙂"), "▰▰▱ 漢字 🙂");
}

#[test]
fn tabs_become_spaces() {
    assert_eq!(clean_text("a\tb"), "a b");
}

#[test]
fn a_long_string_is_cut() {
    assert_eq!(clean_text(&"x".repeat(1_000)).chars().count(), MAX_TEXT_CHARS);
}

/// Sanitizing happens to JSON rows too, labels included.
#[test]
fn json_strings_are_cleaned() {
    let rows =
        parse_output(r#"{"rows": [{"label": "\u001b[1mweek", "spans": [{"text": "\u0007"}]}]}"#)
            .unwrap();
    assert_eq!(rows[0].label.as_deref(), Some("week"));
    assert!(rows[0].spans.is_empty(), "a span with nothing left is dropped");
}
