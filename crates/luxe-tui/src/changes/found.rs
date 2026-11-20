//! Found query — what Find was looking for when it opened the file the pane shows.
//!
//! The File view lights every match of it, the way an editor keeps a search lit after jumping to
//! a hit, so the reader sees the hit and its neighbors at once. It belongs to the file it was
//! opened with: showing another file drops it.

use luxe_core::project_search::{QueryMatcher, SearchOptions};

/// A Find query, compiled once for the frames that light it.
#[derive(Debug, Clone)]
pub(crate) struct FoundQuery {
    query: String,
    options: SearchOptions,
    matcher: QueryMatcher,
}

impl FoundQuery {
    /// The query as Find searched it, or `None` when there's nothing to light: an empty query, or
    /// a regex that doesn't compile.
    pub(crate) fn new(query: &str, options: SearchOptions) -> Option<Self> {
        if query.is_empty() {
            return None;
        }
        let matcher = QueryMatcher::new(query, options).ok()?;
        Some(Self { query: query.to_owned(), options, matcher })
    }

    /// What lights the matches.
    pub(crate) fn matcher(&self) -> &QueryMatcher {
        &self.matcher
    }
}

// The compiled matcher follows from the query and its options, so they are the whole identity
impl PartialEq for FoundQuery {
    fn eq(&self, other: &Self) -> bool {
        self.query == other.query && self.options == other.options
    }
}

impl Eq for FoundQuery {}
