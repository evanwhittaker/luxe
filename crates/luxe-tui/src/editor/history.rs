//! History — recalling sent prompts without losing the one being written.
//!
//! Follows a shell's line editor: ↑ from the reader's own draft sets it aside and ↓ past the
//! newest message brings it back. A recalled prompt the reader has edited is theirs now, so the
//! arrows stop swapping it out. Text that leaves the box unsent (Esc, or a set-aside draft the
//! reader never came back for) goes into history, so ↑ always finds it again.

use super::parts::PromptPart;
use super::{Editor, HISTORY_LIMIT};

/// The reader's own draft, held while ↑ shows older prompts.
#[derive(Debug, Clone, Default)]
pub(super) struct SetAside {
    buffer: String,
    parts: Vec<PromptPart>,
    /// The draft as it would be sent, kept for history if it is never restored
    expanded: String,
}

impl Editor {
    /// Adds a prompt to history, skipping blanks and a repeat of the newest entry.
    pub(super) fn remember(&mut self, text: String) {
        if text.trim().is_empty() || self.history.last() == Some(&text) {
            return;
        }
        self.history.push(text);
        if self.history.len() > HISTORY_LIMIT {
            self.history.remove(0);
        }
    }

    /// Clears the box for Esc, keeping unsent text in history so ↑ brings it back.
    pub(super) fn discard_draft(&mut self) {
        if !self.showing_untouched_recall() {
            let text = self.expanded_input();
            self.remember(text);
        }
        self.clear_draft();
    }

    /// Ends a browse: a draft still set aside is kept in history rather than dropped.
    pub(super) fn end_browse(&mut self) {
        self.history_index = None;
        if let Some(set_aside) = self.set_aside.take() {
            self.remember(set_aside.expanded);
        }
    }

    /// Whether the box shows a recalled prompt exactly as it was sent.
    fn showing_untouched_recall(&self) -> bool {
        self.history_index.is_some_and(|back| self.recalled(back) == Some(&self.buffer))
    }

    /// The message `back` steps behind the newest.
    fn recalled(&self, back: usize) -> Option<&String> {
        self.history.len().checked_sub(back + 1).and_then(|index| self.history.get(index))
    }

    /// Whether ↑/↓ may swap what the box shows: always from the reader's own draft, which is set
    /// aside, and from a recalled prompt only while it is untouched.
    fn may_browse(&self) -> bool {
        self.history_index.is_none() || self.showing_untouched_recall()
    }

    pub(super) fn history_previous(&mut self) {
        let back = self.history_index.map_or(0, |back| back + 1);
        if back >= self.history.len() || !self.may_browse() {
            return;
        }
        if self.history_index.is_none() {
            self.set_draft_aside();
        }
        self.show_recalled(back, false);
    }

    pub(super) fn history_next(&mut self) {
        let Some(back) = self.history_index else { return };
        if !self.may_browse() {
            return;
        }
        if back > 0 {
            self.show_recalled(back - 1, true);
            return;
        }
        self.history_index = None;
        let draft = self.set_aside.take().unwrap_or_default();
        self.buffer = draft.buffer;
        self.parts = draft.parts;
        self.cursor = self.buffer.len();
        self.preferred_column = None;
    }

    fn set_draft_aside(&mut self) {
        let expanded = self.expanded_input();
        let buffer = std::mem::take(&mut self.buffer);
        let parts = std::mem::take(&mut self.parts);
        self.set_aside = Some(SetAside { buffer, parts, expanded });
    }

    fn show_recalled(&mut self, back: usize, caret_at_end: bool) {
        let Some(text) = self.recalled(back).cloned() else { return };
        self.history_index = Some(back);
        self.buffer = text;
        self.parts.clear();
        self.cursor = if caret_at_end { self.buffer.len() } else { 0 };
        self.preferred_column = None;
    }
}
