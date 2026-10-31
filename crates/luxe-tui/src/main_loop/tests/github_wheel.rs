//! The wheel in `/github` and `/pr`: it scrolls the open tab, walks the list, and leaves the
//! conversation hidden behind them alone.

use super::*;
use crate::mouse::ScrollRegionId;
use crossterm::event::{MouseEvent, MouseEventKind};
use ratatui::{backend::TestBackend, Terminal};

const WIDTH: u16 = 200;
const HEIGHT: u16 = 40;

fn draw(app: &mut crate::app::App) {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("a terminal");
    terminal.draw(|frame| crate::view::render(frame, app)).expect("a frame");
}

/// The first cell the last frame gave to `region`, or `None` for a cell no GitHub region holds.
fn cell_of(app: &crate::app::App, region: Option<ScrollRegionId>) -> (u16, u16) {
    let hit = app.hit.borrow();
    let cells = (0..HEIGHT).flat_map(|row| (0..WIDTH).map(move |col| (col, row)));
    let github = |at| matches!(at, Some(ScrollRegionId::GithubList | ScrollRegionId::GithubDetail));
    let found = cells.into_iter().find(|(col, row)| match region {
        Some(_) => hit.scroll_at(*col, *row) == region,
        None => !github(hit.scroll_at(*col, *row)),
    });
    found.unwrap_or_else(|| panic!("no cell for {region:?}"))
}

fn wheel_down(app: &mut crate::app::App, (column, row): (u16, u16)) {
    let notch =
        MouseEvent { kind: MouseEventKind::ScrollDown, column, row, modifiers: KeyModifiers::NONE };
    let action = route_mouse_at(app, &notch, std::time::Instant::now());
    apply_route_action(app, action, &mut TuiExit::Quit);
}

/// `/github` on its detail, with a description long enough to need scrolling.
fn github_with_long_detail() -> crate::app::App {
    let mut app = seeded_app("nord", FixtureScenario::ChatIdle).expect("the fixture");
    crate::fixture::seed_github(&mut app, "github-detail");
    let panel = app.overlays.github.as_mut().expect("the panel");
    let detail = panel.detail_mut().expect("the row's detail");
    if let crate::github_detail::Subject::PullRequest(pr) = &mut detail.subject {
        pr.body = (1..=80).map(|n| format!("Paragraph {n}.\n\n")).collect();
    }
    app.update(Msg::TerminalResized { width: WIDTH, height: HEIGHT });
    draw(&mut app);
    app
}

fn detail_scroll(app: &crate::app::App) -> usize {
    app.overlays.github.as_ref().and_then(|panel| panel.detail()).map_or(0, |detail| detail.scroll)
}

/// The break this catches: the wheel over a pull request's detail doing nothing, so a long
/// description or comment can only be read from the keyboard.
#[test]
fn the_wheel_scrolls_the_open_tab() {
    let mut app = github_with_long_detail();
    let over_detail = cell_of(&app, Some(ScrollRegionId::GithubDetail));
    wheel_down(&mut app, over_detail);
    assert_eq!(detail_scroll(&app), 3);
}

/// The break this catches: the wheel over the list scrolling the detail, or nothing, rather than
/// walking the rows the way `↓` does.
#[test]
fn the_wheel_walks_the_list() {
    let mut app = github_with_long_detail();
    let before = app.overlays.github.as_ref().map(|panel| panel.selected);
    let over_list = cell_of(&app, Some(ScrollRegionId::GithubList));
    wheel_down(&mut app, over_list);
    let panel = app.overlays.github.as_ref().expect("the panel");
    assert_eq!(Some(panel.selected), before.map(|row| row + 1));
    assert_eq!(panel.focus, crate::github_panel::Focus::List, "the list takes the focus");
}

/// The break this catches: the wheel over the panel's chrome scrolling the conversation hidden
/// behind it.
#[test]
fn the_wheel_elsewhere_leaves_the_conversation_alone() {
    let mut app = github_with_long_detail();
    let (column, row) = cell_of(&app, None);
    let notch =
        MouseEvent { kind: MouseEventKind::ScrollDown, column, row, modifiers: KeyModifiers::NONE };
    let action = route_mouse_at(&mut app, &notch, std::time::Instant::now());
    assert!(matches!(action, RouteAction::None), "{action:?}");
}

/// The break this catches: `/pr`'s card, a detail of its own, not taking the wheel.
#[test]
fn the_wheel_scrolls_the_pull_request_card() {
    let mut app = seeded_app("nord", FixtureScenario::ChatIdle).expect("the fixture");
    crate::fixture::seed_github(&mut app, "pr");
    let card = app.overlays.pull_request_card.as_mut().expect("the card");
    let crate::github_detail::CardState::Ready(detail) = &mut card.state else { panic!("ready") };
    if let crate::github_detail::Subject::PullRequest(pr) = &mut detail.subject {
        pr.body = (1..=80).map(|n| format!("Paragraph {n}.\n\n")).collect();
    }
    app.update(Msg::TerminalResized { width: WIDTH, height: HEIGHT });
    draw(&mut app);
    let over_detail = cell_of(&app, Some(ScrollRegionId::GithubDetail));
    wheel_down(&mut app, over_detail);
    let card = app.overlays.pull_request_card.as_ref().expect("the card");
    let crate::github_detail::CardState::Ready(detail) = &card.state else { panic!("ready") };
    assert_eq!(detail.scroll, 3);
}
