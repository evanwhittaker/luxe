//! Changes footer — the pane's shortcuts, fitted to the width it has.

use crate::app::App;
use crate::changes::{ChangesView, PeekView};
use crate::mouse::{ChangesFooterAction, ChangesFooterAction as A, MouseTarget};
use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// One clickable footer shortcut: its chord, then what it does.
struct Shortcut {
    chord: String,
    label: &'static str,
    action: ChangesFooterAction,
}

impl Shortcut {
    /// Cells this shortcut occupies, its separating space included.
    fn width(&self) -> usize {
        self.chord.chars().count() + 1 + self.label.chars().count()
    }
}

/// Cells between two shortcuts.
const SHORTCUT_GAP: usize = 3;

/// Which shortcut yields first when the footer will not fit.
///
/// Least actionable first, and `close` is never in the list: a surface that has taken the column
/// must always say how to give it back. Dropping whole shortcuts rather than letting the row
/// truncate is the point — `Alt+W no wr` is worse than no mention of wrapping at all. The editor
/// goes before the layout keys: it has `/editor` and the focused pane's `o`, while wrapping and the
/// side-by-side switch are chords nothing else on screen names.
const SHORTCUT_SACRIFICE: [ChangesFooterAction; 7] = [
    ChangesFooterAction::FilePrev,
    ChangesFooterAction::Focus,
    ChangesFooterAction::OpenInEditor,
    ChangesFooterAction::ToggleView,
    ChangesFooterAction::ToggleWordWrap,
    ChangesFooterAction::ToggleSplit,
    ChangesFooterAction::FileNext,
];

/// Drops shortcuts by priority until the row fits `width`.
fn fit_shortcuts(shortcuts: &mut Vec<Shortcut>, width: usize) {
    let rendered = |shortcuts: &Vec<Shortcut>| {
        shortcuts.iter().map(Shortcut::width).sum::<usize>()
            + SHORTCUT_GAP * shortcuts.len().saturating_sub(1)
    };
    for action in SHORTCUT_SACRIFICE {
        if rendered(shortcuts) <= width {
            return;
        }
        shortcuts.retain(|shortcut| shortcut.action != action);
    }
    // Whatever is left over: trim from the end, so `Esc close` is the last thing standing.
    while rendered(shortcuts) > width && shortcuts.len() > 1 {
        shortcuts.remove(shortcuts.len() - 2);
    }
}

/// The keys that do something here, and only those.
///
/// Chords are read from the live keymap rather than typed in, so a remapped binding cannot leave
/// the footer advertising a key that does nothing. Stepping disappears when there is nowhere to
/// step: a control that cannot act is worse than an absent one.
fn shortcuts(app: &App, view: &ChangesView, side_by_side: bool) -> Vec<Shortcut> {
    use luxe_core::keymap::UiAction;
    let chord = |action: UiAction, fallback: &str| chord(app, action, fallback);
    if view.focused {
        return focused_shortcuts(app, view, side_by_side);
    }
    let mut shortcuts = Vec::new();
    if view.changed() {
        shortcuts.push(Shortcut {
            chord: chord(UiAction::ChangesToggleView, "alt+m"),
            label: match view.view {
                PeekView::Diff => "whole file",
                PeekView::File => "changes",
            },
            action: ChangesFooterAction::ToggleView,
        });
    }
    if view.files.len() > 1 && view.changed() {
        shortcuts.push(Shortcut {
            chord: chord(UiAction::ChangesFileNext, "alt+n"),
            label: "next file",
            action: ChangesFooterAction::FileNext,
        });
        shortcuts.push(Shortcut {
            chord: chord(UiAction::ChangesFilePrev, "alt+p"),
            label: "previous",
            action: ChangesFooterAction::FilePrev,
        });
    }
    let picture = super::picture::showing(view).is_some();
    if picture {
        shortcuts.push(Shortcut {
            chord: chord(UiAction::OpenImage, "click it"),
            label: "full size",
            action: ChangesFooterAction::Enlarge,
        });
    }
    shortcuts.extend(split_shortcut(app, view, side_by_side));
    if !picture {
        shortcuts.push(wrap_shortcut(app, view));
    }
    // Only for a file the review could actually contain. A review is captured from the project's
    // own worktree, so offering it over a file that lives somewhere else advertises a destination
    // that does not exist.
    if !luxe_core::file_diff::is_external_path(&view.path) && view.changed() {
        shortcuts.push(Shortcut {
            chord: chord(UiAction::ReviewOpen, "/diff"),
            label: "review this file",
            action: ChangesFooterAction::OpenReview,
        });
    }
    shortcuts.push(Shortcut {
        chord: chord(UiAction::ChangesOpenInEditor, "/editor"),
        // A picture goes to the desktop's image viewer, not a text editor; an SVG's source does not
        label: if picture && !view.has_source() { "open" } else { "open in editor" },
        action: ChangesFooterAction::OpenInEditor,
    });
    shortcuts.push(Shortcut {
        chord: chord(UiAction::FocusFiles, "ctrl+t"),
        label: "navigate",
        action: ChangesFooterAction::Focus,
    });
    shortcuts.push(Shortcut {
        chord: "esc".to_string(),
        label: "close",
        action: ChangesFooterAction::Close,
    });
    shortcuts
}

/// The first key bound to `action`, or `fallback` when nothing is.
fn chord(app: &App, action: luxe_core::keymap::UiAction, fallback: &str) -> String {
    app.keymap
        .chords_for(action)
        .and_then(|chords| chords.first())
        .map_or_else(|| fallback.to_string(), ToString::to_string)
}

/// The side-by-side switch, only where the pane could actually draw two columns: a control that
/// cannot act is worse than an absent one, because pressing it teaches the reader that the footer
/// is not to be trusted. Mid-switch `side_by_side` carries the last resolved layout, which is what
/// keeps this from blinking out for the frame the next diff spends loading.
fn split_shortcut(app: &App, view: &ChangesView, side_by_side: bool) -> Option<Shortcut> {
    let two_sided =
        view.two_sided.get() || view.file().is_some_and(crate::guided_review::has_two_sides);
    if view.view != PeekView::Diff || !two_sided {
        return None;
    }
    Some(Shortcut {
        chord: chord(app, luxe_core::keymap::UiAction::ReviewToggleSplit, "/diff"),
        // Name the layout it switches *to*, and say what the layout is *of*: "side by side"
        // alone reads as a property of the pane rather than of the diff in it.
        label: if side_by_side { "unified diff" } else { "side-by-side diff" },
        action: ChangesFooterAction::ToggleSplit,
    })
}

/// The wrap switch, named for what it does next.
fn wrap_shortcut(app: &App, view: &ChangesView) -> Shortcut {
    Shortcut {
        chord: chord(app, luxe_core::keymap::UiAction::ChangesToggleWordWrap, "alt+w"),
        label: if view.word_wrap { "stop wrapping" } else { "wrap lines" },
        action: ChangesFooterAction::ToggleWordWrap,
    }
}

/// The keys the pane answers to while the keyboard is in it, which are not the ones it shows
/// otherwise: a reader who just walked in needs the way around, and the way out.
///
/// The layout chords still work here (the pane passes every chord on), so they follow the
/// movement keys where the row has room, and are the first to go where it doesn't.
fn focused_shortcuts(app: &App, view: &ChangesView, side_by_side: bool) -> Vec<Shortcut> {
    let mut shortcuts = focused_keys(view);
    if view.visual || super::picture::showing(view).is_some() {
        return shortcuts;
    }
    let layout =
        split_shortcut(app, view, side_by_side).into_iter().chain([wrap_shortcut(app, view)]);
    // Ahead of `tab files` and `esc back`, which stay last
    let at = shortcuts.len().saturating_sub(2);
    shortcuts.splice(at..at, layout);
    shortcuts
}

/// The focused pane's own keys, for the view it shows. Each single key is a button that does what
/// pressing it does; a pair like `j k` is only named.
fn focused_keys(view: &ChangesView) -> Vec<Shortcut> {
    let mut shortcuts: Vec<Shortcut> = focused_key_table(view)
        .iter()
        .map(|&(chord, label, action)| Shortcut { chord: chord.to_string(), label, action })
        .collect();
    if let Some(source) = source_shortcut(view).filter(|_| !view.visual) {
        // Ahead of `tab files` and `esc back`, which stay last
        shortcuts.insert(shortcuts.len().saturating_sub(2), source);
    }
    shortcuts
}

/// The switch between an SVG's picture and its source, named for the one it goes to.
fn source_shortcut(view: &ChangesView) -> Option<Shortcut> {
    (view.has_source() && view.view == PeekView::File).then(|| Shortcut {
        chord: "s".to_string(),
        label: if view.source { "picture" } else { "source" },
        action: ChangesFooterAction::ToggleSource,
    })
}

/// A focused-footer key: its chord, its label, and what a click on it does.
type FocusedKey = (&'static str, &'static str, ChangesFooterAction);

/// The single keys and pairs [`focused_keys`] offers for the view showing.
fn focused_key_table(view: &ChangesView) -> &'static [FocusedKey] {
    let picture = super::picture::showing(view).is_some();
    match view.view {
        // An SVG has no viewer of its own to go to, so `o` takes its source to the editor
        PeekView::File if picture && view.has_source() => &SVG_KEYS,
        PeekView::File if picture => &PICTURE_KEYS,
        PeekView::File if view.visual => &VISUAL_KEYS,
        PeekView::File => &FILE_KEYS,
        PeekView::Diff => &DIFF_KEYS,
    }
}

const SVG_KEYS: [FocusedKey; 4] = [
    ("enter", "full size", A::Enlarge),
    ("o", "editor", A::OpenInEditor),
    ("tab", "files", A::Files),
    ("esc", "back", A::Escape),
];

const PICTURE_KEYS: [FocusedKey; 4] = [
    ("enter", "full size", A::Enlarge),
    ("o", "open", A::OpenInEditor),
    ("tab", "files", A::Files),
    ("esc", "back", A::Escape),
];

const VISUAL_KEYS: [FocusedKey; 5] = [
    ("VISUAL", "j k grow", A::Hint),
    ("o", "other end", A::OtherEnd),
    ("enter", "ask", A::Ask),
    ("y", "copy", A::Copy),
    ("esc", "cancel", A::Escape),
];

const FILE_KEYS: [FocusedKey; 7] = [
    ("j k", "line", A::Hint),
    ("v", "select", A::Select),
    ("enter", "ask", A::Ask),
    ("y", "copy", A::Copy),
    ("o", "editor", A::OpenInEditor),
    ("tab", "files", A::Files),
    ("esc", "back", A::Escape),
];

const DIFF_KEYS: [FocusedKey; 6] = [
    ("j k", "line", A::Hint),
    ("space", "page", A::Page),
    ("g G", "ends", A::Hint),
    ("o", "editor", A::OpenInEditor),
    ("tab", "files", A::Files),
    ("esc", "back", A::Escape),
];

/// The footer, with a click region per shortcut so the mouse can do what the keys do.
pub(super) fn render_footer(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    view: &ChangesView,
    side_by_side: bool,
) {
    let styles = &app.theme_styles;
    let mut shortcuts = shortcuts(app, view, side_by_side);
    fit_shortcuts(&mut shortcuts, usize::from(area.width));
    let mut spans = Vec::new();
    let mut regions: Vec<(u16, u16, ChangesFooterAction)> = Vec::new();
    let mut column = 0usize;
    for (index, shortcut) in shortcuts.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" · ", styles.border_muted));
            column += SHORTCUT_GAP;
        }
        let start = column;
        let hovered = app.pointer.hover == Some(MouseTarget::ChangesFooter(shortcut.action));
        let key_style =
            if hovered { styles.accent.add_modifier(Modifier::BOLD) } else { styles.accent };
        let label_style = if hovered { styles.text } else { styles.muted };
        spans.push(Span::styled(format!("{} ", shortcut.chord), key_style));
        spans.push(Span::styled(shortcut.label.to_string(), label_style));
        column += shortcut.chord.chars().count() + 1 + shortcut.label.chars().count();
        if let (Ok(x), Ok(span)) =
            (u16::try_from(start), u16::try_from(column.saturating_sub(start)))
        {
            regions.push((x, span, shortcut.action));
        }
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);

    let mut hit = app.hit.borrow_mut();
    for (x, span, action) in regions {
        let x = area.x.saturating_add(x);
        // A hint is not a button, so the pointer has nothing to light or press there
        if x >= area.right() || action == ChangesFooterAction::Hint {
            continue;
        }
        let width = span.min(area.right().saturating_sub(x));
        hit.push(Rect::new(x, area.y, width, 1), MouseTarget::ChangesFooter(action));
    }
}
