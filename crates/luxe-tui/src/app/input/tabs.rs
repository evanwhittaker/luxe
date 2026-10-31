//! Tab clicks — a click on a tab opens it the way the keyboard would.
//!
//! Each strip goes through the same path its Tab key does, so a click and a key can never disagree
//! about what opening a tab costs: the `@` picker asks for a tab's rows the first time it opens, and
//! a detail reads its conversation the first time it is looked at.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::{App, Msg};
use crate::github_detail::{CardState, DetailTab};
use crate::mouse::{OverlayList, TabStrip};

/// Opens tab `index` of `strip`, when the surface holding it is still open.
pub(in crate::app) fn open_tab(app: &mut App, strip: TabStrip, index: usize) {
    match strip {
        TabStrip::GithubSections => {
            if let Some(panel) = app.overlays.github.as_mut() {
                panel.open_section(index);
            }
        }
        TabStrip::GithubDetail => open_detail_tab(app, index),
        TabStrip::Picker(OverlayList::Mention) => super::flyout::open_mention_tab(app, index),
        TabStrip::Picker(_) => {}
    }
}

/// Opens a detail's Overview or Conversation, under `/github` or on `/pr`'s card.
fn open_detail_tab(app: &mut App, index: usize) {
    let Some(tab) = detail_tab_at(index) else {
        return;
    };
    let switch = match (app.overlays.github.as_mut(), app.overlays.pull_request_card.as_ref()) {
        (Some(panel), _) => panel.focus_detail_tab(tab),
        (None, Some(card)) => match &card.state {
            CardState::Ready(detail) => {
                detail.tab != tab && detail.merging.is_none() && detail.composing.is_none()
            }
            _ => false,
        },
        (None, None) => false,
    };
    // Two tabs, so Tab always lands on the other one
    if switch {
        app.update(Msg::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)));
    }
}

/// The detail tab drawn at `index`, left to right.
fn detail_tab_at(index: usize) -> Option<DetailTab> {
    match index {
        0 => Some(DetailTab::Overview),
        1 => Some(DetailTab::Conversation),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
