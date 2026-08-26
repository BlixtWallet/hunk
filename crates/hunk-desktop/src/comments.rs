use std::collections::BTreeSet;
use std::sync::Arc;

use hunk_app::diff::{
    DiffCommentAnchor, DiffCommentScope, DiffCommentStoreCommand,
    execute_diff_comment_store_command,
};
use hunk_domain::db::{DatabaseStore, now_unix_ms};
use hunk_git::git::FileStatus;
use qtbridge::{QObjectHolder, invoke_method};

use crate::backend::Backend;
use crate::backend_state::{DiffCommentAsyncPayload, DiffCommentRequestKind};
use crate::comment_models::DiffCommentProjection;

const COMMENT_RETENTION_DAYS: i64 = 14;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DiffCommentStartOutcome {
    Started,
    RefreshQueued,
}

impl Backend {
    pub(super) fn active_diff_comment_scope(&self) -> Option<DiffCommentScope> {
        if !self.git_ready || self.git_root.trim().is_empty() {
            return None;
        }
        let branch_name = self.git_branch_name.trim();
        Some(DiffCommentScope {
            repo_root: self.git_root.clone(),
            branch_name: if branch_name.is_empty() || branch_name == "unknown" {
                "detached".to_owned()
            } else {
                branch_name.to_owned()
            },
        })
    }

    pub(super) fn initial_diff_comment_load_command(&self) -> DiffCommentStoreCommand {
        let prune_before_unix_ms = (!self.diff_comment_initial_prune_done).then(|| {
            let retention_ms = COMMENT_RETENTION_DAYS.saturating_mul(24 * 60 * 60 * 1000);
            now_unix_ms().saturating_sub(retention_ms)
        });
        DiffCommentStoreCommand::Load {
            prune_before_unix_ms,
        }
    }

    pub(super) fn start_diff_comment_command(
        &mut self,
        kind: DiffCommentRequestKind,
        command: DiffCommentStoreCommand,
    ) -> Result<DiffCommentStartOutcome, String> {
        if self.diff_comments_loading || self.diff_comments_busy {
            if kind == DiffCommentRequestKind::Load {
                self.diff_comment_refresh_pending = true;
                return Ok(DiffCommentStartOutcome::RefreshQueued);
            }
            return Err("Wait for the current comment operation to finish".to_owned());
        }

        let Some(scope) = self.active_diff_comment_scope() else {
            return Err("Open a Git repository before working with comments".to_owned());
        };
        self.diff_comment_epoch = self.diff_comment_epoch.wrapping_add(1).max(1);
        let epoch = self.diff_comment_epoch;
        let diff_epoch = self.diff_epoch;
        self.diff_comments_loading = kind == DiffCommentRequestKind::Load;
        self.diff_comments_busy = kind != DiffCommentRequestKind::Load;
        self.diff_comments_error.clear();
        let prunes_expired_comments = command_load_prunes(&command).unwrap_or(false);

        let anchors = Arc::clone(&self.diff_comment_anchors);
        let changed_paths = self
            .diff_file_summaries
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let renames_present = self
            .diff_file_summaries
            .values()
            .any(|summary| summary.status == FileStatus::Renamed);
        let results = Arc::clone(&self.diff_comment_results);
        let invoker = self.get_qml_method_invoker();
        let spawn_result = std::thread::Builder::new()
            .name("hunk-desktop-diff-comments".to_owned())
            .spawn(move || {
                let result = (|| {
                    let store = DatabaseStore::new()?;
                    let snapshot = execute_diff_comment_store_command(&store, &scope, command)?;
                    Ok::<_, anyhow::Error>(DiffCommentAsyncPayload {
                        kind,
                        diff_epoch,
                        projection: DiffCommentProjection::from_store_snapshot(
                            scope,
                            snapshot,
                            anchors.as_slice(),
                            changed_paths,
                            renames_present,
                        ),
                    })
                })()
                .map_err(|error| format!("{error:#}"));
                if let Ok(mut pending) = results.lock() {
                    pending.insert(epoch, result);
                }
                invoke_method!(invoker, "apply_diff_comment_result", epoch);
            });

        if let Err(error) = spawn_result {
            self.diff_comments_loading = false;
            self.diff_comments_busy = false;
            return Err(format!("Failed to start comment operation: {error}"));
        }
        if kind == DiffCommentRequestKind::Load && prunes_expired_comments {
            self.diff_comment_initial_prune_done = true;
        }
        Ok(DiffCommentStartOutcome::Started)
    }

    pub(super) fn apply_diff_comment_projection(&mut self, projection: DiffCommentProjection) {
        self.diff_comments_open_count = projection.open_count;
        self.diff_comments_stale_count = projection.stale_count;
        self.diff_comments_resolved_count = projection.resolved_count;
        if self.diff_comments_open_count == 0
            && self
                .diff_comments_stale_count
                .saturating_add(self.diff_comments_resolved_count)
                > 0
        {
            self.diff_comments_show_non_open = true;
        }
        if let Some(message) = projection.status_message.as_ref() {
            self.diff_comments_status_message.clone_from(message);
        }

        if let Some(pending_id) = self.diff_comment_pending_jump_id.clone() {
            if let Some(row) = projection.row_for_comment(pending_id.as_str()) {
                self.set_diff_comment_target(row);
                self.diff_comment_pending_jump_id = None;
                self.diff_comments_status_message = "Jumped to comment location.".to_owned();
            } else if self.diff_ready {
                if !self.diff_comment_anchors.is_empty() {
                    self.set_diff_comment_target(0);
                }
                self.diff_comment_pending_jump_id = None;
                self.diff_comments_status_message =
                    "Comment anchor not found; jumped to its file.".to_owned();
            }
        }

        self.diff_comment_projection = Some(projection);
        self.rebuild_diff_comment_items();
        self.diff_comments_ready = true;
        self.diff_comments_error.clear();
        self.bump_diff_comments_version();
    }

    pub(super) fn rebuild_diff_comment_items(&mut self) {
        let items = self
            .diff_comment_projection
            .as_ref()
            .map(|projection| projection.visible_items(self.diff_comments_show_non_open))
            .unwrap_or_default();
        self.diff_comments.borrow_mut().defer_replace(items);
    }

    pub(super) fn next_diff_comment_reconcile_command(
        &mut self,
    ) -> Option<DiffCommentStoreCommand> {
        let loaded_path = (self.diff_ready && !self.diff_selected_path.is_empty())
            .then_some(self.diff_selected_path.as_str());
        self.diff_comment_projection
            .as_ref()?
            .reconcile_command(loaded_path, &mut self.diff_comment_miss_streaks)
    }

    pub(super) fn clear_diff_comment_row_state(&mut self) {
        self.diff_comment_anchors = Arc::new(Vec::new());
        if let Some(projection) = self.diff_comment_projection.as_mut() {
            projection.clear_row_matches();
        }
        self.rebuild_diff_comment_items();
        self.diff_comment_target_row = -1;
        self.diff_comment_target_revision =
            self.diff_comment_target_revision.wrapping_add(1).max(1);
        self.bump_diff_comments_version();
    }

    pub(super) fn reset_diff_comment_state(&mut self) {
        self.diff_comment_epoch = self.diff_comment_epoch.wrapping_add(1).max(1);
        self.diff_comments.borrow_mut().defer_replace(Vec::new());
        self.diff_comment_projection = None;
        self.diff_comment_anchors = Arc::new(Vec::new());
        self.diff_comments_ready = false;
        self.diff_comments_loading = false;
        self.diff_comments_busy = false;
        self.diff_comments_error.clear();
        self.diff_comments_status_message.clear();
        self.diff_comments_show_non_open = false;
        self.diff_comments_open_count = 0;
        self.diff_comments_stale_count = 0;
        self.diff_comments_resolved_count = 0;
        self.diff_comment_refresh_pending = false;
        self.diff_comment_miss_streaks.clear();
        self.diff_comment_pending_jump_id = None;
        self.diff_comment_target_row = -1;
        self.diff_comment_target_revision =
            self.diff_comment_target_revision.wrapping_add(1).max(1);
        self.bump_diff_comments_version();
    }

    pub(super) fn set_diff_comment_target(&mut self, row: usize) {
        self.diff_comment_target_row = i32::try_from(row).unwrap_or(i32::MAX);
        self.diff_comment_target_revision =
            self.diff_comment_target_revision.wrapping_add(1).max(1);
    }

    pub(super) fn bump_diff_comments_version(&mut self) {
        self.diff_comments_version = self.diff_comments_version.wrapping_add(1).max(1);
    }

    pub(super) fn diff_comment_anchor(&self, row: i32) -> Option<DiffCommentAnchor> {
        let row = usize::try_from(row).ok()?;
        self.diff_comment_anchors.get(row)?.clone()
    }
}

fn command_load_prunes(command: &DiffCommentStoreCommand) -> Option<bool> {
    match command {
        DiffCommentStoreCommand::Load {
            prune_before_unix_ms,
        } => Some(prune_before_unix_ms.is_some()),
        _ => None,
    }
}
