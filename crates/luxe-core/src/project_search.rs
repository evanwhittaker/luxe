//! Project search — the reader's Find: the lines in the project that match what they typed.
//!
//! The `grep` tool answers the model; this answers a person, live, as they type, so it differs in
//! what it optimizes for. The query is literal text unless asked to be a regex (what a person types
//! is rarely one), case is smart (a capital makes it exact, as ripgrep's `--smart-case`), every match
//! comes with its byte range so the overlay can light it, and hits are capped with the true count
//! kept beside them. It walks what the agent's tools walk ([`crate::walk`]): `.gitignore` honored
//! unless asked otherwise, `.git` never. `Action::SearchProject` asks for it, and the engine answers
//! off the loop, stopping the search before when a newer one arrives.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Most hits one search carries; the rest are counted, not carried.
pub const MAX_HITS: usize = 500;

/// Bytes past which a file is skipped: generated bundles and data, not something a person greps.
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

/// Bytes of a long line kept around its first match, so a minified file can't fill the overlay.
const MAX_LINE_BYTES: usize = 300;

/// Bytes kept ahead of the first match when a long line is clipped.
const LEAD_BYTES: usize = 60;

/// Matches lit on one line; a line with more is already found.
const MAX_RANGES_PER_LINE: usize = 16;

/// Identifies one search request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SearchRequestId(String);

impl SearchRequestId {
    /// A distinct request id.
    pub fn new() -> Self {
        Self(format!("search:{}", Uuid::new_v4()))
    }
}

impl Default for SearchRequestId {
    fn default() -> Self {
        Self::new()
    }
}

/// How to read a query.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Read the query as a regex rather than literal text
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub regex: bool,
    /// Match case exactly, or not; absent, a capital letter in the query turns it on
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_sensitive: Option<bool>,
    /// Search what `.gitignore` ignores too
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub include_ignored: bool,
}

/// One line that matched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHit {
    /// Project-relative, with forward slashes
    pub path: String,
    /// 1-based
    pub line: usize,
    /// The line, clipped around its first match when it's long
    pub text: String,
    /// Whether `text` is only part of the line
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub clipped: bool,
    /// Where `text` matched
    pub matches: Vec<MatchRange>,
    /// The scopes around the line, outermost first, as `App › run`; absent where nothing
    /// parsed the file or the line sits in no scope
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// One match, as byte offsets into [`SearchHit::text`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchRange {
    /// First byte of the match
    pub start: usize,
    /// Byte just past the match
    pub end: usize,
}

/// What one search found.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResults {
    /// The query as searched
    pub query: String,
    /// The matching lines, in path then line order, at most [`MAX_HITS`]
    pub hits: Vec<SearchHit>,
    /// Matching lines in all, `hits` included
    pub total: usize,
    /// Files the walk offered, read or skipped
    pub files_searched: usize,
    /// The walk stopped at its ceiling, so later files weren't searched
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub walk_capped: bool,
}

/// A search's answer, for the request that asked: `Event::ProjectSearched`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchAnswer {
    /// Correlates the answer with its request
    pub request_id: SearchRequestId,
    /// The matching lines and how many there were
    pub results: SearchResults,
}

/// A search that has no answer, for the request that asked: `Event::ProjectSearchFailed`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchFailed {
    /// Correlates the failure with its request
    pub request_id: SearchRequestId,
    /// The query that was asked for
    pub query: String,
    /// Why not
    pub error: SearchFailure,
}

/// Why a search has no answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[serde(rename_all = "snake_case")]
pub enum SearchFailure {
    /// The query was asked for as a regex and isn't one.
    #[error("not a valid regex: {0}")]
    InvalidPattern(String),
    /// A newer search stopped this one.
    #[error("a newer search replaced it")]
    Superseded,
    /// The session has no project folder.
    #[error("this session has no project to search")]
    NoProject,
    /// A search worker crashed, so what it scanned is unknown.
    #[error("the search failed: {0}")]
    Failed(String),
}

/// The one search in flight. Starting another stops it, since a reader typing asks once per
/// keystroke and only the newest answer is read.
#[derive(Debug, Default)]
pub(crate) struct SearchSlot(std::sync::Mutex<Arc<AtomicBool>>);

impl SearchSlot {
    /// Stops the search in flight and returns the flag that stops the one starting now.
    pub(crate) fn start(&self) -> Arc<AtomicBool> {
        let fresh = Arc::new(AtomicBool::new(false));
        let mut current = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let previous = std::mem::replace(&mut *current, Arc::clone(&fresh));
        previous.store(true, Ordering::Relaxed);
        fresh
    }
}

/// One file's matching lines: the first [`MAX_HITS`] of them, and how many there were.
#[derive(Debug, Default)]
struct FileHits {
    hits: Vec<SearchHit>,
    total: usize,
}

/// A query compiled the way [`search`] reads it, so whatever lights matches on screen lights the
/// ones the search found: literal unless a regex, case smart unless set.
#[derive(Debug, Clone)]
pub struct QueryMatcher {
    regex: regex::Regex,
    /// A literal can't be anchored to a line's start or end, so a whole-text miss is a miss
    whole_text_rules_out: bool,
}

impl QueryMatcher {
    /// Compiles `query` under `options`. Fails only when a regex was asked for and it isn't one.
    pub fn new(query: &str, options: SearchOptions) -> Result<Self, SearchFailure> {
        let pattern = if options.regex { query.to_owned() } else { regex::escape(query) };
        let exact = options.case_sensitive.unwrap_or_else(|| query.chars().any(char::is_uppercase));
        let regex = regex::RegexBuilder::new(&pattern)
            .case_insensitive(!exact)
            .build()
            .map_err(|error| SearchFailure::InvalidPattern(error.to_string()))?;
        Ok(Self { regex, whole_text_rules_out: !options.regex })
    }

    /// Where `line` matches, at most a few per line. An empty match finds nothing a person could
    /// see, so it doesn't count.
    pub fn find_in(&self, line: &str) -> Vec<MatchRange> {
        self.regex
            .find_iter(line)
            .filter(|found| !found.is_empty())
            .take(MAX_RANGES_PER_LINE)
            .map(|found| MatchRange { start: found.start(), end: found.end() })
            .collect()
    }
}

/// Searches the project under `root` for `query`, until `stop` is raised.
pub fn search(
    root: &Path,
    query: &str,
    options: SearchOptions,
    stop: &Arc<AtomicBool>,
) -> Result<SearchResults, SearchFailure> {
    let mut results = SearchResults { query: query.to_owned(), ..SearchResults::default() };
    if query.is_empty() {
        return Ok(results);
    }
    let matcher = QueryMatcher::new(query, options)?;
    let walked = walk(root, options, stop);
    let scanned = scan_all(root, &matcher, &walked.files, stop)?;
    if stop.load(Ordering::Relaxed) {
        return Err(SearchFailure::Superseded);
    }
    results.walk_capped = walked.capped;
    results.files_searched = walked.files.len();
    gather(&mut results, scanned);
    Ok(results)
}

/// The files to search, sorted, stopping early once `stop` is raised.
fn walk(root: &Path, options: SearchOptions, stop: &Arc<AtomicBool>) -> crate::walk::Walked {
    let stop = Arc::clone(stop);
    let stopped = move || stop.load(Ordering::Relaxed);
    crate::walk::files_until(root, root, options.include_ignored, None, stopped)
}

/// Scans `files` in parallel. Each worker takes a contiguous run of the sorted paths and the runs
/// are joined in order, so the answer reads the same however many threads made it.
fn scan_all(
    root: &Path,
    matcher: &QueryMatcher,
    files: &[PathBuf],
    stop: &AtomicBool,
) -> Result<Vec<FileHits>, SearchFailure> {
    let workers = std::thread::available_parallelism().map_or(1, usize::from).min(8);
    let run = files.len().div_ceil(workers).max(1);
    let joined: Vec<std::thread::Result<Vec<FileHits>>> = std::thread::scope(|scope| {
        let handles: Vec<_> = files
            .chunks(run)
            .map(|chunk| scope.spawn(move || scan_chunk(root, matcher, chunk, stop)))
            .collect();
        handles.into_iter().map(|handle| handle.join()).collect()
    });
    if joined.iter().any(Result::is_err) {
        return Err(SearchFailure::Failed("a search worker crashed".into()));
    }
    Ok(joined.into_iter().flatten().flatten().collect())
}

/// One worker's run of files, stopping between files once `stop` is raised.
fn scan_chunk(
    root: &Path,
    matcher: &QueryMatcher,
    chunk: &[PathBuf],
    stop: &AtomicBool,
) -> Vec<FileHits> {
    let mut found = Vec::new();
    for relative in chunk {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        found.extend(scan_file(root, relative, matcher));
    }
    found
}

/// One file's matching lines, or `None` when it has none or isn't text worth reading.
fn scan_file(root: &Path, relative: &Path, matcher: &QueryMatcher) -> Option<FileHits> {
    let text = readable_text(&root.join(relative))?;
    if matcher.whole_text_rules_out && !matcher.regex.is_match(&text) {
        return None;
    }
    let path = crate::paths::display(relative);
    let mut found = FileHits::default();
    // a per-file cap only bounds memory; `gather` keeps the first MAX_HITS across files anyway
    for (index, line) in text.lines().enumerate() {
        let matches = matcher.find_in(line);
        if matches.is_empty() {
            continue;
        }
        found.total += 1;
        if found.hits.len() < MAX_HITS {
            found.hits.push(hit(&path, index + 1, line, matches));
        }
    }
    (found.total > 0).then_some(found)
}

/// Names the scopes each hit sits in, where `outline` parses its file, until `stop` is raised.
///
/// It labels only the hits a search kept, after the search, so the parse is paid once per file a
/// reader can see rather than for every file that matched.
pub fn label_scopes(
    root: &Path,
    results: &mut SearchResults,
    outline: &dyn crate::hooks::file_changed::OutlineSource,
    stop: &AtomicBool,
) {
    let mut parsed: Option<(String, Option<Vec<crate::hooks::file_changed::Scope>>)> = None;
    for hit in &mut results.hits {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        // a file's hits sit together, so each file is parsed once
        if parsed.as_ref().is_none_or(|(path, _)| *path != hit.path) {
            parsed = Some((hit.path.clone(), parse_scopes(root, &hit.path, outline)));
        }
        let scopes = parsed.as_ref().and_then(|(_, scopes)| scopes.as_deref());
        let line = hit.line.saturating_sub(1);
        hit.scope = scopes.and_then(|scopes| crate::hooks::file_changed::scope_label(scopes, line));
    }
}

/// The scopes in the file at project-relative `path`, where `outline` parses it.
fn parse_scopes(
    root: &Path,
    path: &str,
    outline: &dyn crate::hooks::file_changed::OutlineSource,
) -> Option<Vec<crate::hooks::file_changed::Scope>> {
    let text = readable_text(&root.join(path))?;
    outline.scopes(Path::new(path), &text)
}

/// A file's text, unless it's too big, binary, or not UTF-8.
fn readable_text(path: &Path) -> Option<String> {
    let size = std::fs::metadata(path).ok()?.len();
    if size > MAX_FILE_BYTES {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    if crate::project_files::looks_binary(&bytes) {
        return None;
    }
    String::from_utf8(bytes).ok()
}

/// A hit for `text`, clipped to a window from just before its first match when it's long.
fn hit(path: &str, line: usize, text: &str, matches: Vec<MatchRange>) -> SearchHit {
    let path = path.to_owned();
    let whole = SearchHit { path, line, text: String::new(), clipped: false, matches, scope: None };
    if text.len() <= MAX_LINE_BYTES {
        return SearchHit { text: text.to_owned(), ..whole };
    }
    let first = whole.matches.first().map_or(0, |found| found.start);
    // starts at or before the first match, so no match begins ahead of the window
    let start = floor_boundary(text, first.saturating_sub(LEAD_BYTES));
    let end = floor_boundary(text, (start + MAX_LINE_BYTES).min(text.len()));
    let matches = whole
        .matches
        .iter()
        .filter(|found| found.start < end)
        .map(|found| MatchRange { start: found.start - start, end: found.end.min(end) - start })
        .collect();
    SearchHit { text: text[start..end].to_owned(), clipped: true, matches, ..whole }
}

/// The nearest char boundary at or before `at`.
fn floor_boundary(text: &str, mut at: usize) -> usize {
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// Joins the files' hits in order, keeping the first [`MAX_HITS`] and counting the rest.
fn gather(results: &mut SearchResults, scanned: Vec<FileHits>) {
    for file in scanned {
        results.total += file.total;
        let room = MAX_HITS.saturating_sub(results.hits.len());
        results.hits.extend(file.hits.into_iter().take(room));
    }
}

#[cfg(test)]
mod tests;
