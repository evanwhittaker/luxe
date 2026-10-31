//! Provider adapters must stream, not buffer.
//!
//! Every adapter used to call `response.text().await` before parsing, so reasoning and answer text
//! reached the UI only after the provider had finished the whole call. These tests keep an SSE body
//! deliberately open and assert the first events arrive while it is still open — the one property
//! whole-body parsing cannot satisfy.

use futures::StreamExt;
use luxe_core::message::Message;
use luxe_core::provider::{
    CompletionRequest, ModelInfo, ModelRef, Provider, ProviderEvent, ThinkingLevel,
};
use luxe_providers::anthropic::{AnthropicCompat, AnthropicProvider};
use luxe_providers::config::{CompatFlags, ProviderConfig};
use luxe_providers::openai_compat::OpenaiCompatProvider;
use std::io::Write;
use std::time::Duration;

/// How long the test waits for what must arrive while the body is still open.
///
/// The server holds the second half until the test releases it, so an adapter that waits for the
/// whole body never sees an event in time, however fast the machine. This is a deadline for that
/// hang, not a measure of speed: a wall-clock budget of a few hundred milliseconds failed on a
/// machine busy with the rest of the workspace's tests, where building the HTTP client alone can
/// take that long. It covers `Provider::stream` as well as the first `next()`, since whole-body
/// parsing blocks *there*, before the stream is even returned.
const OPEN_BODY_WAIT: Duration = Duration::from_secs(10);

/// How long the server holds the body open when nothing releases it: well past `OPEN_BODY_WAIT`,
/// so an adapter that buffered is still waiting when the test gives up on it.
const HOLD_LIMIT: Duration = Duration::from_secs(30);

/// Serves one HTTP response whose SSE body arrives in two halves: the first at once, the second
/// when the test sends on the returned channel. Keep the sender until then: dropping it releases
/// the second half too.
///
/// No `Content-Length`: HTTP/1.1 then frames the body by connection close, which is exactly how
/// the real providers stream, and lets us write the halves whenever we like.
fn serve_split_sse(first: String, second: String) -> (u16, std::sync::mpsc::Sender<()>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (release, released) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().expect("accept");
        read_request_head(&mut socket);
        let head =
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n";
        socket.write_all(head.as_bytes()).expect("head");
        socket.write_all(first.as_bytes()).expect("first half");
        socket.flush().expect("flush");
        let _ = released.recv_timeout(HOLD_LIMIT);
        // The client may have hung up already, after a cancel
        let _ = socket.write_all(second.as_bytes());
        let _ = socket.flush();
    });
    (port, release)
}

/// Reads request headers (and any body) off `socket` far enough to reply.
fn read_request_head(socket: &mut std::net::TcpStream) {
    use std::io::Read;

    let mut buffer = Vec::new();
    let mut chunk = [0u8; 2048];
    loop {
        let read = socket.read(&mut chunk).unwrap_or(0);
        if read == 0 {
            return;
        }
        buffer.extend_from_slice(&chunk[..read]);
        let text = String::from_utf8_lossy(&buffer);
        if let Some(head_end) = text.find("\r\n\r\n") {
            let length = text
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|value| value.trim().parse::<usize>().unwrap_or(0))
                })
                .unwrap_or(0);
            if buffer.len() >= head_end + 4 + length {
                return;
            }
        }
    }
}

fn model(provider: &str) -> ModelInfo {
    ModelInfo {
        r#ref: ModelRef::new(provider, "test-model"),
        max_tokens: 1024,
        context_window: 8192,
        ..Default::default()
    }
}

fn request(model: ModelInfo) -> CompletionRequest {
    CompletionRequest {
        model,
        messages: vec![Message::user("hello")],
        system: None,
        tools: vec![],
        thinking: ThinkingLevel::Off,
        max_tokens: Some(256),
        ..Default::default()
    }
}

/// Anthropic: a thinking delta in the first half must be observable before the second half exists.
#[tokio::test]
async fn anthropic_thinking_deltas_arrive_while_the_body_is_still_open() {
    let first = "event: content_block_delta\n\
         data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"pondering\"}}\n\n"
        .to_string();
    let second = "event: content_block_delta\n\
         data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"text_delta\",\"text\":\"answer\"}}\n\n\
         event: message_stop\n\
         data: {\"type\":\"message_stop\"}\n\n"
        .to_string();
    let (port, release) = serve_split_sse(first, second);

    let model = model("anthropic");
    let provider = AnthropicProvider::new(
        "test-key".into(),
        Some(format!("http://127.0.0.1:{port}/v1")),
        model.clone(),
        AnthropicCompat::default(),
    );
    let (_keep, cancel) = tokio::sync::watch::channel(false);
    let (stream, event) = tokio::time::timeout(OPEN_BODY_WAIT, async {
        let mut stream = provider.stream(request(model), cancel).await.expect("stream opens");
        let event = stream.next().await;
        (stream, event)
    })
    .await
    .expect("the first reasoning delta must not wait for the whole response");
    assert!(
        matches!(event, Some(ProviderEvent::ThinkingDelta { ref delta, .. }) if delta == "pondering"),
        "expected a streamed thinking delta, got {event:?}"
    );

    // The rest still arrives once the server sends it, terminator included.
    release.send(()).expect("the server is holding the body open");
    let rest: Vec<ProviderEvent> = stream.collect().await;
    assert!(
        rest.iter().any(
            |event| matches!(event, ProviderEvent::TextDelta { delta, .. } if delta == "answer")
        ),
        "{rest:?}"
    );
    assert!(matches!(rest.last(), Some(ProviderEvent::Done { .. })), "{rest:?}");
}

/// OpenAI-compatible: same property, plus the reasoning field these servers actually use.
#[tokio::test]
async fn openai_compat_deltas_arrive_while_the_body_is_still_open() {
    let first = concat!(
        r#"data: {"choices":[{"index":0,"delta":{"reasoning_content":"pondering"}}]}"#,
        "\n\n",
        r#"data: {"choices":[{"index":0,"delta":{"content":"ans"}}]}"#,
        "\n\n"
    )
    .to_string();
    let second = concat!(
        r#"data: {"choices":[{"index":0,"finish_reason":"stop"}],"usage":{"prompt_tokens":9,"completion_tokens":2,"total_tokens":11}}"#,
        "\n\n",
        "data: [DONE]\n\n"
    )
    .to_string();
    let (port, release) = serve_split_sse(first, second);

    let provider = OpenaiCompatProvider::new(
        ProviderConfig {
            api_key: Some("test-key".into()),
            base_url: Some(format!("http://127.0.0.1:{port}/v1")),
            ..Default::default()
        },
        CompatFlags::default(),
    );
    let (_keep, cancel) = tokio::sync::watch::channel(false);
    let (stream, event) = tokio::time::timeout(OPEN_BODY_WAIT, async {
        let mut stream =
            provider.stream(request(model("openai_compat")), cancel).await.expect("stream opens");
        let event = stream.next().await;
        (stream, event)
    })
    .await
    .expect("the first reasoning delta must not wait for the whole response");
    assert!(
        matches!(event, Some(ProviderEvent::ThinkingDelta { ref delta, .. }) if delta == "pondering"),
        "expected a streamed reasoning delta, got {event:?}"
    );

    release.send(()).expect("the server is holding the body open");
    let rest: Vec<ProviderEvent> = stream.collect().await;
    let usage_before_done = rest
        .iter()
        .position(|event| matches!(event, ProviderEvent::Usage(_)))
        .zip(rest.iter().position(|event| matches!(event, ProviderEvent::Done { .. })))
        .is_some_and(|(usage, done)| usage < done);
    assert!(usage_before_done, "usage must still land before Done: {rest:?}");
}

/// A cancelled turn drops the connection instead of waiting out a provider that has gone quiet.
#[tokio::test]
async fn cancelling_ends_the_stream_without_waiting_for_the_body() {
    let first = "event: content_block_delta\n\
         data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"partial\"}}\n\n"
        .to_string();
    let (port, release) = serve_split_sse(first, String::new());

    let model = model("anthropic");
    let provider = AnthropicProvider::new(
        "test-key".into(),
        Some(format!("http://127.0.0.1:{port}/v1")),
        model.clone(),
        AnthropicCompat::default(),
    );
    let (cancel_tx, cancel) = tokio::sync::watch::channel(false);
    let (mut stream, first_event) = tokio::time::timeout(OPEN_BODY_WAIT, async {
        let mut stream = provider.stream(request(model), cancel).await.expect("stream opens");
        let event = stream.next().await;
        (stream, event)
    })
    .await
    .expect("the first delta must not wait for the whole response");
    assert!(matches!(first_event, Some(ProviderEvent::TextDelta { .. })), "{first_event:?}");

    cancel_tx.send(true).expect("cancel");
    let ended =
        tokio::time::timeout(OPEN_BODY_WAIT, async { while stream.next().await.is_some() {} })
            .await;
    assert!(ended.is_ok(), "a cancelled stream must end without waiting out the body");
    // Held to here: released any earlier, the body would end by itself and pass without the cancel
    drop(release);
}
