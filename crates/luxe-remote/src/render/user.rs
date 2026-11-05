//! User messages — the reader's own words, as the transcript shows them.
//!
//! The notes that ride along for the model are left out, a brief shows as its one line, and a
//! question asked in a review shows under where it was asked. Which is which is core's call
//! (`luxe_core::briefs`).

use luxe_core::briefs;

use super::{escape, Renderer, WireBlock};

/// A user message as the transcript shows it: the reader's words as they wrote them, without the
/// notes that ride along for the model, and each brief as one line.
pub(super) fn user_message(content: &[WireBlock]) -> String {
    let texts = content.iter().map(|block| match block {
        WireBlock::Text(text) => Some(text.as_str()),
        WireBlock::Image | WireBlock::Other => None,
    });
    briefs::readers_texts(texts).iter().map(|(_, text)| user_block(text)).collect()
}

/// The line the TUI puts under a message that carried images.
pub(super) fn images_attached(renderer: &dyn Renderer, content: &[WireBlock]) -> String {
    let count = content.iter().filter(|block| matches!(block, WireBlock::Image)).count();
    if count == 0 {
        return String::new();
    }
    let glyph = renderer.image_glyph();
    let glyph = if glyph.is_empty() { glyph } else { format!("{} ", escape(&glyph)) };
    format!("<p class=\"attached\">{glyph}{count} image(s) attached</p>")
}

/// One block of a user message. A question from a review is the reader's own words, so it shows
/// what they asked under where they asked it.
fn user_block(text: &str) -> String {
    let asked = briefs::review_questions(text);
    if !asked.is_empty() {
        return asked.iter().map(asked_in_review).collect();
    }
    match briefs::label(text) {
        Some(label) => format!("<p class=\"brief\">{}</p>", escape(label)),
        None => format!("<p class=\"words\">{}</p>", escape(text)),
    }
}

fn asked_in_review(asked: &briefs::ReviewQuestion) -> String {
    format!(
        "<p class=\"words\"><span class=\"chip\">Asked in review \u{b7} {}</span>\n{}</p>",
        escape(&asked.location),
        escape(&asked.question)
    )
}
