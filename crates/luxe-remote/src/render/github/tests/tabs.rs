use crate::render::{escape, render, Piece, Renderer, StatusFacts, StatusSegment, Voice};

/// A renderer whose tab marks are the search they were picked for, in brackets.
struct Searched;

impl Renderer for Searched {
    fn markdown(&self, text: &str, _voice: Voice) -> String {
        escape(text)
    }
    fn tool_glyph(&self, _tool: &str) -> String {
        String::new()
    }
    fn status(&self, _facts: &StatusFacts) -> Vec<StatusSegment> {
        Vec::new()
    }
    fn github_section_mark(&self, query: &str) -> String {
        format!("[{query}]")
    }
}

fn tabs(shown: &str) -> String {
    let piece: Piece = serde_json::from_value(serde_json::json!({
        "kind": "github_tabs",
        "shown": shown,
        "tabs": [
            {"name": "Mine", "query": "is:pr author:@me", "count": 2},
            {"name": "Pull <requests>", "query": "is:pr", "count": 3250},
            {"name": "Old host", "count": 0},
        ],
    }))
    .expect("the tabs as the app sends them");
    render(&Searched, &piece)
}

/// The break this catches: a tab marked for another tab's search, its count run together, the open
/// one not said, or a name written unescaped (into the button's data too, which the tap reads).
#[test]
fn each_tab_wears_its_searchs_mark_and_count() {
    let html = tabs("Mine");
    let mine =
        "<button type=\"button\" class=\"gh-tab\" data-section=\"Mine\" aria-pressed=\"true\">\
        <span class=\"gh-tab-mark\" aria-hidden=\"true\">[is:pr author:@me]</span>\
        <span class=\"gh-tab-name\">Mine</span> <span class=\"gh-tab-count\">2</span></button>";
    assert!(html.contains(mine), "{html}");
    assert!(
        html.contains("data-section=\"Pull &lt;requests&gt;\" aria-pressed=\"false\""),
        "{html}"
    );
    assert!(html.contains("<span class=\"gh-tab-count\">3,250</span>"), "{html}");
}

/// The break this catches: a section from a host that didn't say its search losing its tab, or the
/// renderer's mark for an empty search drawn as an empty span.
#[test]
fn a_tab_without_its_search_still_shows() {
    let html = tabs("Old host");
    let old = "aria-pressed=\"true\"><span class=\"gh-tab-mark\" aria-hidden=\"true\">[]</span>\
        <span class=\"gh-tab-name\">Old host</span>";
    assert!(html.contains(old), "{html}");
    assert_eq!(html.matches("aria-pressed=\"true\"").count(), 1, "{html}");
}
