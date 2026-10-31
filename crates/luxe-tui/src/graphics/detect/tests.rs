use super::*;
use std::collections::HashMap;

fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Result<String, std::env::VarError> {
    let map: HashMap<String, String> =
        pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect();
    move |name: &str| map.get(name).cloned().ok_or(std::env::VarError::NotPresent)
}

/// The allowlist is the whole safety of the pixel tier. WezTerm speaks the kitty protocol and
/// renders its placeholder characters as tofu, so "speaks kitty graphics" is not the question
/// being asked here.
#[test]
fn only_terminals_measured_to_draw_placeholders_get_pixels() {
    let pixels = [
        vec![("TERM", "xterm-kitty"), ("LANG", "en_US.UTF-8")],
        vec![("KITTY_WINDOW_ID", "1"), ("TERM", "xterm-256color"), ("LANG", "en_US.UTF-8")],
        vec![("TERM", "xterm-ghostty"), ("LANG", "en_US.UTF-8")],
        vec![("TERM_PROGRAM", "ghostty"), ("TERM", "xterm-256color")],
        vec![("GHOSTTY_RESOURCES_DIR", "/usr/share/ghostty"), ("TERM", "xterm-256color")],
    ];
    for env in pixels {
        let detected = ImageTier::detect(&env_of(&env));
        assert_eq!(detected.tier, ImageTier::Pixels, "{env:?}");
        assert!(!detected.reason.is_empty(), "and it can say why");
    }
}

/// Anything not on the allowlist gets words, never an imitation. A real screenshot is mostly
/// text, and text at two scanlines a cell is a smear that reads as a broken image — which is a
/// worse thing to ship than a sentence saying what happened.
#[test]
fn every_other_terminal_gets_words_rather_than_an_imitation() {
    for env in [
        vec![("TERM", "xterm-256color"), ("WEZTERM_PANE", "0")],
        vec![("TERM", "alacritty")],
        vec![("TERM", "foot")],
        vec![("TERM_PROGRAM", "iterm.app"), ("TERM", "xterm-256color")],
        vec![("TERM", "dumb")],
        vec![],
    ] {
        assert_eq!(ImageTier::detect(&env_of(&env)).tier, ImageTier::Text, "{env:?}");
    }
}

/// A multiplexer forwards graphics it does not model and then repaints the wrong pane with
/// them — whatever terminal is underneath.
#[test]
fn a_multiplexer_gets_words_whatever_is_underneath() {
    for env in [
        vec![("TMUX", "/tmp/tmux-1000/default,1,0"), ("TERM", "xterm-kitty")],
        vec![("TERM", "tmux-256color"), ("KITTY_WINDOW_ID", "1")],
        vec![("TERM", "screen.xterm-256color")],
    ] {
        let detected = ImageTier::detect(&env_of(&env));
        assert_eq!(detected.tier, ImageTier::Text, "{env:?}");
        assert!(detected.reason.contains("multiplexer"), "{}", detected.reason);
    }
}

/// An explicit preference is an instruction, not a hint — including the two nothing is ever
/// resolved to on its own: the low-fidelity preview, and off.
#[test]
fn an_explicit_preference_overrides_detection() {
    assert_eq!(ImageTier::resolve(ImageStyle::Pixels), ImageTier::Pixels);
    assert_eq!(ImageTier::resolve(ImageStyle::Blocks), ImageTier::Blocks);
    assert_eq!(ImageTier::resolve(ImageStyle::Off), ImageTier::Text);
    for preference in [ImageStyle::Pixels, ImageStyle::Blocks, ImageStyle::Off] {
        assert!(ImageTier::resolve_detail(preference).reason.contains("config"));
    }
    assert!(!ImageTier::Text.draws_pixels());
    assert!(ImageTier::Blocks.draws_pixels() && ImageTier::Pixels.draws_pixels());
}

fn answered(sixel: bool) -> Option<TerminalReport> {
    Some(TerminalReport { sixel, cell: None })
}

/// Windows Terminal draws Sixel from 1.22, and says so when asked. Windows sets no `TERM`, which
/// is why this has to hold with none.
///
/// The break this catches: the "no TERM means not a terminal" rule running first, which leaves
/// every native Windows session with words.
#[test]
fn windows_terminal_that_reports_sixel_gets_sixel_with_no_term() {
    let env = env_of(&[("WT_SESSION", "9a81020f-8bd1-4f69-b682-647816a7824d")]);
    let detected = ImageTier::detect_with(&env, answered(true));
    assert_eq!(detected.tier, ImageTier::Sixel);
    assert!(detected.reason.contains("Windows Terminal"));
}

/// An older Windows Terminal, or one that never answered, gets words rather than a blank
/// rectangle where a picture should be.
#[test]
fn windows_terminal_without_sixel_gets_words() {
    let env = env_of(&[("WT_SESSION", "x"), ("TERM", "xterm-256color")]);
    for report in [answered(false), None] {
        let detected = ImageTier::detect_with(&env, report);
        assert_eq!(detected.tier, ImageTier::Text, "{report:?}");
        assert!(detected.reason.contains("Windows Terminal"), "{}", detected.reason);
    }
}

/// A multiplexer inside Windows Terminal still cannot keep track of the picture.
#[test]
fn a_multiplexer_inside_windows_terminal_gets_words() {
    let env = env_of(&[("WT_SESSION", "x"), ("TMUX", "/tmp/tmux-1000/default,1,0")]);
    let detected = ImageTier::detect_with(&env, answered(true));
    assert_eq!(detected.tier, ImageTier::Text);
    assert!(detected.reason.contains("multiplexer"));
}

/// What the terminal answered only matters inside Windows Terminal: a report from a terminal
/// luxe has no rule for is not a reason to draw into it.
#[test]
fn a_report_alone_does_not_make_another_terminal_a_sixel_terminal() {
    let env = env_of(&[("TERM", "xterm-256color")]);
    assert_eq!(ImageTier::detect_with(&env, answered(true)).tier, ImageTier::Text);
}

/// `sixel` in config is an instruction, as `pixels` is, and it counts as drawing a picture.
#[test]
fn the_sixel_preference_is_honored() {
    assert_eq!(ImageTier::resolve(ImageStyle::Sixel), ImageTier::Sixel);
    assert!(ImageTier::resolve_detail(ImageStyle::Sixel).reason.contains("config"));
    assert!(ImageTier::Sixel.draws_pixels());
}

/// A kitty or Ghostty run from inside Windows Terminal (under WSL, say) inherits `WT_SESSION` and
/// is still the terminal it is, so it keeps its own protocol.
#[test]
fn a_kitty_family_terminal_keeps_its_own_protocol_even_with_a_windows_terminal_session() {
    let env = env_of(&[("WT_SESSION", "x"), ("KITTY_WINDOW_ID", "1"), ("TERM", "xterm-kitty")]);
    assert_eq!(ImageTier::detect_with(&env, answered(true)).tier, ImageTier::Pixels);
}
