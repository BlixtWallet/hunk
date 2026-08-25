use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use hunk_app::ai::{AiTurnSessionOverrides, AiWorkerCommand};
use hunk_app::diff::DiffCommentStoreCommand;
use hunk_domain::db::CommentStatus;
use hunk_domain::state::AppStateStore;
use hunk_forge::{ForgeCredentialKind, ForgeProvider, ForgeReviewWorkspace};
use hunk_git::workspace::{GitWorkspaceCommand, execute_git_workspace_command, load_git_workspace};
use qtbridge::{QObjectHolder, invoke_method, qobject, qtbridge_type_lib::QString};

use crate::ai_models::AiThreadListModel;
use crate::ai_runtime::{prepare_ai_worker_config, start_ai_runtime};
use crate::ai_timeline_models::AiTimelineListModel;
use crate::backend_ai::{apply_ai_runtime_events, reset_ai_runtime_state, stop_ai_runtime};
pub use crate::backend_state::{Backend, Workspace};
use crate::backend_state::{DiffCommentRequestKind, ForgeAsyncPayload};
use crate::comment_models::DiffCommentListModel;
use crate::comments::DiffCommentStartOutcome;
use crate::diff_models::{DiffFileSummary, DiffRowListModel, DiffSnapshotPayload};
use crate::forge::{
    ForgeSnapshotPayload, create_or_find_review, load_forge_snapshot, provider_label,
    review_kind_label, review_short_label, review_state_label, run_github_device_flow,
    save_forge_token,
};
use crate::git_models::{
    GitBranchListModel, GitCommitListModel, GitFileListModel, GitSnapshotPayload,
};
use crate::local_path_from_qml_folder_url;

#[qobject]
impl Backend {
    qproperty!(
        "activeWorkspace",
        Member = active_workspace,
        Notify = active_workspace_changed
    );
    qproperty!("diffFiles", Read = diff_files, Constant);
    qproperty!("diffRows", Read = diff_rows, Constant);
    qproperty!(
        "diffSelectedPath",
        Member = diff_selected_path,
        Notify = diff_state_changed
    );
    qproperty!(
        "diffStatusTag",
        Member = diff_status_tag,
        Notify = diff_state_changed
    );
    qproperty!(
        "diffAdditions",
        Member = diff_additions,
        Notify = diff_state_changed
    );
    qproperty!(
        "diffRemovals",
        Member = diff_removals,
        Notify = diff_state_changed
    );
    qproperty!(
        "diffReady",
        Member = diff_ready,
        Notify = diff_state_changed
    );
    qproperty!(
        "diffLoading",
        Member = diff_loading,
        Notify = diff_state_changed
    );
    qproperty!(
        "diffError",
        Member = diff_error,
        Notify = diff_state_changed
    );
    qproperty!(
        "diffSearchQuery",
        Member = diff_search_query,
        Notify = diff_state_changed
    );
    qproperty!(
        "diffSearchMatchCount",
        Member = diff_search_match_count,
        Notify = diff_state_changed
    );
    qproperty!(
        "diffSearchMatchIndex",
        Member = diff_search_match_index,
        Notify = diff_state_changed
    );
    qproperty!(
        "diffSearchTargetRow",
        Member = diff_search_target_row,
        Notify = diff_state_changed
    );
    qproperty!("diffComments", Read = diff_comments, Constant);
    qproperty!(
        "diffCommentsReady",
        Member = diff_comments_ready,
        Notify = diff_comments_state_changed
    );
    qproperty!(
        "diffCommentsLoading",
        Member = diff_comments_loading,
        Notify = diff_comments_state_changed
    );
    qproperty!(
        "diffCommentsBusy",
        Member = diff_comments_busy,
        Notify = diff_comments_state_changed
    );
    qproperty!(
        "diffCommentsError",
        Member = diff_comments_error,
        Notify = diff_comments_state_changed
    );
    qproperty!(
        "diffCommentsStatusMessage",
        Member = diff_comments_status_message,
        Notify = diff_comments_state_changed
    );
    qproperty!(
        "diffCommentsShowNonOpen",
        Member = diff_comments_show_non_open,
        Notify = diff_comments_state_changed
    );
    qproperty!(
        "diffCommentsOpenCount",
        Member = diff_comments_open_count,
        Notify = diff_comments_state_changed
    );
    qproperty!(
        "diffCommentsStaleCount",
        Member = diff_comments_stale_count,
        Notify = diff_comments_state_changed
    );
    qproperty!(
        "diffCommentsResolvedCount",
        Member = diff_comments_resolved_count,
        Notify = diff_comments_state_changed
    );
    qproperty!(
        "diffCommentsVersion",
        Member = diff_comments_version,
        Notify = diff_comments_state_changed
    );
    qproperty!(
        "diffCommentTargetRow",
        Member = diff_comment_target_row,
        Notify = diff_comments_state_changed
    );
    qproperty!(
        "diffCommentTargetRevision",
        Member = diff_comment_target_revision,
        Notify = diff_comments_state_changed
    );
    qproperty!("gitFiles", Read = git_files, Constant);
    qproperty!("gitBranches", Read = git_branches, Constant);
    qproperty!("gitCommits", Read = git_commits, Constant);
    qproperty!("gitRoot", Member = git_root, Notify = git_state_changed);
    qproperty!(
        "gitRepositoryName",
        Member = git_repository_name,
        Notify = git_state_changed
    );
    qproperty!(
        "gitBranchName",
        Member = git_branch_name,
        Notify = git_state_changed
    );
    qproperty!(
        "gitBranchHasUpstream",
        Member = git_branch_has_upstream,
        Notify = git_state_changed
    );
    qproperty!(
        "gitBranchAheadCount",
        Member = git_branch_ahead_count,
        Notify = git_state_changed
    );
    qproperty!(
        "gitBranchBehindCount",
        Member = git_branch_behind_count,
        Notify = git_state_changed
    );
    qproperty!(
        "gitChangedFileCount",
        Member = git_changed_file_count,
        Notify = git_state_changed
    );
    qproperty!(
        "gitStagedFileCount",
        Member = git_staged_file_count,
        Notify = git_state_changed
    );
    qproperty!(
        "gitUnstagedFileCount",
        Member = git_unstaged_file_count,
        Notify = git_state_changed
    );
    qproperty!(
        "gitLastCommitSubject",
        Member = git_last_commit_subject,
        Notify = git_state_changed
    );
    qproperty!("gitReady", Member = git_ready, Notify = git_state_changed);
    qproperty!(
        "gitLoading",
        Member = git_loading,
        Notify = git_state_changed
    );
    qproperty!("gitBusy", Member = git_busy, Notify = git_state_changed);
    qproperty!("gitError", Member = git_error, Notify = git_state_changed);
    qproperty!(
        "gitStatusMessage",
        Member = git_status_message,
        Notify = git_state_changed
    );
    qproperty!(
        "gitActionLabel",
        Member = git_action_label,
        Notify = git_state_changed
    );
    qproperty!("aiThreads", Read = ai_threads, Constant);
    qproperty!("aiTimeline", Read = ai_timeline, Constant);
    qproperty!("aiReady", Member = ai_ready, Notify = ai_state_changed);
    qproperty!("aiLoading", Member = ai_loading, Notify = ai_state_changed);
    qproperty!(
        "aiRequiresAuthentication",
        Member = ai_requires_authentication,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiConnectionState",
        Member = ai_connection_state,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiWorkspaceRoot",
        Member = ai_workspace_root,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiActiveThreadId",
        Member = ai_active_thread_id,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiActiveThreadTitle",
        Member = ai_active_thread_title,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiActiveThreadCwd",
        Member = ai_active_thread_cwd,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiThreadCount",
        Member = ai_thread_count,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiRunningThreadCount",
        Member = ai_running_thread_count,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiTimelineTotalTurnCount",
        Member = ai_timeline_total_turn_count,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiTimelineVisibleTurnCount",
        Member = ai_timeline_visible_turn_count,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiTimelineHiddenTurnCount",
        Member = ai_timeline_hidden_turn_count,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiTimelineTotalRowCount",
        Member = ai_timeline_total_row_count,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiTimelineHiddenRowCount",
        Member = ai_timeline_hidden_row_count,
        Notify = ai_state_changed
    );
    qproperty!("aiError", Member = ai_error, Notify = ai_state_changed);
    qproperty!(
        "aiStatusMessage",
        Member = ai_status_message,
        Notify = ai_state_changed
    );
    qproperty!(
        "forgeAvailable",
        Member = forge_available,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeProviderLabel",
        Member = forge_provider_label,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeReviewKindLabel",
        Member = forge_review_kind_label,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeHost",
        Member = forge_host,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeRepositoryPath",
        Member = forge_repository_path,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeAuthenticated",
        Member = forge_authenticated,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeAccountLabel",
        Member = forge_account_label,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeAuthMode",
        Member = forge_auth_mode,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeReady",
        Member = forge_ready,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeLoading",
        Member = forge_loading,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeBusy",
        Member = forge_busy,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeError",
        Member = forge_error,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeStatusMessage",
        Member = forge_status_message,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeActionLabel",
        Member = forge_action_label,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeDefaultTargetBranch",
        Member = forge_default_target_branch,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeReviewExists",
        Member = forge_review_exists,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeReviewNumber",
        Member = forge_review_number,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeReviewTitle",
        Member = forge_review_title,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeReviewUrl",
        Member = forge_review_url,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeReviewState",
        Member = forge_review_state,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeReviewDraft",
        Member = forge_review_draft,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeDeviceFlowActive",
        Member = forge_device_flow_active,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeDeviceUserCode",
        Member = forge_device_user_code,
        Notify = forge_state_changed
    );
    qproperty!(
        "forgeDeviceVerificationUrl",
        Member = forge_device_verification_url,
        Notify = forge_state_changed
    );
    qproperty!("ready", Member = ready, Notify = ready_changed);
    qproperty!(
        "statusMessage",
        Member = status_message,
        Notify = status_message_changed
    );

    #[qsignal]
    fn active_workspace_changed(&mut self);

    #[qsignal]
    fn ready_changed(&mut self);

    #[qsignal]
    fn status_message_changed(&mut self);

    #[qsignal]
    fn diff_state_changed(&mut self);

    #[qsignal]
    fn diff_comments_state_changed(&mut self);

    #[qsignal]
    fn git_state_changed(&mut self);

    #[qsignal]
    fn ai_state_changed(&mut self);

    #[qsignal]
    fn forge_state_changed(&mut self);

    #[qslot]
    fn select_workspace(&mut self, workspace: String) {
        let Some(workspace) = Workspace::parse(&workspace) else {
            self.set_status_message(format!("Unknown workspace: {workspace}"));
            return;
        };
        if self.active_workspace == workspace.as_str() {
            if workspace == Workspace::Ai {
                self.ensure_ai_runtime_started();
            }
            return;
        }

        self.active_workspace = workspace.as_str().to_owned();
        self.active_workspace_changed();
        if workspace == Workspace::Ai {
            self.ensure_ai_runtime_started();
        }
    }

    #[qslot]
    fn bootstrap(&mut self) {
        if self.bootstrap_started {
            return;
        }
        self.bootstrap_started = true;

        let invoker = self.get_qml_method_invoker();
        let spawn_result = std::thread::Builder::new()
            .name("hunk-qt-bootstrap".to_owned())
            .spawn(move || {
                invoke_method!(
                    invoker,
                    "complete_bootstrap",
                    true,
                    QString::from("Qt shell connected to Rust")
                );
            });

        if let Err(error) = spawn_result {
            self.complete_bootstrap(
                false,
                format!("Failed to start application services: {error}"),
            );
        }
    }

    #[qslot]
    fn complete_bootstrap(&mut self, ready: bool, status_message: String) {
        if self.ready != ready {
            self.ready = ready;
            self.ready_changed();
        }
        self.set_status_message(status_message);
        if ready {
            self.refresh_git_workspace();
        }
    }

    #[qslot]
    fn select_diff_file(&mut self, path: String) {
        if path == self.diff_selected_path {
            return;
        }
        let Some(summary) = self.diff_file_summaries.get(path.as_str()).cloned() else {
            self.diff_error = format!("Changed file is no longer available: {path}");
            self.diff_state_changed();
            return;
        };

        self.apply_diff_selection(&summary);
        self.refresh_diff();
    }

    #[qslot]
    fn refresh_diff(&mut self) {
        if self.diff_loading || self.diff_selected_path.is_empty() {
            return;
        }
        let Some(summary) = self
            .diff_file_summaries
            .get(self.diff_selected_path.as_str())
            .cloned()
        else {
            self.diff_error = "Selected diff is no longer available".to_owned();
            self.diff_state_changed();
            return;
        };

        self.diff_epoch = self.diff_epoch.wrapping_add(1).max(1);
        let epoch = self.diff_epoch;
        self.diff_loading = true;
        self.diff_ready = false;
        self.diff_error.clear();
        self.diff_rows.borrow_mut().replace(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Arc::new(Vec::new()),
        );
        self.clear_diff_comment_row_state();
        self.clear_diff_search_results();
        self.diff_state_changed();
        self.diff_comments_state_changed();

        let root = PathBuf::from(self.git_root.clone());
        let invoker = self.get_qml_method_invoker();
        let refresh_results = Arc::clone(&self.diff_refresh_results);
        let spawn_result = std::thread::Builder::new()
            .name("hunk-qt-diff-refresh".to_owned())
            .spawn(move || {
                let result = DiffSnapshotPayload::load(root.as_path(), &summary)
                    .map_err(|error| format!("{error:#}"));
                if let Ok(mut pending) = refresh_results.lock() {
                    pending.insert(epoch, result);
                }
                invoke_method!(invoker, "apply_diff_snapshot", epoch);
            });

        if let Err(error) = spawn_result {
            self.diff_loading = false;
            self.diff_error = format!("Failed to start diff refresh: {error}");
            self.diff_state_changed();
        }
    }

    #[qslot]
    fn apply_diff_snapshot(&mut self, epoch: i32) {
        let result = self
            .diff_refresh_results
            .lock()
            .ok()
            .and_then(|mut pending| pending.remove(&epoch));
        if epoch != self.diff_epoch {
            return;
        }

        self.diff_loading = false;
        let mut refresh_comments = false;
        match result {
            Some(Ok(payload)) if payload.path == self.diff_selected_path => {
                self.diff_status_tag = payload.status_tag;
                self.diff_additions = payload.additions;
                self.diff_removals = payload.removals;
                self.diff_comment_anchors = Arc::clone(&payload.comment_anchors);
                self.diff_rows.borrow_mut().replace(
                    payload.rows,
                    payload.search_texts,
                    payload.copy_texts,
                    payload.comment_anchors,
                );
                self.rebuild_diff_search_results();
                self.diff_ready = true;
                self.diff_error.clear();
                refresh_comments = true;
            }
            Some(Ok(_)) => return,
            Some(Err(error)) => {
                self.diff_ready = false;
                self.diff_error = error;
            }
            None => {
                self.diff_ready = false;
                self.diff_error = "Diff refresh completed without a queued result".to_owned();
            }
        }
        self.diff_state_changed();
        if refresh_comments {
            self.refresh_diff_comments();
        }
    }

    #[qslot]
    fn set_diff_search(&mut self, query: String) {
        if self.diff_search_query == query {
            return;
        }
        self.diff_search_query = query;
        self.rebuild_diff_search_results();
        self.diff_state_changed();
    }

    #[qslot]
    fn move_diff_search_match(&mut self, direction: i32) {
        let count = self.diff_search_matches.len();
        if count == 0 {
            return;
        }
        let count = i32::try_from(count).unwrap_or(i32::MAX);
        let current = if self.diff_search_match_index < 0 {
            0
        } else {
            (self.diff_search_match_index + direction).rem_euclid(count)
        };
        self.diff_search_match_index = current;
        self.diff_search_target_row = self
            .diff_search_matches
            .get(current as usize)
            .and_then(|index| i32::try_from(*index).ok())
            .unwrap_or(-1);
        self.diff_state_changed();
    }

    #[qslot]
    fn diff_selection_text(&self, anchor: i32, head: i32) -> String {
        self.diff_rows.borrow().selection_text(anchor, head)
    }

    #[qslot]
    fn diff_hunk_target(&self, start: i32, direction: i32) -> i32 {
        self.diff_rows.borrow().hunk_target(start, direction)
    }

    #[qslot]
    fn refresh_diff_comments(&mut self) {
        let command = self.initial_diff_comment_load_command();
        match self.start_diff_comment_command(DiffCommentRequestKind::Load, command) {
            Ok(DiffCommentStartOutcome::Started) => self.diff_comments_state_changed(),
            Ok(DiffCommentStartOutcome::RefreshQueued) => {}
            Err(error) => {
                self.diff_comments_error = error;
                self.diff_comments_state_changed();
            }
        }
    }

    #[qslot]
    fn create_diff_comment(&mut self, row: i32, comment_text: String) {
        let Some(anchor) = self.diff_comment_anchor(row) else {
            self.diff_comments_error =
                "This diff row does not have a stable comment anchor".to_owned();
            self.diff_comments_state_changed();
            return;
        };
        if comment_text.trim().is_empty() {
            self.diff_comments_error = "Comment text cannot be empty".to_owned();
            self.diff_comments_state_changed();
            return;
        }
        let command = DiffCommentStoreCommand::Create {
            anchor,
            comment_text,
        };
        match self.start_diff_comment_command(DiffCommentRequestKind::Mutation, command) {
            Ok(_) => self.diff_comments_state_changed(),
            Err(error) => {
                self.diff_comments_error = error;
                self.diff_comments_state_changed();
            }
        }
    }

    #[qslot]
    fn set_diff_comment_status(&mut self, id: String, status: String) {
        let status = match status.as_str() {
            "open" => CommentStatus::Open,
            "stale" => CommentStatus::Stale,
            "resolved" => CommentStatus::Resolved,
            _ => {
                self.diff_comments_error = format!("Unknown comment status: {status}");
                self.diff_comments_state_changed();
                return;
            }
        };
        let command = DiffCommentStoreCommand::SetStatus { id, status };
        match self.start_diff_comment_command(DiffCommentRequestKind::Mutation, command) {
            Ok(_) => self.diff_comments_state_changed(),
            Err(error) => {
                self.diff_comments_error = error;
                self.diff_comments_state_changed();
            }
        }
    }

    #[qslot]
    fn delete_diff_comment(&mut self, id: String) {
        let command = DiffCommentStoreCommand::Delete { id };
        match self.start_diff_comment_command(DiffCommentRequestKind::Mutation, command) {
            Ok(_) => self.diff_comments_state_changed(),
            Err(error) => {
                self.diff_comments_error = error;
                self.diff_comments_state_changed();
            }
        }
    }

    #[qslot]
    fn set_diff_comments_show_non_open(&mut self, show: bool) {
        if self.diff_comments_show_non_open == show {
            return;
        }
        self.diff_comments_show_non_open = show;
        self.rebuild_diff_comment_items();
        self.diff_comments_state_changed();
    }

    #[qslot]
    fn jump_to_diff_comment(&mut self, id: String) {
        let Some(projection) = self.diff_comment_projection.as_ref() else {
            self.diff_comments_error = "Comments are not loaded yet".to_owned();
            self.diff_comments_state_changed();
            return;
        };
        if let Some(row) = projection.row_for_comment(id.as_str()) {
            self.set_diff_comment_target(row);
            self.diff_comments_status_message = "Jumped to comment location.".to_owned();
            self.diff_comments_state_changed();
            return;
        }
        let Some(path) = projection
            .comment(id.as_str())
            .map(|comment| comment.file_path.clone())
        else {
            self.diff_comments_error = "Comment is no longer available".to_owned();
            self.diff_comments_state_changed();
            return;
        };
        let Some(summary) = self.diff_file_summaries.get(path.as_str()).cloned() else {
            self.diff_comments_status_message =
                "Comment location is not visible in the current changes.".to_owned();
            self.diff_comments_state_changed();
            return;
        };
        if path == self.diff_selected_path && self.diff_ready {
            self.diff_comments_status_message =
                "Comment anchor was not found in this diff.".to_owned();
            self.diff_comments_state_changed();
            return;
        }

        self.diff_comment_pending_jump_id = Some(id);
        self.apply_diff_selection(&summary);
        self.refresh_diff();
    }

    #[qslot]
    fn diff_comment_count_for_row(&self, row: i32) -> i32 {
        self.diff_comment_projection
            .as_ref()
            .map(|projection| projection.row_count(row))
            .unwrap_or_default()
    }

    #[qslot]
    fn diff_row_supports_comments(&self, row: i32) -> bool {
        usize::try_from(row)
            .ok()
            .and_then(|row| self.diff_comment_anchors.get(row))
            .is_some_and(|anchor| anchor.is_some())
    }

    #[qslot]
    fn diff_comment_line_hint(&self, row: i32) -> String {
        let Some(anchor) = usize::try_from(row)
            .ok()
            .and_then(|row| self.diff_comment_anchors.get(row))
            .and_then(Option::as_ref)
        else {
            return String::new();
        };
        let old_line = anchor
            .old_line
            .map(|line| line.to_string())
            .unwrap_or_else(|| "-".to_owned());
        let new_line = anchor
            .new_line
            .map(|line| line.to_string())
            .unwrap_or_else(|| "-".to_owned());
        format!("old {old_line} | new {new_line}")
    }

    #[qslot]
    fn diff_comment_bundle(&self, id: String) -> String {
        self.diff_comment_projection
            .as_ref()
            .and_then(|projection| projection.comment(id.as_str()))
            .map(hunk_domain::db::format_comment_clipboard_blob)
            .unwrap_or_default()
    }

    #[qslot]
    fn diff_all_open_comment_bundles(&self) -> String {
        self.diff_comment_projection
            .as_ref()
            .map(|projection| projection.all_open_clipboard_text())
            .unwrap_or_default()
    }

    #[qslot]
    fn apply_diff_comment_result(&mut self, epoch: i32) {
        let result = self
            .diff_comment_results
            .lock()
            .ok()
            .and_then(|mut pending| pending.remove(&epoch));
        if epoch != self.diff_comment_epoch {
            return;
        }

        self.diff_comments_loading = false;
        self.diff_comments_busy = false;
        let mut completed_kind = None;
        let mut applied = false;
        let mut stale_diff_projection = false;
        match result {
            Some(Ok(payload)) => {
                if self.active_diff_comment_scope().as_ref() != Some(&payload.projection.scope) {
                    return;
                }
                if payload.diff_epoch != self.diff_epoch {
                    if let Some(message) = payload.projection.status_message {
                        self.diff_comments_status_message = message;
                    }
                    stale_diff_projection = true;
                } else {
                    completed_kind = Some(payload.kind);
                    self.apply_diff_comment_projection(payload.projection);
                    applied = true;
                }
            }
            Some(Err(error)) => {
                self.diff_comments_error = error;
                self.diff_comments_ready = self.diff_comment_projection.is_some();
            }
            None => {
                self.diff_comments_error =
                    "Comment operation completed without a queued result".to_owned();
                self.diff_comments_ready = self.diff_comment_projection.is_some();
            }
        }
        self.diff_comments_state_changed();

        if stale_diff_projection {
            self.diff_comment_refresh_pending = false;
            self.refresh_diff_comments();
            return;
        }

        if self.diff_comment_refresh_pending {
            self.diff_comment_refresh_pending = false;
            self.refresh_diff_comments();
            return;
        }
        if applied
            && completed_kind != Some(DiffCommentRequestKind::Reconcile)
            && let Some(command) = self.next_diff_comment_reconcile_command()
        {
            if let Err(error) =
                self.start_diff_comment_command(DiffCommentRequestKind::Reconcile, command)
            {
                self.diff_comments_error = error;
            }
            self.diff_comments_state_changed();
        }
    }

    #[qslot]
    fn refresh_ai_threads(&mut self) {
        self.ensure_ai_runtime_started();
        self.send_ai_worker_command(AiWorkerCommand::RefreshThreads, "Refreshing Codex threads…");
    }

    #[qslot]
    fn select_ai_thread(&mut self, thread_id: String) {
        if thread_id.trim().is_empty() || thread_id == self.ai_active_thread_id {
            return;
        }
        self.ensure_ai_runtime_started();
        self.send_ai_worker_command(
            AiWorkerCommand::SelectThread { thread_id },
            "Opening Codex thread…",
        );
    }

    #[qslot]
    fn create_ai_thread(&mut self) {
        self.ensure_ai_runtime_started();
        self.send_ai_worker_command(
            AiWorkerCommand::StartThread {
                prompt: None,
                local_image_paths: Vec::new(),
                selected_skills: Vec::new(),
                skill_bindings: Vec::new(),
                session_overrides: AiTurnSessionOverrides::default(),
            },
            "Creating a Codex thread…",
        );
    }

    #[qslot]
    fn archive_ai_thread(&mut self, thread_id: String) {
        if thread_id.trim().is_empty() {
            return;
        }
        self.ensure_ai_runtime_started();
        self.send_ai_worker_command(
            AiWorkerCommand::ArchiveThread { thread_id },
            "Archiving Codex thread…",
        );
    }

    #[qslot]
    fn apply_ai_events(&mut self, epoch: i32) {
        let events = self.ai_runtime.mailbox.take(epoch);
        if events.is_empty() {
            return;
        }
        if apply_ai_runtime_events(self, events) {
            stop_ai_runtime(self);
        }
        self.ai_state_changed();
    }

    #[qslot]
    fn refresh_git_workspace(&mut self) {
        if self.git_loading || self.git_busy {
            return;
        }

        self.git_epoch = self.git_epoch.wrapping_add(1).max(1);
        let epoch = self.git_epoch;
        self.git_loading = true;
        self.git_error.clear();
        self.git_state_changed();

        let root = PathBuf::from(self.git_root.clone());
        let invoker = self.get_qml_method_invoker();
        let refresh_results = Arc::clone(&self.git_refresh_results);
        let spawn_result = std::thread::Builder::new()
            .name("hunk-qt-git-refresh".to_owned())
            .spawn(move || {
                let result = load_git_workspace(root.as_path())
                    .map(GitSnapshotPayload::from)
                    .map_err(|error| format!("{error:#}"));
                if let Ok(mut pending) = refresh_results.lock() {
                    pending.insert(epoch, result);
                }
                invoke_method!(invoker, "apply_git_snapshot", epoch);
            });

        if let Err(error) = spawn_result {
            self.git_loading = false;
            self.git_error = format!("Failed to start Git refresh: {error}");
            self.git_state_changed();
        }
    }

    #[qslot]
    fn select_git_root(&mut self, root: String) {
        if self.git_busy {
            self.git_error =
                "Wait for the current Git operation before switching repositories".to_owned();
            self.git_state_changed();
            return;
        }
        let root = match local_path_from_qml_folder_url(root.as_str()) {
            Ok(root) => root,
            Err(error) => {
                self.git_error = error;
                self.git_state_changed();
                return;
            }
        };
        if !root.is_dir() {
            self.git_error = format!("Repository folder does not exist: {}", root.display());
            self.git_state_changed();
            return;
        }
        if root == Path::new(self.git_root.as_str()) {
            return;
        }

        reset_ai_runtime_state(self);
        self.git_epoch = self.git_epoch.wrapping_add(1).max(1);
        self.git_loading = false;
        self.git_root = root.display().to_string();
        self.git_repository_name = root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.git_root.clone());
        self.git_branch_name.clear();
        self.git_branch_has_upstream = false;
        self.git_branch_ahead_count = 0;
        self.git_branch_behind_count = 0;
        self.git_changed_file_count = 0;
        self.git_staged_file_count = 0;
        self.git_unstaged_file_count = 0;
        self.git_last_commit_subject.clear();
        self.git_ready = false;
        self.git_error.clear();
        self.git_status_message.clear();
        self.git_root_pending_persist = true;
        self.git_staged_paths.clear();
        self.git_unstaged_paths.clear();
        self.git_files.borrow_mut().replace(Vec::new());
        self.git_branches.borrow_mut().replace(Vec::new());
        self.git_commits.borrow_mut().replace(Vec::new());
        self.reset_diff_state();
        self.reset_diff_comment_state();
        self.reset_forge_state();
        self.git_state_changed();
        self.ai_state_changed();
        self.diff_comments_state_changed();
        self.forge_state_changed();
        self.refresh_git_workspace();
    }

    #[qslot]
    fn apply_git_snapshot(&mut self, epoch: i32) {
        let result = self
            .git_refresh_results
            .lock()
            .ok()
            .and_then(|mut pending| pending.remove(&epoch));
        if epoch != self.git_epoch {
            return;
        }
        self.git_loading = false;
        let payload = match result {
            Some(Ok(payload)) => payload,
            Some(Err(error)) => {
                self.git_error = error;
                self.git_state_changed();
                return;
            }
            None => {
                self.git_error = "Git refresh completed without a queued result".to_owned();
                self.git_state_changed();
                return;
            }
        };
        self.apply_git_payload(payload);
    }

    #[qslot]
    fn stage_path(&mut self, path: String) {
        self.run_git_command("Staging file", GitWorkspaceCommand::StagePaths(vec![path]));
    }

    #[qslot]
    fn unstage_path(&mut self, path: String) {
        self.run_git_command(
            "Unstaging file",
            GitWorkspaceCommand::UnstagePaths(vec![path]),
        );
    }

    #[qslot]
    fn stage_all(&mut self) {
        self.run_git_command(
            "Staging files",
            GitWorkspaceCommand::StagePaths(self.git_unstaged_paths.clone()),
        );
    }

    #[qslot]
    fn unstage_all(&mut self) {
        self.run_git_command(
            "Unstaging files",
            GitWorkspaceCommand::UnstagePaths(self.git_staged_paths.clone()),
        );
    }

    #[qslot]
    fn discard_path(&mut self, path: String) {
        self.run_git_command(
            "Discarding changes",
            GitWorkspaceCommand::RestorePaths(vec![path]),
        );
    }

    #[qslot]
    fn commit_staged(&mut self, message: String) {
        self.run_git_command(
            "Creating commit",
            GitWorkspaceCommand::CommitStaged { message },
        );
    }

    #[qslot]
    fn activate_branch(&mut self, name: String) {
        self.run_git_command(
            "Activating branch",
            GitWorkspaceCommand::ActivateBranch { name },
        );
    }

    #[qslot]
    fn fetch_remote_branches(&mut self) {
        self.run_git_command(
            "Fetching branches",
            GitWorkspaceCommand::FetchRemoteBranches,
        );
    }

    #[qslot]
    fn publish_branch(&mut self) {
        self.run_git_command(
            "Publishing branch",
            GitWorkspaceCommand::PublishBranch {
                name: self.git_branch_name.clone(),
            },
        );
    }

    #[qslot]
    fn push_branch(&mut self) {
        self.run_git_command(
            "Pushing branch",
            GitWorkspaceCommand::PushBranch {
                name: self.git_branch_name.clone(),
            },
        );
    }

    #[qslot]
    fn sync_branch(&mut self) {
        self.run_git_command(
            "Syncing branch",
            GitWorkspaceCommand::SyncBranch {
                name: self.git_branch_name.clone(),
            },
        );
    }

    #[qslot]
    fn pull_branch_with_rebase(&mut self) {
        self.run_git_command(
            "Rebasing branch",
            GitWorkspaceCommand::PullBranchWithRebase {
                name: self.git_branch_name.clone(),
            },
        );
    }

    #[qslot]
    fn refresh_forge_review(&mut self) {
        if !self.git_ready || self.forge_loading || self.forge_busy {
            return;
        }

        let epoch = self.next_forge_epoch();
        self.forge_loading = true;
        self.forge_error.clear();
        self.forge_status_message.clear();
        self.forge_state_changed();

        let root = PathBuf::from(self.git_root.clone());
        let branch = self.git_branch_name.clone();
        let invoker = self.get_qml_method_invoker();
        let results = Arc::clone(&self.forge_results);
        let spawn_result = std::thread::Builder::new()
            .name("hunk-qt-forge-refresh".to_owned())
            .spawn(move || {
                let result = load_forge_snapshot(root.as_path(), branch.as_str())
                    .map(Box::new)
                    .map(ForgeAsyncPayload::Snapshot)
                    .map_err(|error| format!("{error:#}"));
                if let Ok(mut pending) = results.lock() {
                    pending.insert(epoch, result);
                }
                invoke_method!(invoker, "apply_forge_result", epoch);
            });

        if let Err(error) = spawn_result {
            self.forge_loading = false;
            self.forge_error = format!("Failed to start review refresh: {error}");
            self.forge_state_changed();
        }
    }

    #[qslot]
    fn save_forge_personal_access_token(&mut self, token: String) {
        let Some(workspace) = self.forge_context.clone() else {
            self.forge_error = "No forge repository is available for this branch".to_owned();
            self.forge_state_changed();
            return;
        };
        let token = token.trim().to_string();
        if token.is_empty() {
            self.forge_error = "Access token is required".to_owned();
            self.forge_state_changed();
            return;
        }

        self.run_save_forge_token(
            "Saving credential",
            workspace,
            token,
            ForgeCredentialKind::PersonalAccessToken,
        );
    }

    #[qslot]
    fn create_forge_review(
        &mut self,
        target_branch: String,
        title: String,
        body: String,
        draft: bool,
    ) {
        if self.forge_loading || self.forge_busy {
            return;
        }
        let Some(workspace) = self.forge_context.clone() else {
            self.forge_error = "No forge repository is available for this branch".to_owned();
            self.forge_state_changed();
            return;
        };
        let Some(token) = self.forge_token.clone() else {
            self.forge_error = format!("{} authentication is required", self.forge_provider_label);
            self.forge_state_changed();
            return;
        };

        let epoch = self.begin_forge_action(format!(
            "Creating or finding {}",
            self.forge_review_kind_label
        ));
        let invoker = self.get_qml_method_invoker();
        let results = Arc::clone(&self.forge_results);
        let spawn_result = std::thread::Builder::new()
            .name("hunk-qt-forge-review".to_owned())
            .spawn(move || {
                let result = create_or_find_review(
                    &workspace,
                    token.as_str(),
                    target_branch.as_str(),
                    title.as_str(),
                    (!body.trim().is_empty()).then_some(body),
                    draft,
                )
                .map(ForgeAsyncPayload::Review)
                .map_err(|error| format!("{error:#}"));
                if let Ok(mut pending) = results.lock() {
                    pending.insert(epoch, result);
                }
                invoke_method!(invoker, "apply_forge_result", epoch);
            });
        if let Err(error) = spawn_result {
            self.fail_forge_spawn("review operation", error);
        }
    }

    #[qslot]
    fn start_github_device_flow(&mut self) {
        if self.forge_loading || self.forge_busy {
            return;
        }
        let Some(workspace) = self.forge_context.clone() else {
            self.forge_error = "No GitHub repository is available for this branch".to_owned();
            self.forge_state_changed();
            return;
        };
        if workspace.base_repo.provider != ForgeProvider::GitHub || self.forge_auth_mode != "device"
        {
            self.forge_error = "GitHub device sign-in is only available for github.com".to_owned();
            self.forge_state_changed();
            return;
        }

        let epoch = self.begin_forge_action("Starting GitHub sign-in".to_owned());
        let current_epoch = Arc::clone(&self.forge_current_epoch);
        let start_results = Arc::clone(&self.forge_device_start_results);
        let final_results = Arc::clone(&self.forge_results);
        let invoker = self.get_qml_method_invoker();
        let spawn_result = std::thread::Builder::new()
            .name("hunk-qt-github-auth".to_owned())
            .spawn(move || {
                let result =
                    run_github_device_flow(workspace, epoch, current_epoch, |start_result| {
                        if let Ok(mut pending) = start_results.lock() {
                            pending.insert(epoch, start_result);
                        }
                        invoke_method!(invoker, "apply_github_device_authorization", epoch);
                    });
                let Some(result) = result else {
                    return;
                };
                if let Ok(mut pending) = final_results.lock() {
                    pending.insert(epoch, result.map(Box::new).map(ForgeAsyncPayload::Snapshot));
                }
                invoke_method!(invoker, "apply_forge_result", epoch);
            });
        if let Err(error) = spawn_result {
            self.fail_forge_spawn("GitHub sign-in", error);
        }
    }

    #[qslot]
    fn cancel_github_device_flow(&mut self) {
        if !self.forge_device_flow_active && !self.forge_busy {
            return;
        }
        self.next_forge_epoch();
        self.forge_busy = false;
        self.forge_action_label.clear();
        self.forge_device_flow_active = false;
        self.forge_device_user_code.clear();
        self.forge_device_verification_url.clear();
        self.forge_status_message = "GitHub sign-in cancelled".to_owned();
        self.forge_state_changed();
    }

    #[qslot]
    fn apply_github_device_authorization(&mut self, epoch: i32) {
        let result = self
            .forge_device_start_results
            .lock()
            .ok()
            .and_then(|mut pending| pending.remove(&epoch));
        if epoch != self.forge_epoch {
            return;
        }
        match result {
            Some(Ok(authorization)) => {
                self.forge_device_flow_active = true;
                self.forge_device_user_code = authorization.user_code;
                self.forge_device_verification_url = authorization.verification_uri;
                self.forge_action_label = "Waiting for GitHub authorization".to_owned();
                self.forge_status_message =
                    "Enter the displayed code in GitHub to finish sign-in".to_owned();
            }
            Some(Err(error)) => {
                self.forge_busy = false;
                self.forge_action_label.clear();
                self.forge_error = error;
            }
            None => {
                self.forge_busy = false;
                self.forge_action_label.clear();
                self.forge_error =
                    "GitHub sign-in started without a queued authorization".to_owned();
            }
        }
        self.forge_state_changed();
    }

    #[qslot]
    fn apply_forge_result(&mut self, epoch: i32) {
        let result = self
            .forge_results
            .lock()
            .ok()
            .and_then(|mut pending| pending.remove(&epoch));
        if epoch != self.forge_epoch {
            return;
        }

        self.forge_loading = false;
        self.forge_busy = false;
        self.forge_action_label.clear();
        self.forge_device_flow_active = false;
        self.forge_device_user_code.clear();
        self.forge_device_verification_url.clear();
        match result {
            Some(Ok(ForgeAsyncPayload::Snapshot(payload))) => {
                self.apply_forge_payload(*payload);
            }
            Some(Ok(ForgeAsyncPayload::Review(outcome))) => {
                let action = if outcome.existed { "Using" } else { "Created" };
                let short_label = review_short_label(outcome.review.provider);
                self.forge_status_message = format!(
                    "{action} {short_label} #{} for {}",
                    outcome.review.number, outcome.review.source_branch
                );
                self.apply_review_summary(Some(outcome.review));
            }
            Some(Err(error)) => {
                self.forge_ready = true;
                self.forge_error = error;
            }
            None => {
                self.forge_ready = true;
                self.forge_error = "Forge operation completed without a queued result".to_owned();
            }
        }
        self.forge_state_changed();
    }

    #[qslot]
    fn complete_git_command(&mut self, success: bool, message: String) {
        self.git_busy = false;
        self.git_action_label.clear();
        if success {
            self.git_status_message = message;
            self.git_state_changed();
            self.refresh_git_workspace();
        } else {
            self.git_error = message;
            self.git_state_changed();
        }
    }

    fn ensure_ai_runtime_started(&mut self) {
        let workspace_key = self.git_root.clone();
        if self
            .ai_runtime
            .session
            .as_ref()
            .is_some_and(|runtime| runtime.workspace_key() == workspace_key.as_str())
        {
            return;
        }
        if !self.git_ready {
            self.ai_loading = true;
            self.ai_connection_state = "waiting".to_owned();
            self.ai_status_message = "Waiting for the repository to load…".to_owned();
            self.ai_state_changed();
            return;
        }

        reset_ai_runtime_state(self);
        self.ai_workspace_root = workspace_key.clone();
        let config = match prepare_ai_worker_config(Path::new(workspace_key.as_str())) {
            Ok(config) => config,
            Err(error) => {
                self.ai_connection_state = "failed".to_owned();
                self.ai_error = error.clone();
                self.ai_status_message = error;
                self.ai_state_changed();
                return;
            }
        };
        let starting_status_message = config.starting_status_message();
        let epoch = self.ai_epoch;
        let mailbox = Arc::clone(&self.ai_runtime.mailbox);
        let invoker = self.get_qml_method_invoker();
        let start_result = start_ai_runtime(config, epoch, mailbox, move |event_epoch| {
            invoke_method!(invoker, "apply_ai_events", event_epoch);
        });

        match start_result {
            Ok(runtime) => {
                self.ai_runtime.session = Some(runtime);
                self.ai_loading = true;
                self.ai_connection_state = "connecting".to_owned();
                self.ai_status_message = starting_status_message;
            }
            Err(error) => {
                self.ai_connection_state = "failed".to_owned();
                self.ai_error = error.clone();
                self.ai_status_message = error;
            }
        }
        self.ai_state_changed();
    }

    fn send_ai_worker_command(&mut self, command: AiWorkerCommand, status_message: &str) {
        let Some(runtime) = self.ai_runtime.session.as_ref() else {
            return;
        };
        let result = runtime.send(command);
        match result {
            Ok(()) => {
                self.ai_error.clear();
                self.ai_status_message = status_message.to_owned();
            }
            Err(error) => {
                stop_ai_runtime(self);
                self.ai_ready = false;
                self.ai_loading = false;
                self.ai_connection_state = "failed".to_owned();
                self.ai_error = error.clone();
                self.ai_status_message = error;
            }
        }
        self.ai_state_changed();
    }

    fn set_status_message(&mut self, status_message: String) {
        if self.status_message == status_message {
            return;
        }
        self.status_message = status_message;
        self.status_message_changed();
    }

    fn diff_files(&self) -> Rc<RefCell<GitFileListModel>> {
        self.diff_files.clone()
    }

    fn diff_rows(&self) -> Rc<RefCell<DiffRowListModel>> {
        self.diff_rows.clone()
    }

    fn diff_comments(&self) -> Rc<RefCell<DiffCommentListModel>> {
        self.diff_comments.clone()
    }

    fn git_files(&self) -> Rc<RefCell<GitFileListModel>> {
        self.git_files.clone()
    }

    fn git_branches(&self) -> Rc<RefCell<GitBranchListModel>> {
        self.git_branches.clone()
    }

    fn git_commits(&self) -> Rc<RefCell<GitCommitListModel>> {
        self.git_commits.clone()
    }

    fn ai_threads(&self) -> Rc<RefCell<AiThreadListModel>> {
        self.ai_threads.clone()
    }

    fn ai_timeline(&self) -> Rc<RefCell<AiTimelineListModel>> {
        self.ai_timeline.clone()
    }

    fn apply_git_payload(&mut self, payload: GitSnapshotPayload) {
        let diff_files = payload.diff_files;
        let diff_file_summaries = payload.diff_file_summaries;
        let comment_scope_changed =
            self.git_root != payload.root || self.git_branch_name != payload.branch_name;
        let forge_context_changed = self.git_root != payload.root
            || self.git_branch_name != payload.branch_name
            || !self.forge_ready;
        self.git_staged_paths = payload
            .files
            .iter()
            .filter(|file| file.staged)
            .map(|file| file.path.clone())
            .collect();
        self.git_unstaged_paths = payload
            .files
            .iter()
            .filter(|file| !file.staged)
            .map(|file| file.path.clone())
            .collect();
        self.git_files.borrow_mut().replace(payload.files);
        self.git_branches.borrow_mut().replace(payload.branches);
        self.git_commits.borrow_mut().replace(payload.commits);
        self.git_root = payload.root;
        self.git_repository_name = payload.repository_name;
        self.git_branch_name = payload.branch_name;
        self.git_branch_has_upstream = payload.branch_has_upstream;
        self.git_branch_ahead_count = payload.branch_ahead_count;
        self.git_branch_behind_count = payload.branch_behind_count;
        self.git_changed_file_count = payload.changed_file_count;
        self.git_staged_file_count = payload.staged_file_count;
        self.git_unstaged_file_count = payload.unstaged_file_count;
        self.git_last_commit_subject = payload.last_commit_subject;
        self.git_ready = true;
        self.git_error.clear();
        if comment_scope_changed {
            self.reset_diff_comment_state();
            self.diff_comments_state_changed();
        }
        self.replace_diff_files(diff_files, diff_file_summaries);
        if self.git_root_pending_persist {
            self.git_root_pending_persist = false;
            if let Err(error) = persist_active_project(PathBuf::from(self.git_root.as_str())) {
                self.git_status_message =
                    format!("Repository loaded; failed to save selection: {error:#}");
            }
        }
        self.git_state_changed();
        self.refresh_diff();
        self.refresh_diff_comments();
        if forge_context_changed {
            self.reset_forge_state();
            self.forge_state_changed();
            self.refresh_forge_review();
        }
        if self.active_workspace == Workspace::Ai.as_str() {
            self.ensure_ai_runtime_started();
        }
    }

    fn replace_diff_files(
        &mut self,
        files: Vec<crate::git_models::GitFileItem>,
        summaries: Vec<DiffFileSummary>,
    ) {
        self.diff_epoch = self.diff_epoch.wrapping_add(1).max(1);
        let previous_path = self.diff_selected_path.clone();
        self.diff_loading = false;
        self.diff_ready = false;
        self.diff_error.clear();
        self.diff_rows.borrow_mut().replace(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Arc::new(Vec::new()),
        );
        self.clear_diff_comment_row_state();
        self.clear_diff_search_results();
        self.diff_files.borrow_mut().replace(files);
        self.diff_file_summaries = summaries
            .into_iter()
            .map(|summary| (summary.path.clone(), summary))
            .collect();

        let selected = self
            .diff_file_summaries
            .get(previous_path.as_str())
            .cloned()
            .or_else(|| {
                self.diff_file_summaries
                    .values()
                    .min_by(|left, right| left.path.cmp(&right.path))
                    .cloned()
            });
        if let Some(summary) = selected {
            self.apply_diff_selection(&summary);
        } else {
            self.diff_selected_path.clear();
            self.diff_status_tag.clear();
            self.diff_additions = 0;
            self.diff_removals = 0;
            self.diff_ready = true;
            self.diff_state_changed();
        }
        self.diff_comments_state_changed();
    }

    fn apply_diff_selection(&mut self, summary: &DiffFileSummary) {
        self.diff_epoch = self.diff_epoch.wrapping_add(1).max(1);
        self.diff_loading = false;
        self.diff_rows.borrow_mut().replace(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Arc::new(Vec::new()),
        );
        self.clear_diff_comment_row_state();
        self.clear_diff_search_results();
        self.diff_selected_path = summary.path.clone();
        self.diff_status_tag = summary.status.tag().to_owned();
        self.diff_additions = i32::try_from(summary.line_stats.added).unwrap_or(i32::MAX);
        self.diff_removals = i32::try_from(summary.line_stats.removed).unwrap_or(i32::MAX);
        self.diff_ready = false;
        self.diff_error.clear();
        self.diff_state_changed();
        self.diff_comments_state_changed();
    }

    fn reset_diff_state(&mut self) {
        self.diff_epoch = self.diff_epoch.wrapping_add(1).max(1);
        self.diff_files.borrow_mut().replace(Vec::new());
        self.diff_rows.borrow_mut().replace(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Arc::new(Vec::new()),
        );
        self.clear_diff_comment_row_state();
        self.diff_selected_path.clear();
        self.diff_status_tag.clear();
        self.diff_additions = 0;
        self.diff_removals = 0;
        self.diff_ready = false;
        self.diff_loading = false;
        self.diff_error.clear();
        self.diff_file_summaries.clear();
        self.diff_search_query.clear();
        self.clear_diff_search_results();
        self.diff_state_changed();
        self.diff_comments_state_changed();
    }

    fn rebuild_diff_search_results(&mut self) {
        self.diff_search_matches = self
            .diff_rows
            .borrow()
            .matching_rows(self.diff_search_query.as_str());
        self.diff_search_match_count =
            i32::try_from(self.diff_search_matches.len()).unwrap_or(i32::MAX);
        if let Some(target) = self.diff_search_matches.first().copied() {
            self.diff_search_match_index = 0;
            self.diff_search_target_row = i32::try_from(target).unwrap_or(i32::MAX);
        } else {
            self.diff_search_match_index = -1;
            self.diff_search_target_row = -1;
        }
    }

    fn clear_diff_search_results(&mut self) {
        self.diff_search_matches.clear();
        self.diff_search_match_count = 0;
        self.diff_search_match_index = -1;
        self.diff_search_target_row = -1;
    }

    fn next_forge_epoch(&mut self) -> i32 {
        self.forge_epoch = self.forge_epoch.wrapping_add(1).max(1);
        self.forge_current_epoch
            .store(self.forge_epoch, Ordering::Release);
        self.forge_epoch
    }

    fn begin_forge_action(&mut self, label: String) -> i32 {
        let epoch = self.next_forge_epoch();
        self.forge_busy = true;
        self.forge_error.clear();
        self.forge_status_message.clear();
        self.forge_action_label = label;
        self.forge_state_changed();
        epoch
    }

    fn run_save_forge_token(
        &mut self,
        label: &str,
        workspace: ForgeReviewWorkspace,
        token: String,
        kind: ForgeCredentialKind,
    ) {
        if self.forge_loading || self.forge_busy {
            return;
        }
        let epoch = self.begin_forge_action(label.to_owned());
        let invoker = self.get_qml_method_invoker();
        let results = Arc::clone(&self.forge_results);
        let spawn_result = std::thread::Builder::new()
            .name("hunk-qt-forge-credential".to_owned())
            .spawn(move || {
                let result = save_forge_token(workspace, token.as_str(), kind)
                    .map(Box::new)
                    .map(ForgeAsyncPayload::Snapshot)
                    .map_err(|error| format!("{error:#}"));
                if let Ok(mut pending) = results.lock() {
                    pending.insert(epoch, result);
                }
                invoke_method!(invoker, "apply_forge_result", epoch);
            });
        if let Err(error) = spawn_result {
            self.fail_forge_spawn("credential operation", error);
        }
    }

    fn fail_forge_spawn(&mut self, operation: &str, error: std::io::Error) {
        self.forge_loading = false;
        self.forge_busy = false;
        self.forge_action_label.clear();
        self.forge_error = format!("Failed to start {operation}: {error}");
        self.forge_state_changed();
    }

    fn apply_forge_payload(&mut self, payload: ForgeSnapshotPayload) {
        let provider = payload.workspace.base_repo.provider;
        let authenticated = payload.authenticated();
        let auth_mode = payload.auth_mode().to_owned();
        self.forge_available = true;
        self.forge_provider_label = provider_label(provider).to_owned();
        self.forge_review_kind_label = review_kind_label(provider).to_owned();
        self.forge_host = payload.workspace.base_repo.host.clone();
        self.forge_repository_path = payload.workspace.base_repo.path.clone();
        self.forge_authenticated = authenticated;
        self.forge_account_label = payload.account_label;
        self.forge_auth_mode = auth_mode;
        self.forge_default_target_branch = payload.workspace.target_branch.clone();
        self.forge_context = Some(payload.workspace);
        self.forge_token = payload.token;
        self.forge_ready = true;
        self.forge_error.clear();
        if self.forge_authenticated {
            self.forge_status_message = format!("{} connected", self.forge_provider_label);
        } else {
            self.forge_status_message.clear();
        }
        self.apply_review_summary(payload.review);
    }

    fn apply_review_summary(&mut self, review: Option<hunk_forge::OpenReviewSummary>) {
        let Some(review) = review else {
            self.forge_review_exists = false;
            self.forge_review_number = 0;
            self.forge_review_title.clear();
            self.forge_review_url.clear();
            self.forge_review_state.clear();
            self.forge_review_draft = false;
            return;
        };
        let state_label = review_state_label(&review).to_owned();
        self.forge_review_exists = true;
        self.forge_review_number = i32::try_from(review.number).unwrap_or(i32::MAX);
        self.forge_review_title = review.title;
        self.forge_review_url = review.url;
        self.forge_review_state = state_label;
        self.forge_review_draft = review.draft;
    }

    fn reset_forge_state(&mut self) {
        self.next_forge_epoch();
        self.forge_available = false;
        self.forge_provider_label.clear();
        self.forge_review_kind_label.clear();
        self.forge_host.clear();
        self.forge_repository_path.clear();
        self.forge_authenticated = false;
        self.forge_account_label.clear();
        self.forge_auth_mode.clear();
        self.forge_ready = false;
        self.forge_loading = false;
        self.forge_busy = false;
        self.forge_error.clear();
        self.forge_status_message.clear();
        self.forge_action_label.clear();
        self.forge_default_target_branch.clear();
        self.apply_review_summary(None);
        self.forge_device_flow_active = false;
        self.forge_device_user_code.clear();
        self.forge_device_verification_url.clear();
        self.forge_context = None;
        self.forge_token = None;
    }

    fn run_git_command(&mut self, label: &str, command: GitWorkspaceCommand) {
        if self.git_loading || self.git_busy {
            return;
        }
        self.git_busy = true;
        self.git_error.clear();
        self.git_action_label = label.to_owned();
        self.git_state_changed();

        let root = PathBuf::from(self.git_root.clone());
        let invoker = self.get_qml_method_invoker();
        let spawn_result = std::thread::Builder::new()
            .name("hunk-qt-git-command".to_owned())
            .spawn(move || {
                let result = execute_git_workspace_command(root.as_path(), command);
                let (success, message) = match result {
                    Ok(outcome) => (true, outcome.message),
                    Err(error) => (false, format!("{error:#}")),
                };
                invoke_method!(
                    invoker,
                    "complete_git_command",
                    success,
                    QString::from(message)
                );
            });

        if let Err(error) = spawn_result {
            self.git_busy = false;
            self.git_action_label.clear();
            self.git_error = format!("Failed to start Git command: {error}");
            self.git_state_changed();
        }
    }
}

fn persist_active_project(root: PathBuf) -> anyhow::Result<()> {
    let store = AppStateStore::new()?;
    let mut state = store.load_or_default()?;
    state.activate_workspace_project(root);
    store.save(&state)
}
