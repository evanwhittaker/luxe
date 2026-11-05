//! The tab line across the top of `/github`: each section's mark for its search, its name and how
//! many its search matches, the open one on a slab. A tab is a button whose data names its
//! section; the app owns the tap.

use super::grouped;
use crate::render::{escape, Renderer};

/// One tab, as the app sends it from a `github_listed` section.
#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct Tab {
    /// The section's name
    pub(crate) name: String,
    /// The search it ran, which picks its mark
    #[serde(default)]
    pub(crate) query: Option<String>,
    /// How many the search matches, or how many it brought when GitHub didn't say
    pub(crate) count: u64,
}

/// Every tab, `shown` the open one's name.
pub(in crate::render) fn tab_line(renderer: &dyn Renderer, tabs: &[Tab], shown: &str) -> String {
    let buttons: String = tabs.iter().map(|tab| button(renderer, tab, tab.name == shown)).collect();
    format!("<nav class=\"gh-tabs\" aria-label=\"GitHub sections\">{buttons}</nav>")
}

fn button(renderer: &dyn Renderer, tab: &Tab, open: bool) -> String {
    let mark = renderer.github_section_mark(tab.query.as_deref().unwrap_or_default());
    let mark = match mark.is_empty() {
        true => String::new(),
        false => {
            format!("<span class=\"gh-tab-mark\" aria-hidden=\"true\">{}</span>", escape(&mark))
        }
    };
    let name = escape(&tab.name);
    format!(
        "<button type=\"button\" class=\"gh-tab\" data-section=\"{name}\" aria-pressed=\"{open}\">\
         {mark}<span class=\"gh-tab-name\">{name}</span> \
         <span class=\"gh-tab-count\">{}</span></button>",
        grouped(tab.count)
    )
}
