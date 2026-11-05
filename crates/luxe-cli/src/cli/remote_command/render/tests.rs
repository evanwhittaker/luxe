use luxe_remote::{Renderer, Voice};

use super::TuiRenderer;

/// The break this catches: the voices crossed, so reasoning loses its line breaks and an
/// answer keeps the model's.
#[test]
fn each_voice_reaches_the_tui_as_itself() {
    assert_eq!(TuiRenderer.markdown("a\nb", Voice::Reasoning), "<p>a<br>b</p>");
    assert_eq!(TuiRenderer.markdown("a\nb", Voice::Answer), "<p>a b</p>");
}

/// The break this catches: the glyph taken at this terminal's tier, which is ASCII or none
/// under a plain terminal, though the app's font always has the marks.
#[test]
fn tool_glyphs_are_the_nerd_font_marks() {
    let nerd = luxe_tui::widgets::tool_icon("bash").at(luxe_tui::glyphs::GlyphTier::Nerd);
    luxe_tui::glyphs::set_tier(luxe_tui::glyphs::GlyphTier::Ascii);
    let glyph = TuiRenderer.tool_glyph("bash");
    let image = TuiRenderer.image_glyph();
    let done = TuiRenderer.todo_mark(luxe_core::TodoSummaryStatus::Completed);
    luxe_tui::glyphs::set_tier(luxe_tui::glyphs::GlyphTier::Nerd);
    assert_eq!(glyph, nerd);
    let nerd_image = luxe_tui::glyphs::Icon::Image.at(luxe_tui::glyphs::GlyphTier::Nerd);
    assert_eq!(image, nerd_image, "an attached image's mark too");
    let nerd_done = luxe_tui::glyphs::Mark::Yes.at(luxe_tui::glyphs::GlyphTier::Nerd);
    assert_eq!(done, nerd_done, "a todo's mark too");
}

/// The break this catches: a diff row's code left plain, or a changed file's letter crossed,
/// so the phone's dock and diffs stop matching the TUI's.
#[test]
fn diffs_and_changes_reach_the_tui() {
    let code = TuiRenderer.code_line("fn a() {}", Some("rs"));
    assert!(code.starts_with("<span class=\"t-accent-secondary\">fn</span>"), "{code}");
    let modified = luxe_core::workspace::FileChangeStatus::Modified;
    assert_eq!(TuiRenderer.change_mark(modified), "M");
}

/// The break this catches: a tool card's code left plain, as the bridge's own fallback draws
/// it, or its lines run together.
#[test]
fn a_cards_code_reaches_the_tui_highlighter() {
    let code = TuiRenderer.code_block("fn a() {}\nfn b() {}", Some("rs"));
    assert_eq!(code.matches("<span class=\"t-accent-secondary\">fn</span>").count(), 2, "{code}");
    assert_eq!(code.matches('\n').count(), 1, "{code}");
}

/// The break this catches: a file's mark read from the whole name or a cased extension, a
/// language's hue crossed with another's, or the folder's mark taken at this terminal's tier,
/// so the phone's tree stops matching the TUI's.
#[test]
fn tree_marks_are_the_tui_language_marks() {
    use luxe_remote::MarkHue;
    use luxe_tui::glyphs::{GlyphTier, Icon};
    let nerd = |icon: Icon| icon.at(GlyphTier::Nerd).to_string();
    luxe_tui::glyphs::set_tier(GlyphTier::Ascii);
    let rust = TuiRenderer.file_mark("main.RS");
    let toml = TuiRenderer.file_mark("Cargo.toml");
    let folder = TuiRenderer.folder_mark();
    luxe_tui::glyphs::set_tier(GlyphTier::Nerd);
    assert_eq!(rust, luxe_remote::FileMark { glyph: nerd(Icon::LangRust), hue: MarkHue::Warning });
    assert_eq!(toml, luxe_remote::FileMark { glyph: nerd(Icon::LangToml), hue: MarkHue::Error });
    assert_eq!(TuiRenderer.file_mark("Makefile").hue, MarkHue::Muted);
    assert_eq!(folder, nerd(Icon::Folder));
}

/// The break this catches: the project panel's branch drawn with another mark than the TUI's
/// project section, or at this terminal's tier rather than the phone's.
#[test]
fn the_branch_mark_is_the_tui_branch_mark() {
    use luxe_tui::glyphs::{GlyphTier, Icon};
    luxe_tui::glyphs::set_tier(GlyphTier::Ascii);
    let branch = TuiRenderer.branch_mark();
    luxe_tui::glyphs::set_tier(GlyphTier::Nerd);
    assert_eq!(branch, Icon::Branch.at(GlyphTier::Nerd));
}

/// The break this catches: the composer's model and thinking buttons drawn with other marks than
/// the status line beside them draws for the same facts, or the two crossed.
#[test]
fn the_composer_buttons_wear_the_status_lines_marks() {
    use luxe_remote::ChromeMark;
    let facts: luxe_remote::StatusFacts = serde_json::from_value(serde_json::json!({
        "model": {"provider": "anthropic", "model": "claude-x"},
        "thinking": "high",
    }))
    .expect("the app's status facts parse");
    let glyph = |kind| {
        let segments = TuiRenderer.status(&facts);
        segments.into_iter().find(|s| s.kind == kind).and_then(|s| s.glyph)
    };
    assert_eq!(Some(TuiRenderer.chrome_mark(ChromeMark::Model)), glyph("model"));
    assert_eq!(Some(TuiRenderer.chrome_mark(ChromeMark::Thinking)), glyph("thinking"));
}

/// The break this catches: the GitHub and Review views titled, or their tabs marked, with other
/// marks than the TUI's `/github` and desk draw, or at this terminal's tier.
#[test]
fn the_views_wear_the_tuis_marks() {
    use luxe_remote::ChromeMark;
    use luxe_tui::glyphs::{GlyphTier::Nerd, Icon};
    let marks =
        [ChromeMark::Github, ChromeMark::Review, ChromeMark::Overview, ChromeMark::Conversation];
    let drawn: Vec<String> = marks.into_iter().map(|mark| TuiRenderer.chrome_mark(mark)).collect();
    let icons = [Icon::GitHub, Icon::Outline, Icon::Overview, Icon::Conversation];
    assert_eq!(drawn, icons.map(|icon| icon.at(Nerd).to_string()));
    let tab = |query| TuiRenderer.github_section_mark(query);
    assert_eq!(tab("is:open is:pr author:@me repo:{repo}"), Icon::Mine.at(Nerd));
    assert_eq!(tab("is:open is:issue assignee:@me"), Icon::Issue.at(Nerd));
}

/// The break this catches: a GitHub mark taken at this terminal's tier or crossed with
/// another, ages in other words than the TUI's, or a body's HTML left as tags.
#[test]
fn github_reaches_the_tui_marks_and_words() {
    use luxe_tui::glyphs::{GlyphTier, Icon, Mark};
    luxe_tui::glyphs::set_tier(GlyphTier::Ascii);
    let marks = TuiRenderer.github_marks();
    luxe_tui::glyphs::set_tier(GlyphTier::Nerd);
    assert_eq!(marks.pull_request, Icon::PullRequest.at(GlyphTier::Nerd));
    assert_eq!(marks.issue, Icon::Issue.at(GlyphTier::Nerd));
    assert_eq!(marks.yes, Mark::Yes.at(GlyphTier::Nerd));
    assert_eq!(marks.no, Mark::No.at(GlyphTier::Nerd));
    assert_eq!(marks.waiting, Mark::Waiting.at(GlyphTier::Nerd));
    assert_eq!(marks.elision, Mark::Elision.at(GlyphTier::Nerd));
    assert_eq!(TuiRenderer.ago(1_000, 1_000 + 3 * 3600), "3h ago");
    let body = TuiRenderer.github_readable("<details><summary>Notes</summary>x</details>");
    assert!(!body.contains('<'), "{body}");
}

/// The break this catches: the bridge's default (no icon) left in place, so the home screen
/// gets a screenshot of the page instead of the mark, or the size asked for not passed on.
#[test]
fn the_icon_is_the_tui_tile_at_the_size_asked_for() {
    let colors = luxe_remote::TileColors { plate: [0; 3], ink: [0xff; 3] };
    let png = TuiRenderer.home_icon(180, colors).expect("an icon");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"), "a PNG");
    // the header chunk's width and height, big-endian, after the signature and chunk head
    assert_eq!(png[16..24], [0, 0, 0, 180, 0, 0, 0, 180]);
}

/// The break this catches: the sidebar's sections left to the bridge's default, which draws
/// neither, or a tone crossed on the way, so a full window reads as a roomy one or no model
/// goes unwarned.
#[test]
fn the_sidebar_rows_reach_the_tui_and_keep_their_tones() {
    use luxe_remote::TelemetryTone::{Error, Mode, Warning};
    let facts: luxe_remote::StatusFacts = serde_json::from_value(serde_json::json!({
        "context": {"used_tokens": 950, "context_window": 1000},
    }))
    .expect("the app's status facts parse");
    let tones = |rows: Vec<luxe_remote::TelemetryRow>| {
        rows.into_iter().map(|row| (row.label, row.tone)).collect::<Vec<_>>()
    };
    let model = tones(TuiRenderer.model_rows(&facts, "plan"));
    assert_eq!(model[1..], [("model", Warning), ("mode", Mode)]);
    assert_eq!(tones(TuiRenderer.session_rows(&facts)), [("context", Error)]);
}

/// The break this catches: facts dropped or crossed on the way to the TUI, so the phone's
/// status line misses the context reading, or colors a full window as a roomy one.
#[test]
fn status_facts_reach_the_tui_and_come_back_as_its_segments() {
    let facts: luxe_remote::StatusFacts = serde_json::from_value(serde_json::json!({
        "model": {"provider": "anthropic", "model": "claude-x"},
        "thinking": "medium",
        "context": {"used_tokens": 950, "context_window": 1000},
    }))
    .expect("the app's status facts parse");
    let segments = TuiRenderer.status(&facts);
    let words: Vec<_> = segments.iter().map(|s| (s.kind, s.label.as_str(), s.tone)).collect();
    assert_eq!(
        words,
        [
            ("context", "95.0%/1.0k", luxe_remote::StatusTone::Error),
            ("thinking", "med", luxe_remote::StatusTone::Plain),
            ("model", "claude-x", luxe_remote::StatusTone::Plain),
        ]
    );
}
