use crate::app::App;
use crate::fixture::{seed_github, seeded_app, FixtureScenario};
use crate::github_detail::DetailTab;
use crate::mention_flyout::MentionTab;
use crate::mouse::{MouseTarget, OverlayList, TabStrip};
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{backend::TestBackend, buffer::Buffer, layout::Rect, Terminal};

fn chat() -> App {
    seeded_app("nord", FixtureScenario::ChatIdle).expect("a seeded app")
}

/// Draws a frame so the tabs publish their regions, and hands back what it drew.
fn draw(app: &mut App) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(160, 40)).expect("a terminal");
    terminal.draw(|frame| crate::view::render(frame, app)).expect("a frame");
    terminal.backend().buffer().clone()
}

/// The region of tab `index` in `strip`, as the last frame published it.
fn tab_region(app: &App, strip: TabStrip, index: usize) -> Rect {
    let wanted = MouseTarget::Tab { strip, index };
    let hit = app.hit.borrow();
    let found = hit.clicks().find(|(_, target)| **target == wanted).map(|(rect, _)| rect);
    found.unwrap_or_else(|| panic!("no region for {wanted:?}"))
}

/// The text drawn across `rect`.
fn text_in(buffer: &Buffer, rect: Rect) -> String {
    (rect.x..rect.right()).map(|x| buffer[(x, rect.y)].symbol()).collect()
}

/// Presses and releases the left button on `rect`'s middle cell, the way the event loop does.
fn click(app: &mut App, rect: Rect) {
    let (column, row) = (rect.x + rect.width / 2, rect.y);
    for kind in [MouseEventKind::Down(MouseButton::Left), MouseEventKind::Up(MouseButton::Left)] {
        let event = MouseEvent { kind, column, row, modifiers: KeyModifiers::NONE };
        if let crate::main_loop::RouteAction::Msg(message) =
            crate::main_loop::route_mouse(app, &event)
        {
            app.update(*message);
        }
    }
}

#[test]
fn a_click_on_a_github_section_tab_opens_it() {
    let mut app = chat();
    seed_github(&mut app, "github");
    if let Some(panel) = app.overlays.github.as_mut() {
        panel.selected = 1;
    }
    let buffer = draw(&mut app);
    let region = tab_region(&app, TabStrip::GithubSections, 3);
    assert!(text_in(&buffer, region).contains("Pull requests"), "the region is over its label");

    click(&mut app, region);
    let panel = app.overlays.github.as_ref().expect("still open");
    assert_eq!((panel.section, panel.selected), (3, 0), "opened at the top, as Tab does");
}

#[test]
fn a_click_on_the_open_section_tab_keeps_the_place() {
    let mut app = chat();
    seed_github(&mut app, "github");
    if let Some(panel) = app.overlays.github.as_mut() {
        panel.selected = 1;
    }
    draw(&mut app);
    let region = tab_region(&app, TabStrip::GithubSections, 1);
    click(&mut app, region);
    let panel = app.overlays.github.as_ref().expect("still open");
    assert_eq!((panel.section, panel.selected), (1, 1));
}

#[test]
fn a_click_on_a_detail_tab_opens_it_and_moves_the_keys_there() {
    let mut app = chat();
    seed_github(&mut app, "github");
    let buffer = draw(&mut app);
    let region = tab_region(&app, TabStrip::GithubDetail, 1);
    assert!(text_in(&buffer, region).contains("Conversation"), "the region is over its label");

    click(&mut app, region);
    let panel = app.overlays.github.as_ref().expect("still open");
    assert_eq!(panel.detail().map(|detail| detail.tab), Some(DetailTab::Conversation));
    assert_eq!(panel.focus, crate::github_panel::Focus::Detail);
}

#[test]
fn a_click_on_the_pr_card_tab_opens_it() {
    let mut app = chat();
    seed_github(&mut app, "pr");
    draw(&mut app);
    let region = tab_region(&app, TabStrip::GithubDetail, 1);
    click(&mut app, region);
    let tab = match app.overlays.pull_request_card.as_ref().map(|card| &card.state) {
        Some(crate::github_detail::CardState::Ready(detail)) => detail.tab,
        other => panic!("the card is not ready: {other:?}"),
    };
    assert_eq!(tab, DetailTab::Conversation);
}

/// The card's detail, `Debug`-printed: its tab and any dialog open over it.
fn card_detail(app: &App) -> String {
    match app.overlays.pull_request_card.as_ref().map(|card| &card.state) {
        Some(crate::github_detail::CardState::Ready(detail)) => {
            format!("{:?} {:?}", detail.tab, detail.merging)
        }
        other => panic!("the card is not ready: {other:?}"),
    }
}

#[test]
fn a_detail_under_a_dialog_takes_no_tab_click() {
    let mut app = chat();
    seed_github(&mut app, "pr-merge");
    let before = card_detail(&app);
    // A dialog draws over the detail without taking its regions away, so the click still lands.
    // Replayed as Tab, it would reach the dialog and switch its merge method.
    draw(&mut app);
    let region = tab_region(&app, TabStrip::GithubDetail, 1);
    click(&mut app, region);
    assert_eq!(card_detail(&app), before, "the dialog and the tab under it are left alone");
}

#[test]
fn a_click_on_a_mention_tab_opens_it_and_lists_what_it_needs() {
    let mut app = chat();
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    app.engine_tx = tx;
    let provider = app.file_provider.clone();
    let root = std::path::PathBuf::from("/work");
    app.overlays.mention =
        Some(crate::mention_flyout::MentionFlyoutState::new(provider, root, &[], Vec::new(), &[]));
    let buffer = draw(&mut app);
    let strip = TabStrip::Picker(OverlayList::Mention);
    let region = tab_region(&app, strip, 2);
    assert!(text_in(&buffer, region).contains("GitHub"), "the region is over its label");

    click(&mut app, region);
    let mention = app.overlays.mention.as_ref().expect("still open");
    assert_eq!(*mention.tab(), MentionTab::GitHub);
    let asked = rx.try_recv().expect("the tab asked for its rows");
    assert!(matches!(asked, luxe_core::Action::ListGithub { .. }), "{asked:?}");
}
