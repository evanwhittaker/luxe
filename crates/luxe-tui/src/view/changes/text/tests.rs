use super::*;
use crate::fixture::{seed_file_view, seeded_app, FixtureScenario};

const SOURCE: &str = "let total = 1;\nlet other = 2;\n\tTotal += 3;\n";

/// The File view's rows for `SOURCE`, wide enough that nothing wraps.
fn rows_with(found: Option<&str>) -> (Vec<Line<'static>>, ratatui::style::Style) {
    let mut app = seeded_app("nord", FixtureScenario::ChatIdle).expect("chat fixture");
    seed_file_view(&mut app, "src/lib.rs", SOURCE, None);
    let view = app.changes.as_mut().expect("a preview");
    view.found = found.and_then(|query| crate::changes::FoundQuery::new(query, Default::default()));
    let view = app.changes.as_ref().expect("a preview");
    let TextLoad::Ready(file) = &view.text else { panic!("the text is seeded") };
    let lines: Vec<&str> = file.text.lines().collect();
    let columns = TextColumns { numbers: 1, gutter: 4, code: 60, width: 64, viewport: 10 };
    let rows = text_rows(&app, view, file, &lines, 0, &columns).rows;
    (rows, app.theme_styles.match_bg())
}

/// What on `row` wears the match band.
fn lit(row: &Line<'static>, band: ratatui::style::Style) -> Vec<String> {
    row.spans
        .iter()
        .filter(|span| span.style.bg == band.bg)
        .map(|span| span.content.to_string())
        .collect()
}

/// The break this catches: a hit opened from Find with nothing lit, the query lit on lines it
/// isn't on, or lit after the tab the line starts with shifted it off the word.
#[test]
fn the_query_find_opened_with_is_lit_where_it_matches() {
    let (rows, band) = rows_with(Some("total"));
    assert_eq!(lit(&rows[0], band), ["total"]);
    assert!(lit(&rows[1], band).is_empty(), "a line without it stays plain");
    assert_eq!(lit(&rows[2], band), ["Total"], "found past a tab, in any case");
}

/// The break this catches: matches lit in a file nobody searched.
#[test]
fn a_file_opened_without_a_query_lights_nothing() {
    let (rows, band) = rows_with(None);
    assert!(rows.iter().all(|row| lit(row, band).is_empty()));
}
