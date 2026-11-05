//! The TUI's renderer, handed to the bridge so the phone draws text the way the terminal does.

use luxe_core::workspace::PullRequestPresence;
use luxe_remote::{
    StatusFacts, StatusSegment, StatusSurface, StatusTone, TelemetryRow, TelemetryTone,
};
use luxe_tui::markdown::html;
use luxe_tui::statusline::plain;
use luxe_tui::statusline::plain::dock;

/// Markdown, highlighting, tool glyphs and the status line's wording from `luxe-tui`.
pub(super) struct TuiRenderer;

impl luxe_remote::Renderer for TuiRenderer {
    fn markdown(&self, text: &str, voice: luxe_remote::Voice) -> String {
        let voice = match voice {
            luxe_remote::Voice::Answer => html::Voice::Answer,
            luxe_remote::Voice::Reasoning => html::Voice::Reasoning,
        };
        html::render(text, voice)
    }

    fn tool_glyph(&self, tool: &str) -> String {
        // the app's font has the Nerd Font marks whatever this terminal can draw
        luxe_tui::widgets::tool_icon(tool).at(luxe_tui::glyphs::GlyphTier::Nerd).to_string()
    }

    fn status(&self, facts: &StatusFacts) -> Vec<StatusSegment> {
        plain::segments(&plain_facts(facts)).into_iter().map(status_segment).collect()
    }

    fn status_left(&self, facts: &StatusFacts) -> Vec<StatusSegment> {
        plain::left_segments(&plain_facts(facts)).into_iter().map(status_segment).collect()
    }

    fn theme_properties(
        &self,
        theme: &std::collections::HashMap<String, luxe_core::theme::ColorValue>,
    ) -> String {
        luxe_tui::theme::web::derived_properties(theme)
    }

    fn model_rows(&self, facts: &StatusFacts, mode: &str) -> Vec<TelemetryRow> {
        dock::model_rows(&plain_facts(facts), mode).into_iter().map(telemetry_row).collect()
    }

    fn session_rows(&self, facts: &StatusFacts) -> Vec<TelemetryRow> {
        dock::session_rows(&plain_facts(facts)).into_iter().map(telemetry_row).collect()
    }

    fn image_glyph(&self) -> String {
        luxe_tui::glyphs::Icon::Image.at(luxe_tui::glyphs::GlyphTier::Nerd).to_string()
    }

    fn code_line(&self, text: &str, language: Option<&str>) -> String {
        html::code_line(text, language)
    }

    fn code_block(&self, text: &str, language: Option<&str>) -> String {
        html::code_block(text, language)
    }

    fn todo_mark(&self, status: luxe_core::TodoSummaryStatus) -> String {
        luxe_tui::marks::todo(status, luxe_tui::glyphs::GlyphTier::Nerd).to_string()
    }

    fn change_mark(&self, status: luxe_core::workspace::FileChangeStatus) -> String {
        luxe_tui::marks::change(status).to_string()
    }

    fn file_mark(&self, name: &str) -> luxe_remote::FileMark {
        // the extension, as the TUI's tree reads it for a file's mark
        let extension = name.rsplit_once('.').map_or("", |(_, extension)| extension);
        let icon = luxe_tui::widgets::language_icon(&extension.to_ascii_lowercase());
        let glyph = icon.at(luxe_tui::glyphs::GlyphTier::Nerd).to_string();
        luxe_remote::FileMark { glyph, hue: mark_hue(luxe_tui::widgets::language_hue_role(icon)) }
    }

    fn folder_mark(&self) -> String {
        luxe_tui::glyphs::Icon::Folder.at(luxe_tui::glyphs::GlyphTier::Nerd).to_string()
    }

    fn branch_mark(&self) -> String {
        luxe_tui::glyphs::Icon::Branch.at(luxe_tui::glyphs::GlyphTier::Nerd).to_string()
    }

    fn mode_mark(&self, mode: &str) -> String {
        luxe_tui::modes::icon(mode).at(luxe_tui::glyphs::GlyphTier::Nerd).to_string()
    }

    /// The TUI's own marks: the status line's beside the same facts, `/github`'s frame and detail
    /// tabs, and the review desk's outline.
    fn chrome_mark(&self, mark: luxe_remote::ChromeMark) -> String {
        use luxe_remote::ChromeMark;
        use luxe_tui::glyphs::Icon;
        let icon = match mark {
            ChromeMark::Model => Icon::Model,
            ChromeMark::Thinking => Icon::Thinking,
            ChromeMark::Github => Icon::GitHub,
            ChromeMark::Review => Icon::Outline,
            ChromeMark::Overview => Icon::Overview,
            ChromeMark::Conversation => Icon::Conversation,
        };
        icon.at(luxe_tui::glyphs::GlyphTier::Nerd).to_string()
    }

    fn github_section_mark(&self, query: &str) -> String {
        luxe_tui::github_panel::section_icon(query)
            .at(luxe_tui::glyphs::GlyphTier::Nerd)
            .to_string()
    }

    fn github_marks(&self) -> luxe_remote::GithubMarks {
        use luxe_tui::glyphs::{GlyphTier::Nerd, Icon, Mark};
        luxe_remote::GithubMarks {
            pull_request: Icon::PullRequest.at(Nerd).to_owned(),
            issue: Icon::Issue.at(Nerd).to_owned(),
            yes: Mark::Yes.at(Nerd).to_owned(),
            no: Mark::No.at(Nerd).to_owned(),
            waiting: Mark::Waiting.at(Nerd).to_owned(),
            elision: Mark::Elision.at(Nerd).to_owned(),
        }
    }

    fn ago(&self, at: u64, now: u64) -> String {
        luxe_tui::github_detail::ago(at, now)
    }

    fn github_readable(&self, body: &str) -> String {
        luxe_tui::github_detail::readable(body)
    }

    fn home_icon(&self, side: u32, colors: luxe_remote::TileColors) -> Option<Vec<u8>> {
        luxe_tui::widgets::tile_png(side, colors.plate, colors.ink)
            .inspect_err(|error| tracing::warn!(%error, side, "could not draw the app icon"))
            .ok()
    }
}

fn mark_hue(hue: luxe_tui::widgets::LanguageHue) -> luxe_remote::MarkHue {
    use luxe_remote::MarkHue;
    use luxe_tui::widgets::LanguageHue;
    match hue {
        LanguageHue::Warning => MarkHue::Warning,
        LanguageHue::Info => MarkHue::Info,
        LanguageHue::Success => MarkHue::Success,
        LanguageHue::Error => MarkHue::Error,
        LanguageHue::ThinkingUltra => MarkHue::ThinkingUltra,
        LanguageHue::Text => MarkHue::Text,
        LanguageHue::Muted => MarkHue::Muted,
    }
}

fn plain_facts(facts: &StatusFacts) -> plain::Facts<'_> {
    plain::Facts {
        model: facts.model.as_ref(),
        thinking: facts.thinking,
        context: facts.context.map(|usage| (usage.used_tokens, usage.context_window)),
        plan: facts.plan.as_ref(),
        session: facts.session.as_ref(),
        turn: facts.turn.map(|turn| plain::TurnTokens {
            prompt: turn.prompt,
            completion: turn.completion,
            cache_read: turn.cache_read,
        }),
        pull_request: found_pull_request(facts),
        todos: facts.todos.as_ref().map(|todos| plain::TodoProgress {
            completed: todos.completed,
            total: todos.total,
            current: todos.current.as_deref(),
        }),
        changed_files: facts.changed_files.unwrap_or_default(),
    }
}

/// The pull request the bar draws: only one found, as the TUI's bar draws nothing while it looks.
fn found_pull_request(facts: &StatusFacts) -> Option<&luxe_core::github::PullRequestStatus> {
    match &facts.pull_request {
        Some(PullRequestPresence::Found(pull_request)) => Some(pull_request),
        _ => None,
    }
}

fn telemetry_row(row: dock::Row) -> TelemetryRow {
    let tone = match row.tone {
        dock::RowTone::Bar(plain::Tone::Plain) => TelemetryTone::Plain,
        dock::RowTone::Bar(plain::Tone::Warning) => TelemetryTone::Warning,
        dock::RowTone::Bar(plain::Tone::Error) => TelemetryTone::Error,
        dock::RowTone::Dim => TelemetryTone::Dim,
        dock::RowTone::Mode => TelemetryTone::Mode,
    };
    TelemetryRow { label: row.label, glyph: row.glyph.map(str::to_string), value: row.value, tone }
}

fn status_segment(segment: plain::Segment) -> StatusSegment {
    let kind = match segment.kind {
        plain::Kind::Context => "context",
        plain::Kind::Cost => "cost",
        plain::Kind::Tokens => "tokens",
        plain::Kind::Thinking => "thinking",
        plain::Kind::Model => "model",
        plain::Kind::PullRequest => "pull-request",
        plain::Kind::Todo => "todo",
        plain::Kind::Changes => "changes",
    };
    let tone = match segment.tone {
        plain::Tone::Plain => StatusTone::Plain,
        plain::Tone::Warning => StatusTone::Warning,
        plain::Tone::Error => StatusTone::Error,
    };
    let glyph = segment.glyph.map(str::to_string);
    let surface = status_surface(segment.surface);
    StatusSegment { kind, glyph, label: segment.label, tone, surface }
}

fn status_surface(surface: plain::Surface) -> StatusSurface {
    match surface {
        plain::Surface::Bare => StatusSurface::Bare,
        plain::Surface::Hero => StatusSurface::Hero,
        plain::Surface::Raised => StatusSurface::Raised,
        plain::Surface::Recessed => StatusSurface::Recessed,
    }
}

#[cfg(test)]
mod tests;
