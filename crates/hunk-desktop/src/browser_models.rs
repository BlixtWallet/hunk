use std::collections::HashMap;

use hunk_browser::BrowserTabSummary;
use qtbridge::{QListModel, QListModelBase, QModelItem, qobject};

const BROWSER_TAB_TITLE_MAX_BYTES: usize = 240;

#[derive(Clone, Debug, Default, Eq, PartialEq, QModelItem)]
pub struct BrowserTabItem {
    pub tab_id: String,
    pub title: String,
    pub loading: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrowserTabProjectionSource {
    tab_id: String,
    url: Option<String>,
    title: Option<String>,
    loading: bool,
}

impl BrowserTabProjectionSource {
    fn matches(&self, tab: &BrowserTabSummary) -> bool {
        self.tab_id == tab.tab_id.as_str()
            && self.url == tab.url
            && self.title == tab.title
            && self.loading == tab.loading
    }
}

impl From<&BrowserTabSummary> for BrowserTabProjectionSource {
    fn from(tab: &BrowserTabSummary) -> Self {
        Self {
            tab_id: tab.tab_id.as_str().to_owned(),
            url: tab.url.clone(),
            title: tab.title.clone(),
            loading: tab.loading,
        }
    }
}

impl From<&BrowserTabSummary> for BrowserTabItem {
    fn from(tab: &BrowserTabSummary) -> Self {
        let url = tab.url.clone().unwrap_or_default();
        let title = tab
            .title
            .clone()
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| browser_tab_fallback_title(url.as_str()));
        Self {
            tab_id: tab.tab_id.as_str().to_owned(),
            title: bounded_browser_tab_title(title.as_str()),
            loading: tab.loading,
        }
    }
}

#[qobject(Base = QListModel)]
mod browser_tab_model {
    use qtbridge::QObjectHolder;

    use super::{BrowserTabItem, QListModel, QListModelBase};

    #[derive(Default)]
    pub struct BrowserTabListModel {
        items: Vec<BrowserTabItem>,
        replacement: Option<Vec<BrowserTabItem>>,
        deferred_items: Option<Vec<BrowserTabItem>>,
        deferred_update_scheduled: bool,
    }

    impl BrowserTabListModel {
        pub fn replace_or_patch(&mut self, items: Vec<BrowserTabItem>) {
            if self.items.len() != items.len() {
                self.replacement = Some(items);
                self.reset();
                return;
            }

            for (index, item) in items.into_iter().enumerate() {
                if self.items[index] != item {
                    self.set(index, item);
                }
            }
        }

        pub fn defer_replace_or_patch(&mut self, items: Vec<BrowserTabItem>) {
            let current = self.deferred_items.as_ref().unwrap_or(&self.items);
            if current == &items {
                return;
            }
            self.deferred_items = Some(items);
            if self.deferred_update_scheduled {
                return;
            }
            self.deferred_update_scheduled = true;
            if !self
                .get_qml_method_invoker()
                .invoke_method("apply_deferred_items")
            {
                self.deferred_update_scheduled = false;
            }
        }

        #[qslot]
        fn apply_deferred_items(&mut self) {
            self.deferred_update_scheduled = false;
            let Some(items) = self.deferred_items.take() else {
                return;
            };
            self.replace_or_patch(items);
        }

        pub fn items(&self) -> &[BrowserTabItem] {
            self.deferred_items
                .as_deref()
                .unwrap_or(self.items.as_slice())
        }
    }

    impl QListModel for BrowserTabListModel {
        type Item = BrowserTabItem;

        fn len(&self) -> usize {
            self.items.len()
        }

        fn get(&self, index: usize) -> Option<&Self::Item> {
            self.items.get(index)
        }

        fn set_unnotified(&mut self, index: usize, value: Self::Item) -> bool {
            let Some(item) = self.items.get_mut(index) else {
                return false;
            };
            *item = value;
            true
        }

        fn reset_unnotified(&mut self) {
            self.items = self.replacement.take().unwrap_or_default();
        }
    }
}

pub use browser_tab_model::BrowserTabListModel;

pub fn project_browser_tabs(tabs: &[BrowserTabSummary]) -> Vec<BrowserTabItem> {
    tabs.iter().map(BrowserTabItem::from).collect()
}

pub fn browser_tab_projection_changed(
    sources: &[BrowserTabProjectionSource],
    tabs: &[BrowserTabSummary],
) -> bool {
    sources.len() != tabs.len()
        || sources
            .iter()
            .zip(tabs)
            .any(|(source, tab)| !source.matches(tab))
}

pub fn project_browser_tab_sources(tabs: &[BrowserTabSummary]) -> Vec<BrowserTabProjectionSource> {
    tabs.iter().map(BrowserTabProjectionSource::from).collect()
}

fn browser_tab_fallback_title(url: &str) -> String {
    if url.is_empty() || url == "about:blank" {
        return "New tab".to_owned();
    }
    url::Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| url.to_owned())
}

fn bounded_browser_tab_title(title: &str) -> String {
    if title.len() <= BROWSER_TAB_TITLE_MAX_BYTES {
        return title.to_owned();
    }

    let mut end = BROWSER_TAB_TITLE_MAX_BYTES - '…'.len_utf8();
    while !title.is_char_boundary(end) {
        end -= 1;
    }
    let mut bounded = String::with_capacity(BROWSER_TAB_TITLE_MAX_BYTES);
    bounded.push_str(&title[..end]);
    bounded.push('…');
    bounded
}
