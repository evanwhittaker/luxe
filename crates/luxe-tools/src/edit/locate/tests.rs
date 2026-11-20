use super::*;

const FILE: &str = "fn main() {\n    let x = 1;\n    println!(\"{x}\");\n}\n";

#[test]
fn an_exact_match_is_found_and_reported_as_exact() {
    let found = locate(FILE, "    let x = 1;", "    let x = 2;", false).expect("found");
    assert_eq!(found.kind, MatchKind::Exact);
    assert_eq!(found.ranges.len(), 1);
    assert_eq!(found.kind.note(), None, "an exact match needs no explanation");
}

/// The most common near-miss there is: whitespace the model could not see. It only bites on a
/// multi-line search, where the invisible characters sit *inside* the block being matched.
#[test]
fn trailing_whitespace_is_forgiven_and_said_out_loud() {
    let file = "fn main() {\n    let x = 1;   \n    go();\n}\n";
    let found = locate(file, "    let x = 1;\n    go();", "    let x = 2;\n    go();", false)
        .expect("found");
    assert_eq!(found.kind, MatchKind::TrailingWhitespace);
    assert!(found.kind.note().expect("a note").contains("trailing whitespace"));
    assert_eq!(found.ranges.len(), 1);
}

/// The break this catches: a relaxed pass comparing a line by its start, so a search that ends
/// mid-line takes the whole line and the edit rewrites the part the caller never mentioned.
#[test]
fn a_search_ending_mid_line_does_not_take_the_whole_line() {
    let file = "fn main() {\n    let x = 1;   \n    go(1);\n}\n";
    for search in ["    let x = 1;\n    go(", "let x = 1;\n    go("] {
        let error = locate(file, search, "y", false).expect_err("a fragment");
        assert!(matches!(error, LocateError::NotFound { .. }), "{search:?}");
    }
}

/// The break this catches: a relaxed search refused when it spans the whole file, or one longer
/// than the file read past its end.
#[test]
fn a_relaxed_search_may_span_the_whole_file_but_not_more() {
    let file = "a();   \nb();";
    let found = locate(file, "a();\nb();", "c();", false).expect("the whole file");
    assert_eq!(found.ranges, vec![Range { start: 0, end: file.len() }]);
    let error = locate("x();   \n", "x();\ny();\nz();", "c();", false).expect_err("too long");
    assert!(matches!(error, LocateError::NotFound { .. }));
}

/// A uniformly re-indented block is the same block; the replacement moves with it.
#[test]
fn a_uniform_indentation_shift_is_forgiven_and_applied_to_the_replacement() {
    let file = "impl T {\n        fn go(&self) {\n            work();\n        }\n}\n";
    let found = locate(
        file,
        "fn go(&self) {\n    work();\n}",
        "fn go(&self) {\n    work_twice();\n}",
        false,
    )
    .expect("found");
    assert_eq!(found.kind, MatchKind::Indentation { spaces: 8 });
    assert_eq!(
        found.replacement, "        fn go(&self) {\n            work_twice();\n        }",
        "the replacement is re-indented to match the file, not the search text"
    );
}

/// Lines that drifted *differently* are a different block, not a mis-indented one.
#[test]
fn a_non_uniform_shift_is_not_treated_as_indentation() {
    let file = "impl T {\n    fn go(&self) {\n            work();\n    }\n}\n";
    let error = locate(file, "fn go(&self) {\n    work();\n}", "x", false).expect_err("no");
    assert!(matches!(error, LocateError::NotFound { .. }));
}

/// The break this catches: re-indented occurrences refused when they agree on the shift, or taken
/// when they don't, when one replacement can't be re-indented for both.
#[test]
fn every_re_indented_occurrence_must_share_one_shift() {
    let same = "  a();\n  b();\nx\n  a();\n  b();\n";
    let found = locate(same, "a();\nb();", "c();", true).expect("the same shift");
    assert_eq!(found.ranges.len(), 2);
    let differ = "  a();\n  b();\n    a();\n    b();\n";
    let error = locate(differ, "a();\nb();", "c();", true).expect_err("two shifts");
    assert!(matches!(error, LocateError::NotFound { .. }));
}

#[test]
fn several_matches_are_ambiguous_unless_replace_all_was_asked_for() {
    let file = "a();\nb();\na();\n";
    let error = locate(file, "a();", "c();", false).expect_err("ambiguous");
    let LocateError::Ambiguous { lines } = error else { panic!("expected ambiguity") };
    assert_eq!(lines, vec![1, 3], "and it says where, so the fix is to add context");

    let found = locate(file, "a();", "c();", true).expect("replace_all takes them all");
    assert_eq!(found.ranges.len(), 2);
}

/// "not found" alone cannot be acted on; the closest line can.
#[test]
fn a_near_miss_reports_the_closest_line() {
    let error = locate(FILE, "    let x = 11;", "y", false).expect_err("no exact or relaxed match");
    let LocateError::NotFound { nearest } = error else { panic!("expected not-found") };
    let nearest = nearest.expect("a near miss in the same file");
    assert_eq!(nearest.line, 2);
    assert!(nearest.text.contains("let x = 1;"));
}

#[test]
fn an_unrelated_search_reports_no_near_miss_rather_than_a_coincidence() {
    let error = locate(FILE, "quantum_flux_capacitor()", "y", false).expect_err("no match");
    assert_eq!(error, LocateError::NotFound { nearest: None });
}

/// The near miss `locate` reports for `search` in `file`, as its line number.
fn nearest_line(file: &str, search: &str) -> Option<usize> {
    let error = locate(file, search, "y", false).expect_err("no match");
    let LocateError::NotFound { nearest } = error else { panic!("expected not-found") };
    nearest.map(|nearest| nearest.line)
}

/// The break this catches: the near miss taken from the first or last line that shares anything,
/// or from a later line that only ties the best, rather than from the closest.
#[test]
fn the_near_miss_is_the_first_closest_line() {
    let file = "abcd0000\nabcdef00\nabcde000\nabcdef11\n";
    assert_eq!(nearest_line(file, "abcdefgh"), Some(2));
}

/// The break this catches: the cutoffs moved by one, so a 4-character search or a line sharing
/// exactly half the search goes unreported, or a shorter search or a smaller share is reported.
#[test]
fn a_near_miss_needs_four_characters_and_half_of_them_shared() {
    assert_eq!(nearest_line("abc1\n", "abcZ"), Some(1), "four characters is enough");
    assert_eq!(nearest_line("ab1\n", "abZ"), None, "three is too few to judge");
    assert_eq!(nearest_line("abcd1234\n", "abcdWXYZ"), Some(1), "half shared is a near miss");
    assert_eq!(nearest_line("abcd1234\n", "abcdWXYZqq"), None, "under half is a coincidence");
}

#[test]
fn a_dedent_removes_only_whitespace() {
    assert_eq!(reindent("        deep();\n\n        more();", -4), "    deep();\n\n    more();");
    // Never eats real characters when the line is shallower than the shift asks for.
    assert_eq!(reindent("  x();", -8), "x();");
}
