//! find — the `ctrl+f` overlay: one query, looked for in the project's text, its file names, or
//! the conversation.
//!
//! The state machine is pure, like the palette's: no rendering and no engine calls. A key returns
//! a [`FindOutcome`] and the app does the rest. It sends the search a changed query needs
//! ([`FindState::next_search`]), ranks file names with the `@` picker's provider, opens a hit in
//! the preview, or goes to a message in the conversation. The text search runs in the
//! engine, so an answer can land after the reader has typed on, and one that isn't for the search
//! in flight is dropped.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use luxe_core::project_search::{
    SearchAnswer, SearchFailed, SearchOptions, SearchRequestId, SearchResults,
};

use crate::ref_provider::Candidate;

pub(crate) mod messages;
mod preview;
mod previews;
mod render;
pub(crate) use previews::{PreviewCache, PreviewText};
pub(crate) use render::render_find;

/// Where a query looks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum FindTab {
    /// Every line of the project's files
    #[default]
    Text,
    /// The project's file names
    Files,
    /// What was said in the conversation
    Conversation,
}

impl FindTab {
    /// Every tab, in the order Tab walks them and the header draws them.
    pub(crate) const ALL: [FindTab; 3] = [FindTab::Text, FindTab::Files, FindTab::Conversation];

    /// The tab's name in the header.
    pub(crate) fn label(self) -> &'static str {
        match self {
            FindTab::Text => "Text",
            FindTab::Files => "Files",
            FindTab::Conversation => "Conversation",
        }
    }

    fn step(self, forward: bool) -> Self {
        let len = Self::ALL.len();
        let index = Self::ALL.iter().position(|tab| *tab == self).unwrap_or(0);
        let next = if forward { index + 1 } else { index + len - 1 };
        Self::ALL[next % len]
    }
}

/// The file a row points at, and the line in it when the row is a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FindTarget {
    pub(crate) path: String,
    pub(crate) line: Option<u32>,
}

impl FindTarget {
    /// The mention the composer takes for it, in the form the `@` picker and the preview's Ask
    /// already write.
    pub(crate) fn mention(&self) -> String {
        match self.line {
            Some(line) => format!("@[file:{}:{line}]", self.path),
            None => format!("@[file:{}]", self.path),
        }
    }
}

/// What a key asks the app to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FindOutcome {
    /// Nothing beyond what the state already did
    Consumed,
    /// Close the overlay
    Close,
    /// The query, an option or the tab changed, so the tab's rows are stale until refreshed
    Refresh,
    /// Open a file in the preview.
    ///
    /// Args: the row's file and line.
    Open(FindTarget),
    /// Mention a file in the composer.
    ///
    /// Args: the row's file and line.
    Mention(FindTarget),
    /// Go to a message in the conversation, its search open on the query.
    ShowMessage {
        /// The message
        item: crate::app::ConversationItemId,
        /// What to search for there
        query: String,
    },
}

/// The Find overlay, while it's open.
#[derive(Debug, Clone)]
pub(crate) struct FindState {
    tab: FindTab,
    query: String,
    options: SearchOptions,
    /// Cursor position in the tab's rows
    selected: usize,
    /// The text search in flight, so an answer for any other is dropped
    request: Option<SearchRequestId>,
    /// What the last text search asked, so coming back to the Text tab doesn't ask again
    searched: Option<(String, SearchOptions)>,
    /// The last text search's rows, kept while the next one runs so the list doesn't blink
    results: Option<SearchResults>,
    /// The options `results` were searched with, which the field may since have changed
    results_options: SearchOptions,
    /// Why the last text search has no rows
    error: Option<String>,
    /// The Files tab's rows, ranked by the app
    files: Vec<Candidate>,
    /// The Conversation tab's rows, found by the app
    messages: Vec<messages::MessageHit>,
    /// The files the preview pane has read, and the reads still out
    previews: PreviewCache,
}

impl FindState {
    /// Opens on `tab` with an empty query.
    pub(crate) fn open(tab: FindTab) -> Self {
        Self {
            tab,
            query: String::new(),
            options: SearchOptions::default(),
            selected: 0,
            request: None,
            searched: None,
            results: None,
            results_options: SearchOptions::default(),
            error: None,
            files: Vec::new(),
            messages: Vec::new(),
            previews: PreviewCache::default(),
        }
    }

    /// Answers a key.
    pub(crate) fn on_key(&mut self, key: &KeyEvent) -> FindOutcome {
        if key.modifiers.contains(KeyModifiers::ALT) {
            return self.on_alt(key.code);
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => FindOutcome::Close,
            KeyCode::Enter => self.on_enter(),
            KeyCode::Tab => self.switch(true),
            KeyCode::BackTab => self.switch(false),
            KeyCode::Up => self.step(false),
            KeyCode::Down => self.step(true),
            KeyCode::Char('p') if ctrl => self.step(false),
            KeyCode::Char('n') if ctrl => self.step(true),
            KeyCode::Backspace => self.edit_query(drop_last_char),
            KeyCode::Char(c) if !ctrl => self.edit_query(|query| query.push(c)),
            _ => FindOutcome::Consumed,
        }
    }

    /// Types pasted text into the query, on one line.
    pub(crate) fn paste(&mut self, text: &str) -> FindOutcome {
        let text = crate::editor::single_line_paste(text);
        self.edit_query(|query| query.push_str(&text))
    }

    /// The text search the query now needs, if any.
    ///
    /// An emptied query clears the rows instead, since there is nothing to ask and the old rows
    /// answer text no longer in the field.
    pub(crate) fn next_search(&mut self) -> Option<(String, SearchOptions)> {
        if self.tab != FindTab::Text {
            return None;
        }
        if self.query.is_empty() {
            self.clear_text();
            return None;
        }
        let ask = (self.query.clone(), self.options);
        (self.searched.as_ref() != Some(&ask)).then_some(ask)
    }

    /// Records the search the app sent for `asked`, which only its own answer can now fill.
    pub(crate) fn searching(&mut self, request: SearchRequestId, asked: (String, SearchOptions)) {
        self.request = Some(request);
        self.searched = Some(asked);
    }

    /// Takes a search's answer, unless a newer search has replaced it.
    pub(crate) fn accept(&mut self, answer: &SearchAnswer) -> bool {
        if self.request.as_ref() != Some(&answer.request_id) {
            return false;
        }
        self.request = None;
        self.results = Some(answer.results.clone());
        self.results_options = self.searched.as_ref().map_or(self.options, |(_, asked)| *asked);
        self.error = None;
        self.selected = 0;
        true
    }

    /// Takes a search's failure under the same guard.
    pub(crate) fn fail(&mut self, failed: &SearchFailed) -> bool {
        if self.request.as_ref() != Some(&failed.request_id) {
            return false;
        }
        self.request = None;
        self.results = None;
        self.error = Some(failed.error.to_string());
        true
    }

    /// Records that the search couldn't be sent at all, and why.
    pub(crate) fn unsent(&mut self, reason: &str) {
        self.request = None;
        self.searched = None;
        self.results = None;
        self.error = Some(reason.to_string());
    }

    /// Takes the Files tab's ranked rows.
    pub(crate) fn set_files(&mut self, files: Vec<Candidate>) {
        self.files = files;
        self.selected = 0;
    }

    /// Takes the Conversation tab's rows.
    pub(crate) fn set_messages(&mut self, messages: Vec<messages::MessageHit>) {
        self.messages = messages;
        self.selected = 0;
    }

    /// The row under the cursor, as the file and line it points at.
    pub(crate) fn target(&self) -> Option<FindTarget> {
        match self.tab {
            FindTab::Text => self.hits().get(self.selected).map(|hit| FindTarget {
                path: hit.path.clone(),
                line: u32::try_from(hit.line).ok(),
            }),
            FindTab::Files => self
                .files
                .get(self.selected)
                .map(|file| FindTarget { path: file.insert.clone(), line: None }),
            FindTab::Conversation => None,
        }
    }

    /// Shows the tab at `index` of [`FindTab::ALL`], as a click on it does.
    pub(crate) fn show_tab(&mut self, index: usize) -> FindOutcome {
        let Some(tab) = FindTab::ALL.get(index).copied() else { return FindOutcome::Consumed };
        if tab == self.tab {
            return FindOutcome::Consumed;
        }
        self.tab = tab;
        self.selected = 0;
        FindOutcome::Refresh
    }

    /// Moves the cursor to row `row`, as a click does; clamped to the rows there are.
    pub(crate) fn select(&mut self, row: usize) {
        self.selected = row.min(self.row_count().saturating_sub(1));
    }

    /// How many rows the tab has.
    pub(crate) fn row_count(&self) -> usize {
        match self.tab {
            FindTab::Text => self.hits().len(),
            FindTab::Files => self.files.len(),
            FindTab::Conversation => self.messages.len(),
        }
    }

    pub(crate) fn tab(&self) -> FindTab {
        self.tab
    }

    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    pub(crate) fn options(&self) -> SearchOptions {
        self.options
    }

    pub(crate) fn selected(&self) -> usize {
        self.selected
    }

    /// The last text search's answer, which may be for an older query while a new one runs.
    pub(crate) fn results(&self) -> Option<&SearchResults> {
        self.results.as_ref()
    }

    /// The files the preview pane has read.
    pub(crate) fn previews(&self) -> &PreviewCache {
        &self.previews
    }

    /// The files the preview pane has read, for recording a read and its answer.
    pub(crate) fn previews_mut(&mut self) -> &mut PreviewCache {
        &mut self.previews
    }

    /// The file the row under the cursor shows, when the pane has yet to ask for it.
    pub(crate) fn preview_wanted(&self) -> Option<String> {
        let path = self.target()?.path;
        self.previews.wants(&path).then_some(path)
    }

    /// What the Text tab's rows were searched for, so whatever shows them lights what they
    /// matched rather than what the field says now.
    pub(crate) fn rows_query(&self) -> Option<(&str, SearchOptions)> {
        if self.tab != FindTab::Text {
            return None;
        }
        self.results.as_ref().map(|results| (results.query.as_str(), self.results_options))
    }

    /// Why the last text search has no rows.
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Whether a text search is out.
    pub(crate) fn is_searching(&self) -> bool {
        self.request.is_some()
    }

    pub(crate) fn files(&self) -> &[Candidate] {
        &self.files
    }

    pub(crate) fn messages(&self) -> &[messages::MessageHit] {
        &self.messages
    }

    fn hits(&self) -> &[luxe_core::project_search::SearchHit] {
        self.results.as_ref().map_or(&[], |results| &results.hits)
    }

    fn on_alt(&mut self, code: KeyCode) -> FindOutcome {
        match code {
            KeyCode::Char('m') => self.target().map_or(FindOutcome::Consumed, FindOutcome::Mention),
            KeyCode::Char('r') => self.set_options(|options| options.regex = !options.regex),
            KeyCode::Char('c') => self.set_options(|options| {
                options.case_sensitive = next_case(options.case_sensitive);
            }),
            KeyCode::Char('i') => {
                self.set_options(|options| options.include_ignored = !options.include_ignored)
            }
            _ => FindOutcome::Consumed,
        }
    }

    fn on_enter(&self) -> FindOutcome {
        match self.tab {
            FindTab::Conversation => {
                self.messages.get(self.selected).map_or(FindOutcome::Consumed, |hit| {
                    FindOutcome::ShowMessage { item: hit.item.clone(), query: self.query.clone() }
                })
            }
            _ => self.target().map_or(FindOutcome::Consumed, FindOutcome::Open),
        }
    }

    fn switch(&mut self, forward: bool) -> FindOutcome {
        self.tab = self.tab.step(forward);
        self.selected = 0;
        FindOutcome::Refresh
    }

    fn step(&mut self, forward: bool) -> FindOutcome {
        let len = self.row_count();
        if len > 0 {
            let next = if forward { self.selected + 1 } else { self.selected + len - 1 };
            self.selected = next % len;
        }
        FindOutcome::Consumed
    }

    fn edit_query(&mut self, edit: impl FnOnce(&mut String)) -> FindOutcome {
        let before = self.query.len();
        edit(&mut self.query);
        if self.query.len() == before {
            return FindOutcome::Consumed;
        }
        self.selected = 0;
        FindOutcome::Refresh
    }

    fn set_options(&mut self, change: impl FnOnce(&mut SearchOptions)) -> FindOutcome {
        change(&mut self.options);
        self.selected = 0;
        FindOutcome::Refresh
    }

    fn clear_text(&mut self) {
        self.request = None;
        self.searched = None;
        self.results = None;
        self.error = None;
    }
}

fn drop_last_char(query: &mut String) {
    query.pop();
}

/// Case matching's next setting: smart, then exact, then ignored, then smart again.
fn next_case(case_sensitive: Option<bool>) -> Option<bool> {
    match case_sensitive {
        None => Some(true),
        Some(true) => Some(false),
        Some(false) => None,
    }
}

#[cfg(test)]
mod tests;
