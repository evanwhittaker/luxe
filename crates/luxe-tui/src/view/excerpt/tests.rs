use super::*;

fn styles() -> ThemeStyles {
    let resolved = luxe_core::theme::ThemeRegistry::new().resolve("nord").expect("nord");
    ThemeStyles::from_resolved(&resolved, "nord")
}

fn user(text: &str) -> ConvoBlock {
    ConvoBlock::User { text: text.into(), attached: Vec::new(), chips: Vec::new() }
}

fn text(line: &Line<'static>) -> String {
    line.spans.iter().map(|span| span.content.as_ref()).collect()
}

/// The break this catches: an excerpt that draws a user's message plain, unlike the transcript
/// that wraps it in its band, so the preview reads as a different conversation.
#[test]
fn a_user_message_wears_the_transcript_band() {
    let styles = styles();
    let lines = excerpt_lines(&[user("fix the build")], 60, &styles);
    let message = lines.iter().find(|line| text(line).contains("fix the build")).expect("drawn");
    assert_eq!(message.style.bg, styles.user_msg_bg.bg, "on the user band");
    assert_eq!(message.width(), 60 - 3, "the band runs the canvas, as the transcript's does");
}

/// The break this catches: an assistant's markdown shown as its raw source, the `**` and the
/// backticks left in, rather than styled as the transcript styles it.
#[test]
fn an_assistant_reply_is_rendered_as_markdown() {
    let lines = excerpt_lines(&[ConvoBlock::Assistant("It is **done**.".into())], 60, &styles());
    let all: String = lines.iter().map(text).collect();
    assert!(all.contains("It is done."), "{all}");
    assert!(!all.contains("**"));
}

/// The break this catches: messages run together, or a blank-only block leaving a hole.
#[test]
fn messages_are_set_apart_and_blank_ones_dropped() {
    let blocks =
        [user("one"), ConvoBlock::Assistant("\n\n".into()), ConvoBlock::Assistant("two".into())];
    let lines = excerpt_lines(&blocks, 60, &styles());
    let one = lines.iter().position(|line| text(line).contains("one")).expect("one");
    let two = lines.iter().position(|line| text(line).contains("two")).expect("two");
    // the band's bottom pad, then the gap between messages
    assert_eq!(two - one, 3, "{:?}", lines.iter().map(text).collect::<Vec<_>>());
}
