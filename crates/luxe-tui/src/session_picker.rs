//! Session picker — searchable resume flow for the start screen and `/sessions`.
//!
//! The CLI prepares a bounded catalog, at startup for the start screen and on demand mid-session;
//! this module only filters, renders, and returns the selected session ID. Disk access and resume
//! logic stay outside the TUI. It draws through [`crate::picker::render_picker`] with a preview
//! pane, as Find does, the pane showing the session under the cursor the way its transcript did.

mod preview;

use crate::dashboard::{PickerKind, PreviewTurn, SessionSummary};
use crate::glyphs::Icon;
use crate::picker::{render_picker, Edge, PickerNav, PickerRow, PickerView, PreviewPane};
use crate::theme::styles::ThemeStyles;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Rect, widgets::Paragraph, Frame};

/// The keys the picker answers, along its bottom edge.
const HINT: &str = "enter resume · ctrl+u clear · ↑/↓ choose · esc back";

/// A searchable session row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEntry {
    /// Durable session identifier
    pub id: String,
    /// Human title derived from the first user message
    pub name: String,
    /// Relative modification time
    pub age: String,
    /// Bounded transcript text used for grep-style search
    pub search_text: String,
    /// Role-tagged transcript excerpt rendered (with markdown) in the preview pane
    pub preview: Vec<PreviewTurn>,
}

impl From<&SessionSummary> for SessionEntry {
    fn from(summary: &SessionSummary) -> Self {
        Self {
            id: summary.id.clone(),
            name: summary.name.clone(),
            age: summary.age.clone(),
            search_text: summary.search_text.clone(),
            preview: summary.preview.clone(),
        }
    }
}

/// What a key did in the session picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionPickerKey {
    /// Enter accepted this session
    Selected(String),
    /// The picker used the key, or swallowed it so the composer behind it never sees it
    Used,
    /// A chord the picker has no use for, left for the app's global bindings
    Ignored,
}

/// Pure state for the find/recent/grep session picker.
#[derive(Debug)]
pub struct SessionPickerState {
    kind: PickerKind,
    sessions: Vec<SessionEntry>,
    nav: PickerNav,
    /// The read of the catalog it is waiting for, while it waits
    reading: Option<u64>,
}

impl SessionPickerState {
    /// Creates a find-session picker.
    pub fn new(sessions: Vec<SessionEntry>) -> Self {
        Self::for_kind(sessions, PickerKind::FindSession)
    }

    /// Creates the picker variant requested by a dashboard action.
    pub fn for_kind(sessions: Vec<SessionEntry>, kind: PickerKind) -> Self {
        Self { kind, nav: PickerNav::new(sessions.len()), sessions, reading: None }
    }

    /// Creates a find-session picker for a catalog that is still being read. Args: `(request)`,
    /// the read it waits for.
    ///
    /// It opens at once and takes a search while it waits, so the reader is never held on a
    /// frame that is walking session logs.
    pub(crate) fn reading(request: u64) -> Self {
        Self { reading: Some(request), ..Self::new(Vec::new()) }
    }

    /// Whether the catalog it lists is still being read.
    pub(crate) fn is_reading(&self) -> bool {
        self.reading.is_some()
    }

    /// Whether `request` is the read it is waiting for.
    pub(crate) fn waits_for(&self, request: u64) -> bool {
        self.reading == Some(request)
    }

    /// Lists `sessions`, the catalog a [`Self::reading`] picker was waiting for, keeping whatever
    /// was typed meanwhile.
    pub(crate) fn fill(&mut self, sessions: Vec<SessionEntry>) {
        let query = self.query().to_string();
        *self = Self::for_kind(sessions, self.kind.clone());
        if !query.is_empty() {
            self.set_query(&query);
        }
    }

    /// Returns the current search query.
    pub fn query(&self) -> &str {
        self.nav.query()
    }

    /// The cursor's row among the visible matches.
    pub(crate) fn cursor(&self) -> usize {
        self.nav.selected()
    }

    /// Returns the number of visible matches.
    pub fn match_count(&self) -> usize {
        self.nav.match_count()
    }

    /// Replaces the search query and recomputes ranked matches.
    pub fn set_query(&mut self, query: &str) {
        let haystacks = searchable(&self.sessions);
        self.nav.set_query(query, &haystacks);
    }

    /// Filters by pasted text, the way typing it would.
    pub fn paste(&mut self, text: &str) {
        let haystacks = searchable(&self.sessions);
        self.nav.paste_query(text, &haystacks);
    }

    /// Moves the selection up, wrapping from the first result to the last.
    pub fn move_up(&mut self) {
        self.nav.move_up();
    }

    /// Moves the selection down, wrapping from the last result to the first.
    pub fn move_down(&mut self) {
        self.nav.move_down();
    }

    /// Moves the highlight to filtered-list row `index` (mouse hover or click), clamped.
    pub fn select(&mut self, index: usize) {
        self.nav.select(index);
    }

    /// The session id at filtered-list row `index` (a mouse click on that row).
    pub fn session_id_at(&self, index: usize) -> Option<String> {
        let entry = self.nav.item_index_at(index)?;
        self.sessions.get(entry).map(|session| session.id.clone())
    }

    /// The session under the cursor.
    pub fn selected_session(&self) -> Option<&SessionEntry> {
        self.sessions.get(self.nav.selected_item_index()?)
    }

    /// Handles picker-local input: the session Enter accepts, a key the picker used, or one it
    /// left for the app (a chord, which may be a global binding).
    pub fn on_key(&mut self, event: &KeyEvent) -> SessionPickerKey {
        match event.code {
            KeyCode::Up if event.modifiers.is_empty() => self.nav.move_up(),
            KeyCode::Down if event.modifiers.is_empty() => self.nav.move_down(),
            KeyCode::Enter if event.modifiers.is_empty() => {
                return self.selected_session().map_or(SessionPickerKey::Used, |session| {
                    SessionPickerKey::Selected(session.id.clone())
                });
            }
            _ => {
                let edited = self.nav.edit_query(event, || searchable(&self.sessions));
                let chord = event.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
                if !edited && chord {
                    return SessionPickerKey::Ignored;
                }
            }
        }
        SessionPickerKey::Used
    }

    /// Renders the picker centered over the dashboard: the sessions, and beside or under them the
    /// one under the cursor as its transcript showed it.
    pub fn view(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &ThemeStyles,
        hit: &std::cell::RefCell<crate::mouse::HitRegions>,
    ) {
        if area.width < 28 || area.height < 8 {
            let note = Paragraph::new("Resize to browse sessions").style(theme.muted);
            return frame.render_widget(note, area);
        }
        let (empty, count) = (self.empty_message(), self.count());
        let session = self.selected_session();
        let lines =
            |width, height| preview::preview_lines(session, self.query(), width, height, theme);
        let modal = crate::picker::preview_modal(area);
        let list_width = crate::picker::preview_list_width(modal);
        let view = PickerView {
            glyph: Icon::Session.glyph(),
            title: self.title(),
            empty_message: empty,
            item_count: self.match_count(),
            selected: self.cursor(),
            row: |position| self.row(position),
            hit_target: crate::mouse::MouseTarget::SessionRow,
            hint: Some(HINT),
            query: Some(self.query()),
            count: count.as_deref(),
            tabs: None,
            divider: None,
            detail: None,
            rows: None,
            list: crate::mouse::OverlayList::Sessions,
            preview: Some(PreviewPane { list_width, lines: &lines }),
        };
        render_picker(frame, modal, Edge::Below, theme, hit, view);
    }

    /// The session at filtered `position` as a list row: its name, and how long ago it ran.
    fn row(&self, position: usize) -> PickerRow {
        let Some(session) = self.nav.item_index_at(position).and_then(|at| self.sessions.get(at))
        else {
            return PickerRow::default();
        };
        PickerRow {
            label: session.name.clone(),
            detail: Some(session.age.clone()),
            detail_right: true,
            ..PickerRow::default()
        }
    }

    fn empty_message(&self) -> &'static str {
        match (self.is_reading(), self.query().is_empty()) {
            (true, _) => "Reading sessions…",
            (false, true) => "No sessions to open yet",
            (false, false) => "No sessions match this search",
        }
    }

    /// How many sessions it lists, or how many matched a search; nothing while it reads them.
    fn count(&self) -> Option<String> {
        if self.is_reading() {
            return None;
        }
        let count = self.match_count();
        let noun = match (self.query().is_empty(), count == 1) {
            (true, true) => "session",
            (true, false) => "sessions",
            (false, true) => "match",
            (false, false) => "matches",
        };
        Some(format!("{count} {noun}"))
    }

    fn title(&self) -> &'static str {
        match self.kind {
            PickerKind::FindSession => "Find session",
            PickerKind::RecentSessions => "Recent sessions",
            PickerKind::GrepSessions => "Grep sessions",
        }
    }
}

/// What a reader can type at a session row: its title plus the searchable body text.
fn searchable(sessions: &[SessionEntry]) -> Vec<String> {
    sessions.iter().map(|session| format!("{} {}", session.name, session.search_text)).collect()
}

#[cfg(test)]
mod tests;
