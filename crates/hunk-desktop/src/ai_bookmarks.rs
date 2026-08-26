use std::collections::BTreeSet;
use std::sync::atomic::Ordering;
use std::thread::JoinHandle;

use hunk_app::ai::AiWorkerCommand;
use hunk_domain::state::AppStateStore;
use qtbridge::{QObjectHolder, invoke_method};

use crate::backend_state::{Backend, app_state_write_lock};

pub(super) struct AiBookmarkPersistResult {
    was_bookmarked: bool,
    previous_bookmarked_thread_ids: BTreeSet<String>,
    result: Result<(), String>,
}

#[derive(Default)]
pub(super) struct AiBookmarkTasks {
    tasks: Vec<JoinHandle<()>>,
}

impl AiBookmarkTasks {
    fn push(&mut self, task: JoinHandle<()>) {
        self.tasks.retain(|task| !task.is_finished());
        self.tasks.push(task);
    }

    pub(super) fn discard_finished(&mut self) {
        self.tasks.retain(|task| !task.is_finished());
    }
}

impl Drop for AiBookmarkTasks {
    fn drop(&mut self) {
        for task in self.tasks.drain(..) {
            let _ = task.join();
        }
    }
}

pub(super) fn queue_ai_toggle_thread_bookmark(backend: &mut Backend, thread_id: String) -> bool {
    let thread_id = thread_id.trim();
    if thread_id.is_empty() || !backend.ai_threads.borrow().contains_thread_id(thread_id) {
        return false;
    }

    let thread_id = thread_id.to_owned();
    let previous_bookmarked_thread_ids = backend.ai_bookmarked_thread_ids.clone();
    let was_bookmarked = previous_bookmarked_thread_ids.contains(thread_id.as_str());
    apply_bookmark_state(backend, thread_id.as_str(), !was_bookmarked);
    backend.ai_status_message = "Saving thread bookmarks…".to_owned();

    backend.ai_bookmark_epoch = backend.ai_bookmark_epoch.wrapping_add(1).max(1);
    let epoch = backend.ai_bookmark_epoch;
    backend
        .ai_bookmark_current_epoch
        .store(epoch, Ordering::Release);
    let current_epoch = backend.ai_bookmark_current_epoch.clone();
    let results = backend.ai_bookmark_results.clone();
    let bookmarked_thread_ids = backend.ai_bookmarked_thread_ids.clone();
    let rollback_bookmarked_thread_ids = previous_bookmarked_thread_ids.clone();
    let invoker = backend.get_qml_method_invoker();
    let spawn_result = std::thread::Builder::new()
        .name("hunk-desktop-ai-bookmarks".to_owned())
        .spawn(move || {
            let _writer_guard = app_state_write_lock()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if current_epoch.load(Ordering::Acquire) != epoch {
                return;
            }
            let result = match persist_bookmarks_locked(bookmarked_thread_ids) {
                Ok(()) => Ok(()),
                Err(error) => {
                    match persist_bookmarks_locked(rollback_bookmarked_thread_ids.clone()) {
                        Ok(()) => Err(error),
                        Err(recovery_error) => Err(format!(
                            "{error}; failed to restore saved bookmarks: {recovery_error}"
                        )),
                    }
                }
            };
            results
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .insert(
                    epoch,
                    AiBookmarkPersistResult {
                        was_bookmarked,
                        previous_bookmarked_thread_ids: rollback_bookmarked_thread_ids,
                        result,
                    },
                );
            invoke_method!(invoker, "complete_ai_bookmark_persist", epoch);
        });
    match spawn_result {
        Ok(task) => backend.ai_bookmark_tasks.push(task),
        Err(error) => {
            replace_bookmark_state(backend, previous_bookmarked_thread_ids);
            let recovery_error = persist_bookmarks(backend.ai_bookmarked_thread_ids.clone()).err();
            backend.ai_status_message = recovery_error.map_or_else(
                || format!("Failed to start bookmark save: {error}"),
                |recovery_error| {
                    format!(
                        "Failed to start bookmark save: {error}; failed to restore saved bookmarks: {recovery_error}"
                    )
                },
            );
            return false;
        }
    }

    true
}

pub(super) fn complete_ai_bookmark_persist(backend: &mut Backend, epoch: i32) {
    backend.ai_bookmark_tasks.discard_finished();
    let result = backend
        .ai_bookmark_results
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&epoch);
    let Some(result) = result else {
        return;
    };
    if backend.ai_bookmark_current_epoch.load(Ordering::Acquire) != epoch {
        return;
    }

    match result.result {
        Ok(()) => {
            backend.ai_status_message = if result.was_bookmarked {
                "Removed the thread bookmark."
            } else {
                "Bookmarked the thread."
            }
            .to_owned();
        }
        Err(error) => {
            replace_bookmark_state(backend, result.previous_bookmarked_thread_ids);
            backend.ai_status_message = format!("Failed to save thread bookmarks: {error}");
        }
    }
}

fn apply_bookmark_state(backend: &mut Backend, thread_id: &str, bookmarked: bool) {
    if bookmarked {
        backend
            .ai_bookmarked_thread_ids
            .insert(thread_id.to_owned());
    } else {
        backend.ai_bookmarked_thread_ids.remove(thread_id);
    }
    sync_bookmark_projection(backend);
}

fn replace_bookmark_state(backend: &mut Backend, bookmarked_thread_ids: BTreeSet<String>) {
    backend.ai_bookmarked_thread_ids = bookmarked_thread_ids;
    sync_bookmark_projection(backend);
}

fn sync_bookmark_projection(backend: &mut Backend) {
    backend
        .ai_threads
        .borrow_mut()
        .defer_apply_bookmarks(&backend.ai_bookmarked_thread_ids);
    backend
        .ai_runtime
        .mailbox
        .set_bookmarked_thread_ids(backend.ai_bookmarked_thread_ids.clone());
    if let Some(runtime) = &backend.ai_runtime.session {
        let _ = runtime.send(AiWorkerCommand::RefreshThreads);
    }
}

fn persist_bookmarks(bookmarked_thread_ids: BTreeSet<String>) -> Result<(), String> {
    let _guard = app_state_write_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    persist_bookmarks_locked(bookmarked_thread_ids)
}

fn persist_bookmarks_locked(bookmarked_thread_ids: BTreeSet<String>) -> Result<(), String> {
    let store = AppStateStore::new().map_err(|error| format!("{error:#}"))?;
    let mut state = store
        .load_or_default()
        .map_err(|error| format!("{error:#}"))?;
    state.ai_bookmarked_thread_ids = bookmarked_thread_ids;
    store.save(&state).map_err(|error| format!("{error:#}"))
}
