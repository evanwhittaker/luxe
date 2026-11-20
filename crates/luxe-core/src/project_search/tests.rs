use super::*;

/// A project with `files` written into a temp folder.
fn project(files: &[(&str, &[u8])]) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a temp dir");
    for (path, contents) in files {
        let full = root.path().join(path);
        std::fs::create_dir_all(full.parent().expect("a file has a folder")).expect("folders");
        std::fs::write(full, contents).expect("a file");
    }
    root
}

fn find(root: &tempfile::TempDir, query: &str, options: SearchOptions) -> SearchResults {
    search(root.path(), query, options, &Arc::default()).expect("the search answers")
}

/// Each hit as `path:line:text`.
fn rows(results: &SearchResults) -> Vec<String> {
    results.hits.iter().map(|hit| format!("{}:{}:{}", hit.path, hit.line, hit.text)).collect()
}

/// The break this catches: a query passed to the regex unescaped, so `a.b` finds `axb` too.
#[test]
fn a_literal_query_matches_only_itself() {
    let root = project(&[("notes.txt", b"a.b\naxb\n")]);
    let found = find(&root, "a.b", SearchOptions::default());
    assert_eq!(rows(&found), ["notes.txt:1:a.b"]);
    assert_eq!(found.query, "a.b", "the answer says what it answers");
}

#[test]
fn a_lowercase_query_ignores_case() {
    let root = project(&[("notes.txt", b"Config\nconfig\n")]);
    assert_eq!(find(&root, "config", SearchOptions::default()).total, 2);
}

#[test]
fn a_capital_in_the_query_makes_case_exact() {
    let root = project(&[("notes.txt", b"Config\nconfig\n")]);
    assert_eq!(rows(&find(&root, "Config", SearchOptions::default())), ["notes.txt:1:Config"]);
}

#[test]
fn asking_for_any_case_overrides_the_capital() {
    let root = project(&[("notes.txt", b"Config\nconfig\n")]);
    let options = SearchOptions { case_sensitive: Some(false), ..SearchOptions::default() };
    assert_eq!(find(&root, "Config", options).total, 2);
}

#[test]
fn a_regex_query_is_read_as_one() {
    let root = project(&[("notes.txt", b"fooo\nfx\n")]);
    let options = SearchOptions { regex: true, ..SearchOptions::default() };
    assert_eq!(rows(&find(&root, "fo+", options)), ["notes.txt:1:fooo"]);
}

/// An anchored regex finds a line in the middle of a file, which a whole-text pass would miss.
#[test]
fn an_anchored_regex_matches_lines_mid_file() {
    let root = project(&[("notes.txt", b"intro\nfn main\n")]);
    let options = SearchOptions { regex: true, ..SearchOptions::default() };
    assert_eq!(rows(&find(&root, "^fn", options)), ["notes.txt:2:fn main"]);
}

#[test]
fn a_broken_regex_says_so() {
    let root = project(&[("notes.txt", b"x\n")]);
    let options = SearchOptions { regex: true, ..SearchOptions::default() };
    let failure = search(root.path(), "(", options, &Arc::default());
    assert!(matches!(failure, Err(SearchFailure::InvalidPattern(_))), "{failure:?}");
}

#[test]
fn every_match_on_a_line_has_its_range() {
    let root = project(&[("notes.txt", b"x foo y foo\n")]);
    let hit = &find(&root, "foo", SearchOptions::default()).hits[0];
    let ranges: Vec<(usize, usize)> = hit.matches.iter().map(|m| (m.start, m.end)).collect();
    assert_eq!(ranges, [(2, 5), (8, 11)]);
}

/// The break this catches: the matcher a screen lights matches with reading a query differently
/// from the search that found the line, so a hit shows with nothing lit, or the wrong thing lit.
#[test]
fn the_public_matcher_lights_what_the_search_found() {
    let root = project(&[("notes.txt", b"a.b axb Foo foo\n")]);
    for query in ["a.b", "Foo", "foo"] {
        let hit = &find(&root, query, SearchOptions::default()).hits[0];
        let matcher =
            QueryMatcher::new(query, SearchOptions::default()).expect("a literal compiles");
        assert_eq!(matcher.find_in(&hit.text), hit.matches, "{query}");
    }
    let matcher = QueryMatcher::new("Foo", SearchOptions::default()).expect("a literal compiles");
    assert_eq!(matcher.find_in("foo Foo"), [MatchRange { start: 4, end: 7 }], "smart case");
}

/// A regex that can match nothing finds no line on that alone.
#[test]
fn an_empty_match_is_not_a_hit() {
    let root = project(&[("notes.txt", b"abc\nxyz\n")]);
    let options = SearchOptions { regex: true, ..SearchOptions::default() };
    assert_eq!(rows(&find(&root, "a*", options)), ["notes.txt:1:abc"]);
}

#[test]
fn ignored_files_are_left_out_unless_asked_for() {
    let root = project(&[(".gitignore", b"build/\n"), ("build/out.txt", b"needle\n")]);
    assert_eq!(find(&root, "needle", SearchOptions::default()).total, 0);
    let options = SearchOptions { include_ignored: true, ..SearchOptions::default() };
    assert_eq!(find(&root, "needle", options).total, 1);
}

#[test]
fn git_internals_are_never_searched() {
    let root = project(&[(".git/config", b"needle\n")]);
    let options = SearchOptions { include_ignored: true, ..SearchOptions::default() };
    assert_eq!(find(&root, "needle", options).total, 0);
}

#[test]
fn binary_files_are_skipped() {
    let root = project(&[("blob.bin", b"needle\0\x01"), ("text.txt", b"needle\n")]);
    assert_eq!(rows(&find(&root, "needle", SearchOptions::default())), ["text.txt:1:needle"]);
}

/// By path component, as the FILES tree lists them: a folder's files before a sibling file.
#[test]
fn hits_come_in_path_order() {
    let root = project(&[("b.txt", b"needle\n"), ("a/z.txt", b"needle\n"), ("a.txt", b"needle\n")]);
    let found = find(&root, "needle", SearchOptions::default());
    let paths: Vec<&str> = found.hits.iter().map(|hit| hit.path.as_str()).collect();
    assert_eq!(paths, ["a/z.txt", "a.txt", "b.txt"]);
}

/// The break this catches: the total counting only what was carried, so a cap reads as all.
#[test]
fn past_the_cap_hits_are_counted_not_carried() {
    let many = "needle\n".repeat(MAX_HITS + 5);
    let root = project(&[("many.txt", many.as_bytes()), ("more.txt", b"needle\n")]);
    let found = find(&root, "needle", SearchOptions::default());
    assert_eq!((found.hits.len(), found.total), (MAX_HITS, MAX_HITS + 6));
}

#[test]
fn a_long_line_is_clipped_around_its_match() {
    let line = format!("{}needle{}\n", "a".repeat(1_000), "b".repeat(1_000));
    let root = project(&[("min.js", line.as_bytes())]);
    let hit = &find(&root, "needle", SearchOptions::default()).hits[0];
    assert!(hit.clipped && hit.text.len() <= MAX_LINE_BYTES, "{} bytes", hit.text.len());
    let lit = &hit.text[hit.matches[0].start..hit.matches[0].end];
    assert_eq!(lit, "needle", "the range moves with the clip");
}

/// A clip that would cut a character in half backs up to its start, keeping the full lead.
/// `abcd` puts the lead's edge inside a three-byte `€`; with the needle right after the `€`s, a
/// lead divisible by three would land on a boundary and never test it.
#[test]
fn a_clip_never_splits_a_character() {
    let line = format!("{}abcdneedle{}\n", "€".repeat(400), "€".repeat(400));
    let root = project(&[("wide.txt", line.as_bytes())]);
    let hit = &find(&root, "needle", SearchOptions::default()).hits[0];
    assert_eq!(&hit.text[hit.matches[0].start..hit.matches[0].end], "needle");
    assert!(
        hit.matches[0].start >= LEAD_BYTES,
        "backs up, never forward: {}",
        hit.matches[0].start
    );
}

/// The break this catches: a match past the clipped window kept, with a range off the text's end.
#[test]
fn a_match_past_the_clip_is_dropped() {
    let line = format!("needle{}needle{}\n", "a".repeat(MAX_LINE_BYTES - 6), "b".repeat(500));
    let root = project(&[("min.js", line.as_bytes())]);
    let hit = &find(&root, "needle", SearchOptions::default()).hits[0];
    assert_eq!(
        hit.matches,
        [MatchRange { start: 0, end: 6 }],
        "the second starts at the clip's end"
    );
}

#[test]
fn a_file_at_the_size_cap_is_searched() {
    let mut text = "needle\n".to_owned();
    text.push_str(&"x".repeat(MAX_FILE_BYTES as usize - text.len()));
    let root = project(&[("big.txt", text.as_bytes())]);
    assert_eq!(find(&root, "needle", SearchOptions::default()).total, 1);
}

#[test]
fn a_file_past_the_size_cap_is_skipped() {
    let mut text = "needle\n".to_owned();
    text.push_str(&"x".repeat(MAX_FILE_BYTES as usize + 1 - text.len()));
    let root = project(&[("big.txt", text.as_bytes())]);
    assert_eq!(find(&root, "needle", SearchOptions::default()).total, 0);
}

#[test]
fn an_empty_query_finds_nothing() {
    let root = project(&[("notes.txt", b"anything\n")]);
    let found = find(&root, "", SearchOptions::default());
    assert_eq!((found.total, found.files_searched), (0, 0));
}

#[test]
fn a_stopped_search_reports_it_was_replaced() {
    let root = project(&[("notes.txt", b"needle\n")]);
    let stop = Arc::new(AtomicBool::new(true));
    let stopped = search(root.path(), "needle", SearchOptions::default(), &stop);
    assert_eq!(stopped, Err(SearchFailure::Superseded));
}

/// The break this catches: a new search leaving the old one running, so stale scans pile up.
#[test]
fn starting_a_search_stops_the_one_before() {
    let slot = SearchSlot::default();
    let first = slot.start();
    let second = slot.start();
    assert!(first.load(Ordering::Relaxed), "the first is stopped");
    assert!(!second.load(Ordering::Relaxed), "the newest runs");
}

fn request_one() -> SearchRequestId {
    SearchRequestId("search:1".into())
}

/// The frames a client reads, pinned against literals so a rename breaks here first.
#[test]
fn the_answer_frames_read_as_clients_parse_them() {
    let results = SearchResults { query: "needle".into(), ..SearchResults::default() };
    let found = crate::event::Event::ProjectSearched(Box::new(SearchAnswer {
        request_id: request_one(),
        results,
    }));
    let failed = crate::event::Event::ProjectSearchFailed(SearchFailed {
        request_id: request_one(),
        query: "(".into(),
        error: SearchFailure::InvalidPattern("unclosed group".into()),
    });
    assert_eq!(
        serde_json::to_string(&found).expect("serializes"),
        r#"{"kind":"project_searched","content":{"request_id":"search:1","results":{"query":"needle","hits":[],"total":0,"files_searched":0}}}"#
    );
    assert_eq!(
        serde_json::to_string(&failed).expect("serializes"),
        r#"{"kind":"project_search_failed","content":{"request_id":"search:1","query":"(","error":{"invalid_pattern":"unclosed group"}}}"#
    );
}

/// A client that leaves out the options gets the defaults: literal, smart case, ignores honored.
#[test]
fn a_search_without_options_parses_with_the_defaults() {
    let line = r#"{"kind":"search_project","content":{"request_id":"search:1","query":"needle"}}"#;
    let parsed: crate::event::Action = serde_json::from_str(line).expect("parses");
    let expected = crate::event::Action::SearchProject {
        request_id: request_one(),
        query: "needle".into(),
        options: SearchOptions::default(),
    };
    assert_eq!(parsed, expected);
}

/// Pinned against a literal, so a renamed field breaks a client's parse here first.
#[test]
fn results_serialize_as_clients_read_them() {
    let hit = SearchHit {
        path: "src/main.rs".into(),
        line: 3,
        text: "fn main() {}".into(),
        clipped: false,
        matches: vec![MatchRange { start: 3, end: 7 }],
        scope: None,
    };
    let json = serde_json::to_string(&hit).expect("serializes");
    assert_eq!(
        json,
        r#"{"path":"src/main.rs","line":3,"text":"fn main() {}","matches":[{"start":3,"end":7}]}"#
    );
    let scoped = SearchHit { scope: Some("App › run".into()), ..hit };
    let json = serde_json::to_string(&scoped).expect("serializes");
    assert!(json.ends_with(r#","scope":"App › run"}"#), "{json}");
}

/// Every file as two scopes: `Outer` over lines 0 to 10, and `inner` inside it over 2 to 4.
struct TwoScopes;

impl crate::hooks::file_changed::OutlineSource for TwoScopes {
    fn functions(
        &self,
        _path: &std::path::Path,
        _source: &str,
    ) -> Option<Vec<crate::hooks::file_changed::FunctionSize>> {
        None
    }

    fn scopes(
        &self,
        _path: &std::path::Path,
        _source: &str,
    ) -> Option<Vec<crate::hooks::file_changed::Scope>> {
        let scope = |name: &str, start_line, end_line| crate::hooks::file_changed::Scope {
            name: name.into(),
            start_line,
            end_line,
        };
        Some(vec![scope("Outer", 0, 10), scope("inner", 2, 4)])
    }
}

/// Each hit's line and scope, after labeling.
fn scopes_of(text: &str) -> Vec<(usize, Option<String>)> {
    let root = project(&[("lib.rs", text.as_bytes())]);
    let mut found = find(&root, "needle", SearchOptions::default());
    label_scopes(root.path(), &mut found, &TwoScopes, &AtomicBool::new(false));
    found.hits.into_iter().map(|hit| (hit.line, hit.scope)).collect()
}

/// The break this catches: a hit labeled with only its outermost or innermost scope, one outside
/// every scope given a name, or one on a declaration named for the scope its own line opens.
#[test]
fn hits_are_labeled_with_the_scopes_around_them() {
    let mut lines = vec![""; 13];
    lines[2] = "needle opens inner";
    lines[3] = "needle in inner";
    lines[6] = "needle in outer";
    lines[12] = "needle outside";
    let labeled = scopes_of(&lines.join("\n"));
    let label = |text: &str| Some(text.to_owned());
    assert_eq!(
        labeled,
        [(3, label("Outer")), (4, label("Outer › inner")), (7, label("Outer")), (13, None)]
    );
}

/// The break this catches: labeling that ignores `stop`, parsing on for a search already replaced.
#[test]
fn labeling_stops_when_asked() {
    let root = project(&[("lib.rs", b"\n\n\nneedle\n")]);
    let mut found = find(&root, "needle", SearchOptions::default());
    label_scopes(root.path(), &mut found, &TwoScopes, &AtomicBool::new(true));
    assert_eq!(found.hits[0].scope, None);
}
