//! Transcript excerpt — a few conversation blocks drawn the way the transcript draws them.
//!
//! The session picker's preview and Find's conversation results show messages in a pane. Drawn
//! through the transcript's own block renderer, wrapping and bands, a message there looks as it
//! does in the conversation: a user's message on its band, a reply's markdown styled, a tool call
//! as its card. What only a live transcript has (images, a spinner, a selection) is left out.

use super::{blocks, conversation_canvas_widths, transcript, wrapping};
use crate::app::ConvoBlock;
use crate::theme::styles::ThemeStyles;
use ratatui::text::Line;

/// `blocks` drawn for a pane `width` cells wide, a blank line between messages.
pub(crate) fn excerpt_lines(
    blocks: &[ConvoBlock],
    width: u16,
    styles: &ThemeStyles,
) -> Vec<Line<'static>> {
    let canvas = conversation_canvas_widths(width);
    let images = crate::graphics::ImageStore::default();
    let tool_images = std::collections::HashMap::new();
    let context = block_context(styles, canvas, &images, &tool_images);
    let mut out = Vec::new();
    for block in blocks {
        let rows = block_rows(block, &context, canvas);
        if rows.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(Line::default());
        }
        out.extend(rows);
    }
    out
}

/// What the block renderer needs, for blocks that aren't live: nothing selected, running,
/// expanded or drawn as a picture.
fn block_context<'a>(
    styles: &'a ThemeStyles,
    canvas: super::ConversationCanvasWidths,
    images: &'a crate::graphics::ImageStore,
    tool_images: &'a ToolImages,
) -> blocks::BlockContext<'a> {
    blocks::BlockContext {
        styles,
        spinner_frame: 0,
        waiting_on_user: false,
        never_sent: false,
        expanded: false,
        images,
        tool_images,
        image_bounds: (0, 0),
        card_width: canvas.wide_width,
        prose_width: canvas.prose_width,
        diff_layout: luxe_core::config::DiffLayout::Auto,
        tool_output_expanded: false,
        expand_chord: None,
        running_ms: None,
    }
}

type ToolImages = std::collections::HashMap<(String, String), Vec<crate::graphics::ImageKey>>;

/// One block wrapped to the canvas and banded as the transcript bands it; nothing for a block
/// with no text to show.
fn block_rows(
    block: &ConvoBlock,
    context: &blocks::BlockContext<'_>,
    canvas: super::ConversationCanvasWidths,
) -> Vec<Line<'static>> {
    let content: Vec<Line<'static>> = blocks::lines(block, context)
        .into_iter()
        .flat_map(|layout| wrapping::conversation_line(layout, canvas))
        .collect();
    if content.iter().all(is_blank) {
        return Vec::new();
    }
    let band = blocks::background(block, context.styles);
    let selected = context.styles.selected_bg;
    transcript::render_banded(content, band, false, selected, canvas.available_width)
}

fn is_blank(line: &Line<'static>) -> bool {
    line.spans.iter().all(|span| span.content.trim().is_empty())
}

#[cfg(test)]
mod tests;
