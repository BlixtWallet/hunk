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
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Default)]
pub struct AiThreadCatalogProjection {
    pub items: Vec<AiThreadItem>,
    pub active_thread_id: String,
    pub thread_count: i32,
    pub running_thread_count: i32,
}

impl AiThreadCatalogProjection {
    pub fn from_state(state: &AiState, active_thread_id: Option<&str>) -> Self {
        let mut threads = state
            .threads
            .values()
            .filter(|thread| thread.status != ThreadLifecycleStatus::Archived)
            .collect::<Vec<_>>();
        threads.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
        });

        let active_thread_id = active_thread_id
            .filter(|active_thread_id| threads.iter().any(|thread| thread.id == *active_thread_id));
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
            .take(AI_THREAD_CATALOG_MAX_ITEMS)
            .map(|thread| {
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
                    created_at: thread.created_at,
                    updated_at: thread.updated_at,
                }
            })
            .collect();

        Self {
            items,
            active_thread_id: active_thread_id.unwrap_or_default().to_owned(),
            thread_count,
            running_thread_count,
        }
    }
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
    use super::{AiThreadItem, QListModel, QListModelBase};

    #[derive(Default)]
    pub struct AiThreadListModel {
        items: Vec<AiThreadItem>,
        replacement: Option<Vec<AiThreadItem>>,
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
    }

    impl QListModel for AiThreadListModel {
        type Item = AiThreadItem;

        fn len(&self) -> usize {
            self.items.len()
        }

        fn get(&self, index: usize) -> Option<&Self::Item> {
            self.items.get(index)
        }

        fn reset_unnotified(&mut self) {
            self.items = self.replacement.take().unwrap_or_default();
        }
    }
}

pub use thread_model::AiThreadListModel;
