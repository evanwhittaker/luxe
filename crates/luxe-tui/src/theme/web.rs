//! Web — the colors the TUI derives from a theme, as CSS custom properties for the phone app.
//!
//! A theme's own tokens reach the app as they are (`luxe_core::theme::css`). What the TUI works out
//! from them, it works out here too, so the app draws the same colors rather than guessing at them
//! with `color-mix`: the mode's hue, which plan picks from whichever token stands apart from the
//! border; the status line's slab ramp; and the activity bar's comets and the rail under them.

use super::styles::ThemeStyles;
use luxe_core::theme::ColorValue;
use ratatui::style::{Color, Style};
use std::collections::HashMap;
use std::fmt::Write as _;

/// The derived colors as `--luxe-*` declarations, one a line, for a `:root` rule. A color the
/// terminal owns (an ANSI index, the default) has no hex to give, so it's left out and the app's
/// fallback stands.
pub fn derived_properties(resolved: &HashMap<String, ColorValue>) -> String {
    let styles = ThemeStyles::from_resolved(resolved, "");
    let mut out = String::new();
    let mut put = |name: &str, color: Option<Color>| {
        if let Some(hex) = color.and_then(hex) {
            let _ = writeln!(out, "  --luxe-{name}: {hex};");
        }
    };
    put("mode-build", styles.border_accent.fg);
    put("mode-plan", styles.mode_plan.fg);
    put("slab-hero", Some(styles.slab.hero));
    put("slab-raised", Some(styles.slab.receded[0]));
    put("slab-recessed", Some(styles.slab.receded[1]));
    put("slab-ink", Some(styles.slab.ink));
    put_ramp(&mut put, "trail", &styles.activity_trail);
    put_ramp(&mut put, "trail-plan", &styles.activity_trail_plan);
    put_ramp(&mut put, "rail", &styles.activity_rail);
    out
}

/// A ramp as `--luxe-{name}-0` (its near end) onward.
fn put_ramp(put: &mut impl FnMut(&str, Option<Color>), name: &str, ramp: &[Style]) {
    for (step, style) in ramp.iter().enumerate() {
        put(&format!("{name}-{step}"), style.fg);
    }
}

fn hex(color: Color) -> Option<String> {
    match color {
        Color::Rgb(red, green, blue) => Some(format!("#{red:02x}{green:02x}{blue:02x}")),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
