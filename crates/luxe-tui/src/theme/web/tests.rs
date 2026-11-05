use super::*;

fn luxe_theme() -> HashMap<String, ColorValue> {
    luxe_core::theme::ThemeRegistry::new().resolve("luxe").expect("the luxe theme")
}

/// The break this catches: a derived color renamed, dropped or computed otherwise than the TUI
/// draws it, so the phone's mode, slabs or comet drift from the terminal's. Copied from the luxe
/// theme's real output.
#[test]
fn the_luxe_theme_derives_the_tuis_colors() {
    let css = derived_properties(&luxe_theme());
    for line in [
        "  --luxe-mode-build: #ddc084;",
        "  --luxe-mode-plan: #d2a5cd;",
        "  --luxe-slab-raised: #8c7a55;",
        "  --luxe-slab-recessed: #4f4531;",
        "  --luxe-slab-ink: #12100e;",
        "  --luxe-trail-0: #e3cb9a;",
        "  --luxe-trail-9: #2e2b27;",
        "  --luxe-trail-plan-0: #dab5d6;",
        "  --luxe-rail-9: #151311;",
    ] {
        assert!(css.lines().any(|found| found == line), "missing {line:?} in\n{css}");
    }
}

/// The break this catches: a ramp cut short, so the app's comet indexes a shade that isn't there.
#[test]
fn every_ramp_has_all_its_steps() {
    let css = derived_properties(&luxe_theme());
    for name in ["trail", "trail-plan", "rail"] {
        let prefix = format!("  --luxe-{name}-");
        let is_step = |line: &&str| {
            let rest = line.strip_prefix(&prefix);
            rest.is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
        };
        let steps = css.lines().filter(is_step).count();
        assert_eq!(steps, crate::theme::styles::ACTIVITY_TRAIL_LEN, "{name}");
    }
}

/// The break this catches: a terminal-owned color written as something CSS can't read, which
/// would break the whole rule rather than leave the app's fallback standing.
#[test]
fn a_color_the_terminal_owns_is_left_out() {
    let mut theme = luxe_theme();
    theme.insert("accent".into(), ColorValue::Ansi(3));
    let css = derived_properties(&theme);
    assert!(!css.contains("--luxe-slab-hero"), "{css}");
    assert!(css.lines().all(|line| line.ends_with(';') && line.contains(": #")), "{css}");
}
