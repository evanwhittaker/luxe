use super::*;
use luxe_core::project_files::ProjectFileFailure;

fn file(path: &str) -> ProjectFile {
    ProjectFile {
        path: path.into(),
        text: "one\ntwo\n".into(),
        bytes: 8,
        truncated: false,
        binary: false,
        image: None,
    }
}

/// The break this catches: the same file read again on every cursor move, or a read already out
/// asked for a second time while it runs.
#[test]
fn a_file_is_asked_for_once() {
    let mut cache = PreviewCache::default();
    assert!(cache.wants("a.rs"));
    let request = FileReadRequestId::new();
    cache.reading(request.clone(), "a.rs");
    assert!(!cache.wants("a.rs"), "not while it's being read");
    assert!(cache.accept(&request, &file("a.rs")));
    assert!(!cache.wants("a.rs"), "nor once it's here");
    assert!(matches!(cache.get("a.rs"), Some(PreviewText::Ready(_))));
}

/// The break this catches: Find taking a read the preview pane asked for, which then never lands
/// where it was wanted.
#[test]
fn a_read_find_did_not_ask_for_is_not_taken() {
    let mut cache = PreviewCache::default();
    assert!(!cache.accept(&FileReadRequestId::new(), &file("a.rs")));
    assert!(!cache.fail(&FileReadRequestId::new(), &ProjectFileFailure::NotFound));
    assert_eq!(cache.get("a.rs"), None);
}

/// The break this catches: a read that failed showing "reading" forever, or asked for again on
/// every move.
#[test]
fn a_failed_read_says_why_and_is_not_asked_again() {
    let mut cache = PreviewCache::default();
    let request = FileReadRequestId::new();
    cache.reading(request.clone(), "gone.rs");
    assert!(cache.fail(&request, &ProjectFileFailure::NotFound));
    assert!(matches!(cache.get("gone.rs"), Some(PreviewText::Failed(_))));
    assert!(!cache.wants("gone.rs"));
}

/// The break this catches: a long session of browsing holding every file it ever previewed.
#[test]
fn only_the_newest_files_are_kept() {
    let mut cache = PreviewCache::default();
    for index in 0..=MAX_FILES {
        let path = format!("f{index}.rs");
        let request = FileReadRequestId::new();
        cache.reading(request.clone(), &path);
        cache.accept(&request, &file(&path));
    }
    assert!(cache.wants("f0.rs"), "the oldest was let go");
    assert!(!cache.wants("f1.rs"), "and only the oldest: it holds {MAX_FILES}");
    assert!(!cache.wants(&format!("f{MAX_FILES}.rs")));
}
