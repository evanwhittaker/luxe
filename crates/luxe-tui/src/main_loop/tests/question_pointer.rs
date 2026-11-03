//! The question prompt under the pointer: an answer being typed survives the pointer leaving it.

use super::*;
use crate::app::App;
use crate::mouse::MouseTarget;
use crate::question_prompt::QuestionPromptState;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use luxe_core::question::{QuestionOption, QuestionReply, QuestionRequestId};
use luxe_core::Action;
use ratatui::{backend::TestBackend, Terminal};
use tokio::sync::mpsc;

const WIDTH: u16 = 120;
const HEIGHT: u16 = 40;

/// A chat asking "Which?" of `one` and `two`, whose answers can be read.
fn asking() -> (App, mpsc::Receiver<Action>) {
    let (tx, rx) = mpsc::channel(16);
    let keymap = luxe_core::keymap::KeyMap::defaults();
    let mut app = App::new(tx, keymap, luxe_core::config::Config::default());
    app.dashboard = None;
    app.current_view = crate::app::ViewMode::Chat;
    app.update(Msg::TerminalResized { width: WIDTH, height: HEIGHT });
    let option =
        |label: &str| QuestionOption { label: label.into(), description: None, recommended: false };
    let options = vec![option("one"), option("two")];
    let prompt =
        QuestionPromptState::new(QuestionRequestId::new(), "Which?".into(), options, false);
    app.overlays.question = Some(prompt);
    (app, rx)
}

/// A cell inside question row `index`, as the frame just drawn laid it out.
fn row_cell(app: &mut App, index: usize) -> (u16, u16) {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("a terminal");
    terminal.draw(|frame| crate::view::render(frame, app)).expect("a frame");
    let hit = app.hit.borrow();
    let row = hit.clicks().find_map(|(rect, target)| {
        (*target == MouseTarget::QuestionRow(index)).then_some((rect.x + 2, rect.y))
    });
    row.expect("the row is drawn")
}

fn pointer(app: &mut App, kind: MouseEventKind, (column, row): (u16, u16)) {
    let event = MouseEvent { kind, column, row, modifiers: KeyModifiers::NONE };
    let action = route_mouse(app, &event);
    apply_route_action(app, action, &mut TuiExit::Quit);
}

fn click(app: &mut App, at: (u16, u16)) {
    pointer(app, MouseEventKind::Down(MouseButton::Left), at);
    pointer(app, MouseEventKind::Up(MouseButton::Left), at);
}

fn press(app: &mut App, code: KeyCode) {
    let key = KeyEvent::new(code, KeyModifiers::NONE);
    apply_terminal_event(app, crossterm::event::Event::Key(key), &mut TuiExit::Quit);
}

fn written(app: &App) -> String {
    app.overlays.question.as_ref().expect("still asking").custom().to_string()
}

fn answered(rx: &mut mpsc::Receiver<Action>) -> Vec<String> {
    match rx.try_recv() {
        Ok(Action::AnswerQuestion { reply: QuestionReply::Answered(answers), .. }) => answers,
        other => panic!("no answer went out: {other:?}"),
    }
}

/// The break this catches: the pointer passing over an option on its way out taking the highlight
/// from the answer being typed, so the next key answers that option instead of going into it.
#[test]
fn the_pointer_passing_by_leaves_the_answer_being_typed() {
    let (mut app, mut rx) = asking();
    press(&mut app, KeyCode::Char('x'));
    let option = row_cell(&mut app, 0);
    pointer(&mut app, MouseEventKind::Moved, option);
    press(&mut app, KeyCode::Enter);
    assert_eq!(answered(&mut rx), ["x"]);
}

/// The break this catches: a click on the answer's row, to carry on writing it, sending what was
/// there so far; or a click on an option no longer answering with it.
#[test]
fn a_click_on_the_written_answer_puts_the_caret_back_in_it() {
    let (mut app, mut rx) = asking();
    press(&mut app, KeyCode::Char('x'));
    press(&mut app, KeyCode::Esc);
    let answer = row_cell(&mut app, 2);
    click(&mut app, answer);
    press(&mut app, KeyCode::Char('2'));
    assert_eq!(written(&app), "x2", "the click sent nothing and the digit was typed");

    let option = row_cell(&mut app, 1);
    click(&mut app, option);
    assert_eq!(answered(&mut rx), ["two"], "a click on an option answers with it");
}
