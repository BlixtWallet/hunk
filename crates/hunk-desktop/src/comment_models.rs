use std::collections::{BTreeSet, HashMap};

use hunk_app::diff::{
    DiffCommentAnchor, DiffCommentLookup, DiffCommentScope, DiffCommentStoreCommand,
    DiffCommentStoreSnapshot, find_diff_comment_row,
};
use hunk_domain::db::{
    CommentRecord, CommentStatus, comment_status_label, format_comment_clipboard_blob,
};
use qtbridge::{QListModel, QListModelBase, QModelItem, qobject};

const COMMENT_RECONCILE_MISS_THRESHOLD: u8 = 2;
const COMMENT_PREVIEW_MAX_ITEMS: usize = 64;

#[derive(Clone, Debug, Default, QModelItem)]
pub struct DiffCommentItem {
    pub comment_id: String,
    pub status: String,
    pub file_path: String,
    pub line_hint: String,
    pub comment_text: String,
    pub clipboard_text: String,
    pub row: i32,
    pub can_jump: bool,
}

#[derive(Clone, Debug)]
pub struct DiffCommentProjection {
    pub scope: DiffCommentScope,
    pub records: Vec<CommentRecord>,
    pub row_matches: HashMap<String, usize>,
    pub row_counts: Vec<i32>,
    pub open_count: i32,
    pub stale_count: i32,
    pub resolved_count: i32,
    pub status_message: Option<String>,
    changed_paths: BTreeSet<String>,
    renames_present: bool,
}

impl DiffCommentProjection {
    pub fn from_store_snapshot(
        scope: DiffCommentScope,
        snapshot: DiffCommentStoreSnapshot,
        anchors: &[Option<DiffCommentAnchor>],
        changed_paths: BTreeSet<String>,
        renames_present: bool,
    ) -> Self {
        let mut row_matches = HashMap::new();
        let mut row_counts = vec![0_i32; anchors.len()];
        let mut open_count = 0_i32;
        let mut stale_count = 0_i32;
        let mut resolved_count = 0_i32;

        for comment in &snapshot.comments {
            match comment.status {
                CommentStatus::Open => {
                    open_count = open_count.saturating_add(1);
                    if let Some(row) = find_diff_comment_row(anchors, comment_lookup(comment)) {
                        row_matches.insert(comment.id.clone(), row);
                        if let Some(count) = row_counts.get_mut(row) {
                            *count = count.saturating_add(1);
                        }
                    }
                }
                CommentStatus::Stale => stale_count = stale_count.saturating_add(1),
                CommentStatus::Resolved => resolved_count = resolved_count.saturating_add(1),
            }
        }

        Self {
            scope,
            records: snapshot.comments,
            row_matches,
            row_counts,
            open_count,
            stale_count,
            resolved_count,
            status_message: snapshot.status_message,
            changed_paths,
            renames_present,
        }
    }

    pub fn visible_items(&self, include_non_open: bool) -> Vec<DiffCommentItem> {
        self.records
            .iter()
            .filter(|comment| include_non_open || comment.status == CommentStatus::Open)
            .take(COMMENT_PREVIEW_MAX_ITEMS)
            .map(|comment| DiffCommentItem {
                comment_id: comment.id.clone(),
                status: comment_status_label(comment.status).to_owned(),
                file_path: comment.file_path.clone(),
                line_hint: comment_line_hint(comment),
                comment_text: comment.comment_text.clone(),
                clipboard_text: format_comment_clipboard_blob(comment),
                row: self
                    .row_matches
                    .get(comment.id.as_str())
                    .and_then(|row| i32::try_from(*row).ok())
                    .unwrap_or(-1),
                can_jump: self.row_matches.contains_key(comment.id.as_str())
                    || self.changed_paths.contains(comment.file_path.as_str()),
            })
            .collect()
    }

    pub fn comment(&self, id: &str) -> Option<&CommentRecord> {
        self.records.iter().find(|comment| comment.id == id)
    }

    pub fn row_for_comment(&self, id: &str) -> Option<usize> {
        self.row_matches.get(id).copied()
    }

    pub fn all_open_clipboard_text(&self) -> String {
        self.records
            .iter()
            .filter(|comment| comment.status == CommentStatus::Open)
            .map(format_comment_clipboard_blob)
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
    }

    pub fn row_count(&self, row: i32) -> i32 {
        usize::try_from(row)
            .ok()
            .and_then(|row| self.row_counts.get(row))
            .copied()
            .unwrap_or_default()
    }

    pub fn clear_row_matches(&mut self) {
        self.row_matches.clear();
        self.row_counts.clear();
    }

    pub fn reconcile_command(
        &self,
        loaded_path: Option<&str>,
        miss_streaks: &mut HashMap<String, u8>,
    ) -> Option<DiffCommentStoreCommand> {
        if !self.changed_paths.is_empty() && loaded_path.is_none() {
            return None;
        }

        let open_ids = self
            .records
            .iter()
            .filter(|comment| comment.status == CommentStatus::Open)
            .map(|comment| comment.id.as_str())
            .collect::<BTreeSet<_>>();
        miss_streaks.retain(|id, _| open_ids.contains(id.as_str()));

        let mut seen_ids = Vec::new();
        let mut stale_ids = Vec::new();
        let mut resolved_ids = Vec::new();
        for comment in self
            .records
            .iter()
            .filter(|comment| comment.status == CommentStatus::Open)
        {
            if self.row_matches.contains_key(comment.id.as_str()) {
                miss_streaks.remove(comment.id.as_str());
                seen_ids.push(comment.id.clone());
                continue;
            }

            let file_is_changed = self.changed_paths.contains(comment.file_path.as_str());
            if !file_is_changed && self.renames_present {
                continue;
            }
            if file_is_changed && loaded_path != Some(comment.file_path.as_str()) {
                continue;
            }

            let next_streak = miss_streaks
                .get(comment.id.as_str())
                .copied()
                .unwrap_or_default()
                .saturating_add(1);
            if next_streak < COMMENT_RECONCILE_MISS_THRESHOLD {
                miss_streaks.insert(comment.id.clone(), next_streak);
                continue;
            }
            miss_streaks.remove(comment.id.as_str());
            if file_is_changed {
                stale_ids.push(comment.id.clone());
            } else {
                resolved_ids.push(comment.id.clone());
            }
        }

        if seen_ids.is_empty() && stale_ids.is_empty() && resolved_ids.is_empty() {
            None
        } else {
            Some(DiffCommentStoreCommand::Reconcile {
                seen_ids,
                stale_ids,
                resolved_ids,
            })
        }
    }
}

fn comment_lookup(comment: &CommentRecord) -> DiffCommentLookup<'_> {
    DiffCommentLookup {
        file_path: comment.file_path.as_str(),
        line_side: comment.line_side,
        old_line: comment.old_line,
        new_line: comment.new_line,
        hunk_header: comment.hunk_header.as_deref(),
        line_text: comment.line_text.as_str(),
        context_before: comment.context_before.as_str(),
        context_after: comment.context_after.as_str(),
        anchor_hash: comment.anchor_hash.as_str(),
    }
}

fn comment_line_hint(comment: &CommentRecord) -> String {
    let old_line = comment
        .old_line
        .map(|line| line.to_string())
        .unwrap_or_else(|| "-".to_owned());
    let new_line = comment
        .new_line
        .map(|line| line.to_string())
        .unwrap_or_else(|| "-".to_owned());
    format!("old {old_line} | new {new_line}")
}

#[qobject(Base = QListModel)]
mod comment_model {
    use qtbridge::QObjectHolder;

    use super::{DiffCommentItem, QListModel, QListModelBase};

    #[derive(Default)]
    pub struct DiffCommentListModel {
        items: Vec<DiffCommentItem>,
        replacement: Option<Vec<DiffCommentItem>>,
        deferred_replacement: Option<Vec<DiffCommentItem>>,
        deferred_update_scheduled: bool,
    }

    impl DiffCommentListModel {
        pub fn replace(&mut self, items: Vec<DiffCommentItem>) {
            self.replacement = Some(items);
            self.reset();
        }

        pub fn defer_replace(&mut self, items: Vec<DiffCommentItem>) {
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

    impl QListModel for DiffCommentListModel {
        type Item = DiffCommentItem;

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

pub use comment_model::DiffCommentListModel;
