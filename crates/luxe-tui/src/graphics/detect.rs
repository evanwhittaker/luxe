//! Image tier detection — which fidelity the terminal can draw at, and what told us.

use super::probe::{self, TerminalReport};
use luxe_core::config::ImageStyle;

/// An environment lookup, injected so the rules are testable.
pub(super) type Env<'a> = &'a dyn Fn(&str) -> Result<String, std::env::VarError>;

/// Whether luxe is running inside tmux or screen.
///
/// A multiplexer forwards graphics it does not model and then repaints the wrong pane with them.
pub(super) fn behind_multiplexer(env: Env) -> bool {
    let term = env("TERM").unwrap_or_default().to_ascii_lowercase();
    env("TMUX").is_ok_and(|tmux| !tmux.is_empty())
        || term.starts_with("tmux")
        || term.starts_with("screen")
}

/// How much image fidelity the terminal can be asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageTier {
    /// Kitty graphics protocol with Unicode placeholders.
    Pixels,
    /// Sixel graphics, drawn over cells left blank for them.
    Sixel,
    /// Half-block cells.
    #[default]
    Blocks,
    /// A text line naming the image.
    Text,
}

/// A resolved tier and the reason it was chosen, which is the whole content of `/images`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Detection {
    pub tier: ImageTier,
    /// Why, in the words a person would use — never "detection failed".
    pub reason: &'static str,
}

impl ImageTier {
    /// Resolves the reader's preference against what the terminal can actually do.
    pub fn resolve(preference: ImageStyle) -> Self {
        Self::resolve_detail(preference).tier
    }

    /// [`Self::resolve`], keeping the reason so a status view can give one.
    pub fn resolve_detail(preference: ImageStyle) -> Detection {
        Self::resolve_detail_in(preference, &|name| std::env::var(name), probe::recorded())
    }

    /// [`Self::resolve`], against an environment lookup rather than the process's own.
    #[cfg(any(test, feature = "fixture"))]
    pub(crate) fn resolve_in(preference: ImageStyle, env: Env) -> Self {
        Self::resolve_detail_in(preference, env, None).tier
    }

    fn resolve_detail_in(
        preference: ImageStyle,
        env: Env,
        report: Option<TerminalReport>,
    ) -> Detection {
        match preference {
            ImageStyle::Pixels => {
                Detection { tier: Self::Pixels, reason: "set to `pixels` in config" }
            }
            ImageStyle::Sixel => {
                Detection { tier: Self::Sixel, reason: "set to `sixel` in config" }
            }
            ImageStyle::Blocks => {
                Detection { tier: Self::Blocks, reason: "set to `blocks` in config" }
            }
            ImageStyle::Off => Detection { tier: Self::Text, reason: "turned off in config" },
            ImageStyle::Auto | ImageStyle::Unknown => Self::detect_with(env, report),
        }
    }

    /// `auto`, against an environment lookup — a parameter so the branches are testable.
    ///
    /// Env only, and never a query the terminal has to answer: a probe that expects a reply has to
    /// read stdin, which the event loop owns, and one that times out has already cost the startup
    /// it was meant to protect.
    ///
    /// The answer is only ever pixels or words. A low-fidelity imitation is not a courtesy — a real
    /// screenshot is mostly text, and two scanlines per cell turn text into a smear that reads as a
    /// broken image rather than as a considered fallback. `blocks` exists for anyone who wants it
    /// (a multiplexer, mostly) and is never chosen on someone's behalf.
    #[cfg(test)]
    fn detect(env: Env) -> Detection {
        Self::detect_with(env, None)
    }

    /// [`Self::detect`], given what the terminal answered when it was asked, if it was.
    fn detect_with(env: Env, report: Option<TerminalReport>) -> Detection {
        let var = |name: &str| env(name).unwrap_or_default().to_ascii_lowercase();
        let term = var("TERM");
        let words = |reason| Detection { tier: Self::Text, reason };

        if term == "dumb" {
            return words("this is not an interactive terminal");
        }

        if behind_multiplexer(env) {
            return words("a multiplexer cannot keep track of an image it does not model");
        }

        // Positive allowlist, not a guess. Speaking the kitty protocol is not the same as
        // supporting its Unicode placeholders: WezTerm speaks it and draws the placeholder
        // characters as tofu. `pixels` is the lever for anyone whose terminal has since learned to.
        if let Some(found) = Self::kitty_family(&var, &term) {
            return found;
        }
        // Windows sets no TERM, so this has to come before the check that treats that as "not a
        // terminal". After the kitty family, because a terminal run under WSL inherits
        // `WT_SESSION` and is still the terminal it is.
        if !var("WT_SESSION").is_empty() {
            return Self::windows_terminal(report);
        }
        if term.is_empty() {
            return words("this is not an interactive terminal");
        }
        words("this terminal is not known to draw images inline")
    }

    /// The terminals measured to draw the kitty protocol's Unicode placeholders.
    fn kitty_family(var: &dyn Fn(&str) -> String, term: &str) -> Option<Detection> {
        let kitty = !var("KITTY_WINDOW_ID").is_empty() || term.contains("kitty");
        let ghostty = var("TERM_PROGRAM") == "ghostty"
            || term.contains("ghostty")
            || !var("GHOSTTY_RESOURCES_DIR").is_empty();
        if kitty {
            return Some(Detection { tier: Self::Pixels, reason: "kitty draws images inline" });
        }
        ghostty.then_some(Detection { tier: Self::Pixels, reason: "Ghostty draws images inline" })
    }

    /// Windows Terminal draws Sixel from 1.22 on, and says so when asked; one that is older, or
    /// that never answered, gets words rather than a blank rectangle.
    fn windows_terminal(report: Option<TerminalReport>) -> Detection {
        match report {
            Some(report) if report.sixel => {
                Detection { tier: Self::Sixel, reason: "Windows Terminal draws images with Sixel" }
            }
            Some(_) => Detection {
                tier: Self::Text,
                reason: "this Windows Terminal does not draw Sixel images (1.22 added them)",
            },
            None => Detection {
                tier: Self::Text,
                reason: "Windows Terminal did not say whether it draws Sixel images",
            },
        }
    }

    /// Whether this tier draws an actual picture.
    pub fn draws_pixels(&self) -> bool {
        !matches!(self, Self::Text)
    }
}

#[cfg(test)]
mod tests;
