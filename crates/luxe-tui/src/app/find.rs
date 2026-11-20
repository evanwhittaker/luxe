//! Find glue — opens the overlay, sends the searches its query needs, and does what a row asks.
//!
//! [`crate::find::FindState`] decides what a key means. This applies it: a text search goes to the
//! engine as `Action::SearchProject` and comes back as an event only the search in flight can
//! fill, file names rank through the `@` picker's provider, and a chosen row opens in the preview,
//! lands in the draft as a mention, or is a message the conversation scrolls to with its search
//! open.

use crossterm::event::KeyEvent;
use luxe_core::engine::Action;
use luxe_core::event::Event;
use luxe_core::project_files::FileReadRequestId;
use luxe_core::project_search::SearchRequestId;

use super::{App, Msg};
use crate::changes::FoundQuery;
use crate::find::{FindOutcome, FindState, FindTab, FindTarget};
use crate::ref_provider::RefProvider;

/// Opens Find on the tab it last showed, with that tab's rows for an empty query.
pub(crate) fn open(app: &mut App) {
    app.overlays.find = Some(FindState::open(app.overlays.find_tab));
    refresh(app);
}

/// Answers a key while Find is open.
pub(super) fn take_key(app: &mut App, key: &KeyEvent) {
    let Some(find) = app.overlays.find.as_mut() else { return };
    let outcome = find.on_key(key);
    apply(app, outcome);
}

/// Types a paste into Find's query, when Find is open to take it.
pub(super) fn take_paste(app: &mut App, text: &str) -> bool {
    let Some(find) = app.overlays.find.as_mut() else { return false };
    let outcome = find.paste(text);
    apply(app, outcome);
    true
}

/// Shows the tab a click landed on.
pub(crate) fn open_tab(app: &mut App, index: usize) {
    let Some(find) = app.overlays.find.as_mut() else { return };
    let outcome = find.show_tab(index);
    apply(app, outcome);
}

/// Takes a search's answer or failure, when it's for the search Find is waiting on.
pub(super) fn take_event(app: &mut App, event: &Event) -> bool {
    let Some(find) = app.overlays.find.as_mut() else { return false };
    let took = match event {
        Event::ProjectSearched(answer) => find.accept(answer),
        Event::ProjectSearchFailed(failed) => find.fail(failed),
        _ => false,
    };
    if took {
        request_preview(app);
    }
    took
}

/// Takes a file read's answer, when it's one Find sent for its preview pane; any other read is
/// the preview pane's own, and goes on to it.
pub(super) fn take_file(app: &mut App, event: &Event) -> bool {
    let Some(find) = app.overlays.find.as_mut() else { return false };
    let previews = find.previews_mut();
    match event {
        Event::ProjectFileRead { request_id, file } => previews.accept(request_id, file),
        Event::ProjectFileReadFailed { request_id, error, .. } => previews.fail(request_id, error),
        _ => false,
    }
}

/// Asks for the file the row under the cursor shows, when the pane hasn't got it or asked yet.
fn request_preview(app: &mut App) {
    let Some(path) = app.overlays.find.as_ref().and_then(FindState::preview_wanted) else { return };
    let request_id = FileReadRequestId::new();
    let action = Action::ReadProjectFile { request_id: request_id.clone(), path: path.clone() };
    // Unsent, nothing is recorded, so the next move asks again
    if !app.send_action(action, "read a file to preview") {
        return;
    }
    if let Some(find) = app.overlays.find.as_mut() {
        find.previews_mut().reading(request_id, &path);
    }
}

fn apply(app: &mut App, outcome: FindOutcome) {
    match outcome {
        FindOutcome::Consumed => request_preview(app),
        FindOutcome::Refresh => {
            refresh(app);
            request_preview(app);
        }
        FindOutcome::Close => close(app),
        FindOutcome::Open(target) => open_row(app, target),
        FindOutcome::Mention(target) => mention(app, &target),
        FindOutcome::ShowMessage { item, query } => show_message(app, item, query),
    }
}

/// Closes Find and mentions `target` in the draft, out in the conversation where the draft is.
fn mention(app: &mut App, target: &FindTarget) {
    close(app);
    leave_review(app);
    super::messages::changes::mention_in_draft(app, &target.mention());
}

/// Closes Find and goes to message `item` in the conversation, its search open on `query`.
fn show_message(app: &mut App, item: super::ConversationItemId, query: String) {
    close(app);
    leave_review(app);
    app.search_state.enter_at(query, item);
}

/// Closes Find, remembering its tab for next time.
fn close(app: &mut App) {
    if let Some(find) = app.overlays.find.take() {
        app.overlays.find_tab = find.tab();
    }
}

/// Closes Find and opens the chosen row, with the rows' query to light when there is one.
fn open_row(app: &mut App, target: FindTarget) {
    let found = app.overlays.find.as_ref().and_then(|find| {
        find.rows_query().and_then(|(query, options)| FoundQuery::new(query, options))
    });
    close(app);
    if went_to_in_review(app, &target) {
        return;
    }
    leave_review(app);
    open_target(app, target, found);
}

/// In a review, goes to the target's line when the review shows it, and says whether it did.
fn went_to_in_review(app: &mut App, target: &FindTarget) -> bool {
    if app.current_view != super::ViewMode::GuidedReview {
        return false;
    }
    let (Some(review), Some(line)) = (app.guided_review.as_deref(), target.line) else {
        return false;
    };
    let path = std::path::Path::new(&target.path);
    app.guided_review_workspace.jump_to_line(path, line, review)
}

/// Steps out of a review to the conversation, where the File view and the draft are; the review
/// keeps its place for coming back.
fn leave_review(app: &mut App) {
    if app.current_view == super::ViewMode::GuidedReview {
        super::navigation::exit_guided_review(app);
    }
}

/// Opens a row in the preview, lighting `found` in it when the row came from a text search.
fn open_target(app: &mut App, target: FindTarget, found: Option<FoundQuery>) {
    match target.line {
        Some(line) => super::messages::changes::open_at_line(app, target.path, line),
        None => app.update(Msg::PreviewProjectFile(target.path)),
    }
    if let Some(view) = app.changes.as_mut() {
        view.found = found;
    }
}

/// Brings the showing tab's rows up to date with the query.
fn refresh(app: &mut App) {
    let Some(find) = app.overlays.find.as_mut() else { return };
    match find.tab() {
        FindTab::Text => send_search(app),
        FindTab::Files => {
            let root = app.project_root.clone().unwrap_or_default();
            find.set_files(app.file_provider.candidates(find.query(), &root));
        }
        FindTab::Conversation => {
            find.set_messages(crate::find::messages::message_hits(&app.convo, find.query()));
        }
    }
}

/// Sends the text search the query needs, if it needs one.
///
/// Every keystroke can send one: starting a search in the engine stops the one before it, and
/// the answer that would have come back for it is dropped here anyway.
fn send_search(app: &mut App) {
    let Some(asked) = app.overlays.find.as_mut().and_then(FindState::next_search) else { return };
    let request_id = SearchRequestId::new();
    let (query, options) = asked.clone();
    let action = Action::SearchProject { request_id: request_id.clone(), query, options };
    let sent = app.send_action(action, "search the project");
    let Some(find) = app.overlays.find.as_mut() else { return };
    match sent {
        true => find.searching(request_id, asked),
        false => find.unsent("luxe is not accepting requests"),
    }
}

#[cfg(test)]
mod tests;
