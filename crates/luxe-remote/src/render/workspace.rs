//! Workspace — the dock's plan as the app shows it, and the change marks its FILES tree shares.
//!
//! The TUI's words and marks, laid out for a touch screen: every row of the plan wraps rather than
//! being cut. A changed file is marked in the tree (`tree.rs`), which opens its diff.

use super::{escape, Renderer};
use luxe_core::workspace::{FileChangeStatus, TodoSummary, TodoSummaryItem};
use luxe_core::TodoSummaryStatus;

/// `PLAN 2/5`, the counts, and the checklist in the order it was written.
pub(super) fn plan(renderer: &dyn Renderer, todos: &TodoSummary) -> String {
    let mut out = heading("PLAN", &format!("{}/{}", todos.completed, todos.total));
    out.push_str(&format!(
        "<p class=\"dock-sub\">{} pending \u{b7} {} active \u{b7} {} done</p><ol class=\"todos\">",
        todos.pending, todos.in_progress, todos.completed
    ));
    for item in &todos.items {
        out.push_str(&todo_row(renderer, item));
    }
    out.push_str("</ol>");
    out + &more(todos.total, todos.items.len())
}

fn todo_row(renderer: &dyn Renderer, item: &TodoSummaryItem) -> String {
    let (status, words) = todo_status(item.status);
    format!(
        "<li class=\"todo todo-{status}\"><span class=\"todo-mark\" role=\"img\" \
         aria-label=\"{words}\">{}</span> <span class=\"todo-subject\">{}</span></li>",
        escape(&renderer.todo_mark(item.status)),
        escape(&item.subject)
    )
}

/// The status as the class the stylesheet colors by, and as words for a screen reader.
fn todo_status(status: TodoSummaryStatus) -> (&'static str, &'static str) {
    match status {
        TodoSummaryStatus::Pending => ("pending", "to do"),
        TodoSummaryStatus::InProgress => ("in-progress", "in progress"),
        TodoSummaryStatus::Completed => ("completed", "done"),
        TodoSummaryStatus::Blocked => ("blocked", "blocked"),
    }
}

/// The class the stylesheet colors a change by.
pub(super) fn change_status(status: FileChangeStatus) -> &'static str {
    match status {
        FileChangeStatus::Added => "added",
        FileChangeStatus::Modified => "modified",
        FileChangeStatus::Deleted => "deleted",
        FileChangeStatus::Renamed => "renamed",
        FileChangeStatus::Untracked => "untracked",
    }
}

/// ` +6 −0`, or nothing when Git couldn't count, since `+0 −0` would claim nothing changed.
pub(super) fn counts(additions: Option<u64>, deletions: Option<u64>) -> String {
    let (Some(additions), Some(deletions)) = (additions, deletions) else {
        return String::new();
    };
    format!(
        " <span class=\"add\">+{additions}</span> <span class=\"del\">\u{2212}{deletions}</span>"
    )
}

/// The TUI's heading: the word cut into a badge as the sidebar's other parts head theirs, with what
/// it sums up riding bare beside it.
fn heading(title: &str, rest: &str) -> String {
    format!("<p class=\"dock-head\"><span class=\"badge\">{title}</span> {rest}</p>")
}

/// `and 3 more`, for rows the snapshot left out.
fn more(total: usize, shown: usize) -> String {
    match total.saturating_sub(shown) {
        0 => String::new(),
        left => format!("<p class=\"dock-sub\">and {left} more</p>"),
    }
}

#[cfg(test)]
mod tests;
