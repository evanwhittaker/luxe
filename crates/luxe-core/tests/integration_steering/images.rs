//! A steer carries its images, as a submission does: into the running turn at its checkpoint, and
//! into the turn it starts when none is running.

use luxe_core::engine::ImageAttachment;
use luxe_core::message::Message;

use super::*;

fn photo() -> ImageAttachment {
    ImageAttachment {
        data: "aGk=".into(),
        mime: "image/jpeg".into(),
        name: Some("photo.jpg".into()),
    }
}

/// Whether `message` is the reader's, with `words` and the photo in it.
fn carries_words_and_photo(message: &Message, words: &str) -> bool {
    let said = message
        .content
        .iter()
        .any(|block| matches!(block, ContentBlock::Text(text) if text == words));
    let shown = message.content.iter().any(|block| {
        matches!(block, ContentBlock::Image { data, mime, .. } if data == "aGk=" && mime == "image/jpeg")
    });
    message.role == Role::User && said && shown
}

/// A turn whose gate opens on its own, so a steer sent while idle runs straight through.
fn self_releasing_gate() -> GateTool {
    let release = Arc::new(tokio::sync::Notify::new());
    let releaser = release.clone();
    tokio::spawn(async move {
        loop {
            releaser.notify_one();
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    });
    GateTool { entered: Arc::new(tokio::sync::Notify::new()), release }
}

/// An engine on a fresh session, whose provider keeps every request it was sent.
struct Running {
    action_tx: tokio::sync::mpsc::Sender<Action>,
    event_rx: tokio::sync::mpsc::Receiver<Event>,
    calls: Arc<Mutex<Vec<CompletionRequest>>>,
    _sessions: tempfile::TempDir,
}

fn spawn_with(gate: GateTool) -> Running {
    let sessions = tempfile::tempdir().unwrap();
    let mut store = JsonlSessionStore::new(sessions.path().join("sessions"));
    let session_id = store.create("/tmp/test").unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let provider = Arc::new(ScriptedProvider { calls: calls.clone() });
    let (action_tx, event_rx) = Engine::spawn(engine_config(store, session_id, provider, gate));
    Running { action_tx, event_rx, calls, _sessions: sessions }
}

/// Fails unless the provider's request `index` holds the reader's `words` with the photo.
async fn assert_request_carries(engine: &Running, index: usize, words: &str) {
    let calls = engine.calls.lock().await;
    let request = &calls[index].messages;
    assert!(request.iter().any(|message| carries_words_and_photo(message, words)), "{request:?}");
}

/// Every event up to the end of the turn.
async fn until_turn_ends(event_rx: &mut tokio::sync::mpsc::Receiver<Event>) -> Vec<Event> {
    let mut events = Vec::new();
    loop {
        match tokio::time::timeout(std::time::Duration::from_secs(5), event_rx.recv()).await {
            Ok(Some(Event::TurnEnded { .. })) => return events,
            Ok(Some(event)) => events.push(event),
            Ok(None) => panic!("event channel closed"),
            Err(_) => panic!("timed out waiting for the turn to end"),
        }
    }
}

/// A photo sent with a steer reaches the model in the same turn, and is announced with it.
///
/// The break this catches: the steer's photo dropped at the checkpoint, so the model reads "what's
/// wrong here?" with nothing to look at, or committed but left out of `MessageCommitted`, so the
/// phone that sent it never shows it in the transcript.
#[tokio::test]
async fn a_steer_carries_its_photo_into_the_running_turn() {
    let (entered, release) =
        (Arc::new(tokio::sync::Notify::new()), Arc::new(tokio::sync::Notify::new()));
    let mut engine = spawn_with(GateTool { entered: entered.clone(), release: release.clone() });
    let content = "start the work".to_string();
    let start = Action::SubmitUserMessage { content, images: vec![], remediation: None };
    engine.action_tx.send(start).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), entered.notified()).await.unwrap();

    let steer = Action::SteerTurn { text: "what's wrong here?".into(), images: vec![photo()] };
    engine.action_tx.send(steer).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    release.notify_one();
    let events = until_turn_ends(&mut engine.event_rx).await;

    assert_request_carries(&engine, 1, "what's wrong here?").await;
    let announced = events.iter().any(|event| {
        matches!(event, Event::MessageCommitted { message, .. } if carries_words_and_photo(message, "what's wrong here?"))
    });
    assert!(announced, "the committed steer is announced with its photo");
    let _ = engine.action_tx.send(Action::Close).await;
}

/// A steer that finds no turn running starts one with its photo, as a submission would.
///
/// The break this catches: the turn an idle steer starts sent as words alone, which drops a photo
/// sent in the instant between a turn ending and the phone hearing it had.
#[tokio::test]
async fn a_steer_with_no_turn_running_starts_one_with_its_photo() {
    let mut engine = spawn_with(self_releasing_gate());
    let steer = Action::SteerTurn { text: "look at this".into(), images: vec![photo()] };
    engine.action_tx.send(steer).await.unwrap();
    until_turn_ends(&mut engine.event_rx).await;

    assert_request_carries(&engine, 0, "look at this").await;
    let _ = engine.action_tx.send(Action::Close).await;
}
