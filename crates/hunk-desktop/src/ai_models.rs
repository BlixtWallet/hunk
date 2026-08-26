use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};

use hunk_codex::state::{AiState, ThreadLifecycleStatus, TurnStatus};
use qtbridge::{QListModel, QListModelBase, QModelItem, qobject};

const AI_THREAD_CATALOG_MAX_ITEMS: usize = 200;

#[derive(Clone, Debug, Default, Eq, PartialEq, QModelItem)]
pub struct AiThreadItem {
    pub thread_id: String,
    pub title: String,
    pub cwd: String,
    pub workspace_label: String,
    pub status: String,
    pub active: bool,
    pub running: bool,
    pub attention: bool,
    pub bookmarked: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Default)]
pub struct AiThreadCatalogProjection {
    pub items: Vec<AiThreadItem>,
    pub active_thread_id: String,
    pub active_thread_title: String,
    pub active_thread_cwd: String,
    pub thread_count: i32,
    pub running_thread_count: i32,
}

impl AiThreadCatalogProjection {
    pub fn from_state(state: &AiState, active_thread_id: Option<&str>) -> Self {
        Self::from_state_with_bookmarks(state, active_thread_id, &BTreeSet::new())
    }

    pub fn from_state_with_bookmarks(
        state: &AiState,
        active_thread_id: Option<&str>,
        bookmarked_thread_ids: &BTreeSet<String>,
    ) -> Self {
        let mut threads = state
            .threads
            .values()
            .filter(|thread| thread.status != ThreadLifecycleStatus::Archived)
            .collect::<Vec<_>>();
        threads.sort_by(|left, right| {
            bookmarked_thread_ids
                .contains(right.id.as_str())
                .cmp(&bookmarked_thread_ids.contains(left.id.as_str()))
                .then_with(|| right.created_at.cmp(&left.created_at))
                .then_with(|| right.id.cmp(&left.id))
        });

        let active_thread = active_thread_id
            .and_then(|active_thread_id| {
                threads.iter().find(|thread| thread.id == active_thread_id)
            })
            .copied();
        let active_thread_id = active_thread.map(|thread| thread.id.as_str());
        let active_thread_position = active_thread_id.and_then(|active_thread_id| {
            threads
                .iter()
                .position(|thread| thread.id == active_thread_id)
        });
        let newest_thread_limit = if active_thread_position
            .is_some_and(|position| position >= AI_THREAD_CATALOG_MAX_ITEMS)
        {
            AI_THREAD_CATALOG_MAX_ITEMS.saturating_sub(1)
        } else {
            AI_THREAD_CATALOG_MAX_ITEMS
        };
        let thread_count = saturating_usize_to_i32(threads.len());
        let running_thread_ids = state
            .turns
            .values()
            .filter(|turn| turn.status == TurnStatus::InProgress)
            .map(|turn| turn.thread_id.as_str())
            .collect::<BTreeSet<_>>();
        let running_thread_count = saturating_usize_to_i32(
            threads
                .iter()
                .filter(|thread| running_thread_ids.contains(thread.id.as_str()))
                .count(),
        );
        let items = threads
            .into_iter()
            .enumerate()
            .filter(|(index, thread)| {
                *index < newest_thread_limit || active_thread_id == Some(thread.id.as_str())
            })
            .take(AI_THREAD_CATALOG_MAX_ITEMS)
            .map(|(_, thread)| {
                let running = running_thread_ids.contains(thread.id.as_str());
                AiThreadItem {
                    thread_id: thread.id.clone(),
                    title: thread
                        .title
                        .as_deref()
                        .map(str::trim)
                        .filter(|title| !title.is_empty())
                        .unwrap_or("Untitled thread")
                        .to_owned(),
                    cwd: thread.cwd.clone(),
                    workspace_label: workspace_label(thread.cwd.as_str()),
                    status: thread_status_label(thread.status).to_owned(),
                    active: active_thread_id == Some(thread.id.as_str()),
                    running,
                    attention: false,
                    bookmarked: bookmarked_thread_ids.contains(thread.id.as_str()),
                    created_at: thread.created_at,
                    updated_at: thread.updated_at,
                }
            })
            .collect();

        Self {
            items,
            active_thread_id: active_thread_id.unwrap_or_default().to_owned(),
            active_thread_title: active_thread
                .map(|thread| {
                    thread
                        .title
                        .as_deref()
                        .map(str::trim)
                        .filter(|title| !title.is_empty())
                        .unwrap_or("Untitled thread")
                        .to_owned()
                })
                .unwrap_or_default(),
            active_thread_cwd: active_thread
                .map(|thread| thread.cwd.clone())
                .unwrap_or_default(),
            thread_count,
            running_thread_count,
        }
    }

    pub fn mark_attention(&mut self, thread_ids: &BTreeSet<String>) {
        for item in &mut self.items {
            item.attention = thread_ids.contains(item.thread_id.as_str());
        }
    }

    pub fn apply_bookmarks(&mut self, bookmarked_thread_ids: &BTreeSet<String>) {
        apply_bookmarks_to_items(&mut self.items, bookmarked_thread_ids);
    }
}

fn apply_bookmarks_to_items(items: &mut [AiThreadItem], bookmarked_thread_ids: &BTreeSet<String>) {
    for item in items.iter_mut() {
        item.bookmarked = bookmarked_thread_ids.contains(item.thread_id.as_str());
    }
    items.sort_by(compare_thread_items);
}

fn compare_thread_items(left: &AiThreadItem, right: &AiThreadItem) -> Ordering {
    right
        .bookmarked
        .cmp(&left.bookmarked)
        .then_with(|| right.created_at.cmp(&left.created_at))
        .then_with(|| right.thread_id.cmp(&left.thread_id))
}

fn workspace_label(cwd: &str) -> String {
    std::path::Path::new(cwd)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| cwd.to_owned())
}

fn thread_status_label(status: ThreadLifecycleStatus) -> &'static str {
    match status {
        ThreadLifecycleStatus::Active => "active",
        ThreadLifecycleStatus::Idle => "idle",
        ThreadLifecycleStatus::NotLoaded => "not loaded",
        ThreadLifecycleStatus::Archived => "archived",
        ThreadLifecycleStatus::Closed => "closed",
    }
}

fn saturating_usize_to_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

#[qobject(Base = QListModel)]
mod thread_model {
    use std::collections::BTreeSet;

    use qtbridge::QObjectHolder;
    use qtbridge::qtbridge_type_lib::QModelIndex;

    use super::{
        AiThreadItem, QListModel, QListModelBase, apply_bookmarks_to_items, compare_thread_items,
    };

    #[derive(Default)]
    pub struct AiThreadListModel {
        items: Vec<AiThreadItem>,
        replacement: Option<Vec<AiThreadItem>>,
        deferred_replacement: Option<Vec<AiThreadItem>>,
        deferred_update_scheduled: bool,
    }

    impl AiThreadListModel {
        pub fn replace(&mut self, items: Vec<AiThreadItem>) {
            self.replacement = Some(items);
            self.reset();
        }

        pub fn replace_if_changed(&mut self, items: Vec<AiThreadItem>) -> bool {
            if self.items == items {
                return false;
            }
            self.replace(items);
            true
        }

        pub fn defer_replace(&mut self, items: Vec<AiThreadItem>) {
            self.queue_deferred_replacement(items);
        }

        pub fn defer_replace_if_changed(&mut self, items: Vec<AiThreadItem>) -> bool {
            let current = self.deferred_replacement.as_ref().unwrap_or(&self.items);
            if current == &items {
                return false;
            }
            self.queue_deferred_replacement(items);
            true
        }

        pub fn contains_thread_id(&self, thread_id: &str) -> bool {
            self.deferred_replacement
                .as_ref()
                .unwrap_or(&self.items)
                .iter()
                .any(|thread| thread.thread_id == thread_id)
        }

        pub fn defer_apply_bookmarks(&mut self, bookmarked_thread_ids: &BTreeSet<String>) -> bool {
            let mut items = self
                .deferred_replacement
                .as_ref()
                .unwrap_or(&self.items)
                .clone();
            let previous = items.clone();
            apply_bookmarks_to_items(&mut items, bookmarked_thread_ids);
            if items == previous {
                return false;
            }
            self.queue_deferred_replacement(items);
            true
        }

        pub fn apply_bookmarks(&mut self, bookmarked_thread_ids: &BTreeSet<String>) -> bool {
            let mut changed = false;
            while let Some(source) = self.items.iter().position(|item| {
                item.bookmarked != bookmarked_thread_ids.contains(item.thread_id.as_str())
            }) {
                let mut item = self.items[source].clone();
                item.bookmarked = !item.bookmarked;
                let destination = self
                    .items
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != source)
                    .filter(|(_, candidate)| compare_thread_items(candidate, &item).is_lt())
                    .count();
                if !self.set(source, item) {
                    break;
                }
                if source != destination {
                    self.move_item(source, destination);
                }
                changed = true;
            }
            changed
        }

        fn move_item(&mut self, source: usize, destination: usize) {
            let proxy = self.try_get_rust_proxy_ptr().expect("No proxy");
            let parent = QModelIndex::default();
            let destination_child = if destination > source {
                destination + 1
            } else {
                destination
            };
            unsafe { &mut *proxy }.base_begin_move_rows(
                &mut *self,
                &parent,
                source as i32,
                source as i32,
                &parent,
                destination_child as i32,
            );
            let item = self.items.remove(source);
            self.items.insert(destination, item);
            unsafe { &mut *proxy }.base_end_move_rows(&mut *self);
        }

        fn queue_deferred_replacement(&mut self, items: Vec<AiThreadItem>) {
            self.deferred_replacement = Some(items);
            if self.deferred_update_scheduled {
                return;
            }
            self.deferred_update_scheduled = true;
            if !self
                .get_qml_method_invoker()
                .invoke_method("apply_deferred_replacement")
            {
                self.deferred_update_scheduled = false;
            }
        }

        #[qslot]
        fn apply_deferred_replacement(&mut self) {
            self.deferred_update_scheduled = false;
            let Some(items) = self.deferred_replacement.take() else {
                return;
            };
            self.replace(items);
        }
    }

    impl QListModel for AiThreadListModel {
        type Item = AiThreadItem;

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

pub use thread_model::AiThreadListModel;
