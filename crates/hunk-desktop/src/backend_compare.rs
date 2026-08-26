use std::path::PathBuf;
use std::sync::Arc;

use hunk_git::compare::load_compare_snapshot;
use qtbridge::{QObjectHolder, invoke_method};

use crate::Backend;
use crate::backend_state::{load_review_compare_selection, persist_review_compare_selection};
use crate::compare_models::{
    DiffCompareSnapshotPayload, DiffCompareSourceCatalog, DiffCompareSourceItem,
};

impl Backend {
    pub(super) fn configure_diff_compare(&mut self, catalog: DiffCompareSourceCatalog) -> bool {
        let persisted = load_review_compare_selection(self.git_root.as_str());
        self.diff_compare_sources
            .borrow_mut()
            .defer_replace(catalog.items);

        let sources = self.diff_compare_sources.borrow();
        let valid_source_id =
            |source_id: &String| !source_id.is_empty() && sources.contains(source_id.as_str());
        let mut left_source_id = [
            Some(self.diff_compare_left_source_id.clone()),
            persisted
                .as_ref()
                .and_then(|selection| selection.left_source_id.clone()),
            Some(catalog.default_left_source_id),
        ]
        .into_iter()
        .flatten()
        .find(valid_source_id)
        .unwrap_or_default();
        let right_source_id = [
            Some(self.diff_compare_right_source_id.clone()),
            persisted.and_then(|selection| selection.right_source_id),
            Some(catalog.default_right_source_id),
        ]
        .into_iter()
        .flatten()
        .find(valid_source_id)
        .unwrap_or_default();

        if left_source_id == right_source_id {
            left_source_id = sources
                .first_except(right_source_id.as_str())
                .map(|item| item.source_id)
                .unwrap_or_default();
        }
        drop(sources);
        if left_source_id.is_empty() || right_source_id.is_empty() {
            self.clear_diff_compare_selection();
            return false;
        }

        self.apply_diff_compare_selection(left_source_id, right_source_id)
    }

    pub(super) fn update_diff_compare_source(&mut self, side: &str, index: i32) {
        let Some(item) = self.diff_compare_sources.borrow().item_at(index) else {
            return;
        };
        let (left_source_id, right_source_id) = match side {
            "left" if item.source_id != self.diff_compare_right_source_id => {
                (item.source_id, self.diff_compare_right_source_id.clone())
            }
            "right" if item.source_id != self.diff_compare_left_source_id => {
                (self.diff_compare_left_source_id.clone(), item.source_id)
            }
            "left" | "right" => {
                self.diff_error = "Choose two different comparison sources".to_owned();
                self.diff_state_changed();
                return;
            }
            _ => return,
        };
        if !self.apply_diff_compare_selection(left_source_id, right_source_id) {
            return;
        }
        if let Err(error) = persist_review_compare_selection(
            self.git_root.as_str(),
            self.diff_compare_left_source_id.as_str(),
            self.diff_compare_right_source_id.as_str(),
        ) {
            self.set_status_message(format!(
                "Comparison changed; failed to save selection: {error:#}"
            ));
        }
        self.start_diff_compare_refresh();
    }

    pub(super) fn start_diff_compare_refresh(&mut self) {
        let Some((left, right)) = self.selected_diff_compare_sources() else {
            return;
        };
        if self.git_root.is_empty() {
            return;
        }

        self.diff_compare_epoch = self.diff_compare_epoch.wrapping_add(1).max(1);
        let epoch = self.diff_compare_epoch;
        self.diff_loading = true;
        self.diff_error.clear();
        self.diff_state_changed();

        let primary_repo_root = PathBuf::from(self.git_root.as_str());
        let invoker = self.get_qml_method_invoker();
        let pending_results = Arc::clone(&self.diff_compare_results);
        let spawn_result = std::thread::Builder::new()
            .name("hunk-desktop-diff-compare".to_owned())
            .spawn(move || {
                let result = load_compare_snapshot(primary_repo_root.as_path(), &left, &right)
                    .map(DiffCompareSnapshotPayload::from)
                    .map_err(|error| format!("{error:#}"));
                if let Ok(mut pending) = pending_results.lock() {
                    pending.insert(epoch, result);
                }
                invoke_method!(invoker, "apply_diff_compare_snapshot", epoch);
            });

        if let Err(error) = spawn_result {
            self.diff_loading = false;
            self.diff_error = format!("Failed to start comparison: {error}");
            self.diff_state_changed();
        }
    }

    pub(super) fn complete_diff_compare_snapshot(&mut self, epoch: i32) {
        let result = self
            .diff_compare_results
            .lock()
            .ok()
            .and_then(|mut pending| pending.remove(&epoch));
        if epoch != self.diff_compare_epoch {
            return;
        }

        match result {
            Some(Ok(payload)) => {
                self.diff_compare_file_count =
                    i32::try_from(payload.files.len()).unwrap_or(i32::MAX);
                self.diff_compare_patches = payload.patches_by_path;
                self.replace_diff_files(payload.files, payload.file_summaries);
                self.refresh_diff();
            }
            Some(Err(error)) => {
                self.diff_loading = false;
                self.diff_ready = false;
                self.diff_error = error;
                self.diff_state_changed();
            }
            None => {
                self.diff_loading = false;
                self.diff_ready = false;
                self.diff_error = "Comparison completed without a queued result".to_owned();
                self.diff_state_changed();
            }
        }
    }

    fn apply_diff_compare_selection(
        &mut self,
        left_source_id: String,
        right_source_id: String,
    ) -> bool {
        let sources = self.diff_compare_sources.borrow();
        let Some(left_index) = sources.index_of(left_source_id.as_str()) else {
            return false;
        };
        let Some(right_index) = sources.index_of(right_source_id.as_str()) else {
            return false;
        };
        let Some(left) = sources.item_at(i32::try_from(left_index).unwrap_or(i32::MAX)) else {
            return false;
        };
        let Some(right) = sources.item_at(i32::try_from(right_index).unwrap_or(i32::MAX)) else {
            return false;
        };
        drop(sources);

        self.diff_compare_left_source_id = left_source_id;
        self.diff_compare_right_source_id = right_source_id;
        self.diff_compare_left_label = left.label;
        self.diff_compare_right_label = right.label;
        self.diff_compare_left_index = i32::try_from(left_index).unwrap_or(i32::MAX);
        self.diff_compare_right_index = i32::try_from(right_index).unwrap_or(i32::MAX);
        self.diff_error.clear();
        self.diff_state_changed();
        true
    }

    fn selected_diff_compare_sources(
        &self,
    ) -> Option<(
        hunk_git::compare::CompareSource,
        hunk_git::compare::CompareSource,
    )> {
        let sources = self.diff_compare_sources.borrow();
        let left = source_with_id(&sources, self.diff_compare_left_source_id.as_str())?;
        let right = source_with_id(&sources, self.diff_compare_right_source_id.as_str())?;
        Some((left.compare_source()?, right.compare_source()?))
    }

    pub(super) fn clear_diff_compare_selection(&mut self) {
        self.diff_compare_left_source_id.clear();
        self.diff_compare_right_source_id.clear();
        self.diff_compare_left_label.clear();
        self.diff_compare_right_label.clear();
        self.diff_compare_left_index = -1;
        self.diff_compare_right_index = -1;
        self.diff_compare_file_count = 0;
        self.diff_compare_patches.clear();
    }
}

fn source_with_id(
    sources: &crate::DiffCompareSourceListModel,
    source_id: &str,
) -> Option<DiffCompareSourceItem> {
    let index = sources.index_of(source_id)?;
    sources.item_at(i32::try_from(index).ok()?)
}
