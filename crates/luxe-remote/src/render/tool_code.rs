//! Tool code — the command or file content a tool's card shows, highlighted as the TUI's card is.
//!
//! The app asks for the lines it shows; the language is decided here, the way the TUI's card
//! decides it, so the app needs no idea of what a file extension means.

use super::{escape, Renderer};
use std::path::Path;

/// `text` as HTML for `tool`'s card: highlighted for the language the card's code is in, or only
/// escaped when it has none.
pub(super) fn html(renderer: &dyn Renderer, tool: &str, label: &str, text: &str) -> String {
    match language(tool, label) {
        Some(language) => renderer.code_block(text, Some(&language)),
        None => escape(text),
    }
}

/// What a card's code is written in: a command is shell, a written file is in its extension's
/// language (or its name's, for a `Makefile`), and anything else is prose, such as a list of
/// searches.
fn language(tool: &str, label: &str) -> Option<String> {
    match tool {
        "bash" => Some("sh".to_owned()),
        "write" => file_language(label),
        _ => None,
    }
}

fn file_language(path: &str) -> Option<String> {
    let path = Path::new(path.trim());
    let token = path.extension().or_else(|| path.file_name())?;
    Some(token.to_string_lossy().to_lowercase()).filter(|token| !token.is_empty())
}

#[cfg(test)]
mod tests {
    use super::html;
    use crate::render::{escape, Renderer, StatusFacts, StatusSegment, Voice};

    /// Code that shows the language it was highlighted for, escaped.
    struct Bracketed;

    impl Renderer for Bracketed {
        fn markdown(&self, text: &str, _voice: Voice) -> String {
            escape(text)
        }
        fn tool_glyph(&self, _tool: &str) -> String {
            String::new()
        }
        fn status(&self, _facts: &StatusFacts) -> Vec<StatusSegment> {
            Vec::new()
        }
        fn code_block(&self, text: &str, language: Option<&str>) -> String {
            format!("[{}:{}]", language.unwrap_or("-"), escape(text))
        }
    }

    /// The break this catches: a command not highlighted as shell, or highlighted line by line
    /// rather than as one block, so a heredoc loses its color after its first line.
    #[test]
    fn a_command_is_highlighted_whole_as_shell() {
        let shown = html(&Bracketed, "bash", "Run the tests", "cat <<EOF\n<b>\nEOF");
        assert_eq!(shown, "[sh:cat &lt;&lt;EOF\n&lt;b&gt;\nEOF]");
    }

    /// The break this catches: a written file highlighted as something other than its type, its
    /// extension read case by case, or a `Makefile` with no extension left plain.
    #[test]
    fn a_written_file_is_highlighted_by_its_type() {
        assert_eq!(html(&Bracketed, "write", "src/Main.RS", "fn"), "[rs:fn]");
        assert_eq!(html(&Bracketed, "write", "Makefile", "all:"), "[makefile:all:]");
    }

    /// The break this catches: a search's queries, which are prose, highlighted as some language,
    /// or code left unescaped when nothing highlights it.
    #[test]
    fn other_tools_show_their_code_plain() {
        assert_eq!(html(&Bracketed, "web_search", "2 queries", "<one>\ntwo"), "&lt;one&gt;\ntwo");
    }
}
