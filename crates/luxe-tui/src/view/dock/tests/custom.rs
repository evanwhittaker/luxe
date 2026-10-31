//! Custom sections — blocks a `dock.custom` program feeds, placed and drawn like the built-ins.

use super::sections::{above, app_with, position, regions};
use super::*;
use crate::app::Msg;
use luxe_core::config::{CustomDockSection, DockPlace};
use luxe_core::custom_dock::{DockInk, DockRow, DockSpan};
use luxe_core::Event;

/// An app showing `sections`, with each of `custom` defined at its place.
fn app_with_custom(sections: &[&str], custom: &[(&str, DockPlace)]) -> App {
    let mut app = app_with(sections);
    let define = |(name, place): &(&str, DockPlace)| {
        let definition = CustomDockSection { place: Some(*place), ..CustomDockSection::default() };
        (name.to_string(), definition)
    };
    app.config.dock.custom = Some(custom.iter().map(define).collect());
    app
}

fn row(label: Option<&str>, text: &str) -> DockRow {
    let span = DockSpan { text: text.into(), ink: DockInk::Accent };
    DockRow { label: label.map(Into::into), spans: vec![span] }
}

/// What a program answering `rows` for `name` sends, through the reducer.
fn answer(app: &mut App, name: &str, rows: Vec<DockRow>, failure: Option<&str>) {
    let failure = failure.map(Into::into);
    let answer = luxe_core::custom_dock::DockAnswer { name: name.into(), rows, failure };
    app.update(Msg::Engine(Event::DockSection(answer)));
}

#[test]
fn a_bottom_section_draws_its_heading_and_rows_in_list_order() {
    let mut app = app_with_custom(&["model", "quota", "session"], &[("quota", DockPlace::Bottom)]);
    answer(&mut app, "quota", vec![row(Some("weekly"), "30%")], None);
    let (_, middle, footer) = regions(&app);
    assert!(above(&footer, "MODEL", "QUOTA") && above(&footer, "QUOTA", "SESSION"), "{footer:?}");
    let weekly = footer.iter().find(|line| line.starts_with("weekly")).expect("the row");
    assert!(weekly.ends_with("30%"), "{weekly:?}");
    assert_eq!(position(&middle, "QUOTA"), None, "one region only");
}

#[test]
fn a_middle_section_lands_in_the_middle_in_list_order() {
    let mut app = app_with_custom(&["ci", "plan", "model"], &[("ci", DockPlace::Middle)]);
    answer(&mut app, "ci", vec![row(None, "green")], None);
    let (_, middle, footer) = regions(&app);
    assert!(above(&middle, "CI", "PLAN"), "{middle:?}");
    assert!(position(&middle, "green").is_some(), "{middle:?}");
    assert_eq!(position(&footer, "CI"), None);
}

/// Over the conversation the dock leaves MODEL, USAGE and SESSION to the statusline, which carries
/// them. A custom section isn't on the bar, so leaving it out would lose it.
#[test]
fn a_custom_section_stays_when_the_statusline_carries_the_rest() {
    let mut app = app_with_custom(&["model", "quota", "session"], &[("quota", DockPlace::Bottom)]);
    answer(&mut app, "quota", vec![row(Some("weekly"), "30%")], None);
    let (.., footer) = super::sections::overlay_regions(&app);
    assert!(position(&footer, "QUOTA").is_some(), "{footer:?}");
    assert_eq!(position(&footer, "MODEL"), None, "the bar carries MODEL: {footer:?}");
}

/// A program that hasn't answered yet has nothing to say, so the dock doesn't save it a row.
#[test]
fn nothing_is_drawn_before_the_first_answer() {
    let app = app_with_custom(&["model", "quota"], &[("quota", DockPlace::Bottom)]);
    let (.., footer) = regions(&app);
    assert_eq!(position(&footer, "QUOTA"), None, "{footer:?}");
}

/// Printing nothing is how a program says it has nothing to show, like plan bars on an API key:
/// the section goes, rather than leaving a heading over nothing. A program that showed rows and
/// then prints none goes too.
#[test]
fn a_program_that_prints_nothing_hides_its_section() {
    let mut app = app_with_custom(&["model", "quota"], &[("quota", DockPlace::Bottom)]);
    answer(&mut app, "quota", vec![], None);
    let (.., footer) = regions(&app);
    assert_eq!(position(&footer, "QUOTA"), None, "{footer:?}");
    answer(&mut app, "quota", vec![row(None, "30%")], None);
    answer(&mut app, "quota", vec![], None);
    let (.., footer) = regions(&app);
    assert_eq!(position(&footer, "QUOTA"), None, "{footer:?}");
}

#[test]
fn a_failure_keeps_the_last_rows_and_says_why_under_them() {
    let mut app = app_with_custom(&["quota"], &[("quota", DockPlace::Bottom)]);
    answer(&mut app, "quota", vec![row(Some("weekly"), "30%")], None);
    answer(&mut app, "quota", vec![], Some("timed out after 2000 ms"));
    let (.., footer) = regions(&app);
    assert!(above(&footer, "weekly", "last run: timed out"), "{footer:?}");
}

/// The name in `sections` is what places a section; a definition alone, unlisted, draws nothing.
#[test]
fn a_defined_section_left_out_of_the_list_is_not_drawn() {
    let mut app = app_with_custom(&["model"], &[("quota", DockPlace::Bottom)]);
    answer(&mut app, "quota", vec![row(None, "30%")], None);
    let (.., footer) = regions(&app);
    assert_eq!(position(&footer, "QUOTA"), None, "{footer:?}");
}

/// How someone draws their own SESSION: a custom section takes the built-in's place.
#[test]
fn a_custom_section_named_like_a_built_in_replaces_it() {
    let mut app = app_with_custom(&["model", "session"], &[("session", DockPlace::Bottom)]);
    app.config.dock.custom.as_mut().unwrap().get_mut("session").unwrap().title =
        Some("MINE".into());
    app.status.session.total.calls = 3;
    answer(&mut app, "session", vec![row(None, "my numbers")], None);
    let (.., footer) = regions(&app);
    assert!(position(&footer, "MINE").is_some(), "{footer:?}");
    assert_eq!(position(&footer, "SESSION"), None, "{footer:?}");
}

/// A row that wrapped would shift everything pinned under it.
#[test]
fn a_long_row_is_cut_to_the_rail() {
    let mut app = app_with_custom(&["quota"], &[("quota", DockPlace::Bottom)]);
    let long = DockRow {
        label: Some("a-very-long-label-indeed".into()),
        spans: vec![
            DockSpan { text: "▰".repeat(30), ink: DockInk::Accent },
            DockSpan { text: " 界界界界界界界界界界".into(), ink: DockInk::Unknown },
        ],
    };
    answer(&mut app, "quota", vec![long], None);
    let (footer, _) = footer_lines(&app, true, DockPresentation::SideBySide);
    let room = row_width(&app);
    for line in &footer {
        assert!(text(line).width() <= room, "{:?} is wider than {room}", text(line));
    }
}

/// Programs hear the width their rows are drawn at once per change: a redraw at the same width
/// sends nothing, since each report reruns them, and a dragged rail sends the new one.
#[test]
fn the_row_widths_reach_the_engine_once_per_change() {
    let (tx, mut engine) = tokio::sync::mpsc::channel(4);
    let mut app = App::new(tx, KeyMap::defaults(), Config::default());
    let first = reported(&mut app, &mut engine).expect("the first frame reports");
    assert_eq!((first.dock, first.dock_label, first.statusline), (40, 15, 32));
    assert_eq!(reported(&mut app, &mut engine), None, "the same width again");
    app.dock_rail.set(62);
    assert_eq!(reported(&mut app, &mut engine).map(|widths| widths.dock), Some(60));
}

/// The plan-bars picture was recorded from a run told the default rail's widths, so the fixture's
/// rail has to give the same ones, and the row then draws the script's whole answer. A recording
/// made at another width, or a rail that moved, cuts the tail off the published picture.
#[test]
fn the_plan_bars_picture_draws_its_recorded_row_whole() {
    use crate::fixture::{seeded_app, FixtureScenario};
    let mut app = seeded_app("luxe", FixtureScenario::DockPlanBars).expect("the fixture");
    app.update(Msg::TerminalResized { width: 140, height: 34 });
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(140, 34)).unwrap();
    terminal.draw(|frame| crate::view::render(frame, &mut app)).unwrap();
    let widths = custom_row_widths(&app);
    assert_eq!((widths.dock, widths.dock_label), (40, 15), "the widths it was recorded at");
    let buffer = terminal.backend().buffer();
    let row = (0..34)
        .map(|y| (0..140).map(|x| buffer[(x, y)].symbol()).collect::<String>())
        .find(|line| line.contains("monthly"))
        .expect("the QUOTA row");
    assert!(row.contains("monthly        ▰▰▰▰▰▱▱▱▱▱▱▱▱▱▱▱ 32% · 3d"), "{row:?}");
}

/// What one frame's report sent the engine, if anything.
fn reported(
    app: &mut App,
    engine: &mut tokio::sync::mpsc::Receiver<luxe_core::Action>,
) -> Option<luxe_core::custom_dock::RowWidths> {
    report_custom_row_widths(app);
    match engine.try_recv() {
        Ok(luxe_core::Action::CustomRowWidths(widths)) => Some(widths),
        _ => None,
    }
}
