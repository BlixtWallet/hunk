use std::path::{Path, PathBuf};
use std::sync::Arc;

use hunk_app::ai::AiWorkerCommand;
use hunk_app::diff::DiffCommentStoreCommand;
use hunk_domain::db::CommentStatus;
use hunk_forge::{ForgeCredentialKind, ForgeProvider};
use hunk_git::workspace::{GitWorkspaceCommand, load_git_workspace};
use qtbridge::{QObjectHolder, invoke_method, qobject, qtbridge_type_lib::QString};

use crate::ai_attachments::{
    complete_ai_attachment_add, queue_ai_attachments, remove_ai_attachment,
};
use crate::ai_bookmarks::{complete_ai_bookmark_persist, queue_ai_toggle_thread_bookmark};
use crate::ai_session::{
    complete_ai_session_persist, queue_ai_select_collaboration_mode, queue_ai_select_effort,
    queue_ai_select_model, queue_ai_select_service_tier, queue_ai_set_mad_max_mode,
};
use crate::backend_ai::{
    apply_ai_runtime_events, clear_ai_message_queue, edit_last_ai_queued_prompt,
    ensure_ai_runtime_started, queue_ai_approval, queue_ai_archive_thread, queue_ai_create_thread,
    queue_ai_follow_up, queue_ai_fork_thread, queue_ai_interrupt, queue_ai_prompt,
    queue_ai_select_thread, queue_ai_user_input, reset_ai_runtime_state, send_ai_worker_command,
    stop_ai_runtime, take_ai_recovered_prompt,
};
pub use crate::backend_state::{Backend, Workspace};
use crate::backend_state::{DiffCommentRequestKind, ForgeAsyncPayload, next_forge_epoch};
use crate::comments::DiffCommentStartOutcome;
use crate::diff_models::DiffSnapshotPayload;
use crate::forge::{
    create_or_find_review, load_forge_snapshot, review_short_label, run_github_device_flow,
};
use crate::git_models::GitSnapshotPayload;
use crate::local_path_from_qml_folder_url;

#[qobject]
impl Backend {
    qproperty!(
        "activeWorkspace",
        Member = active_workspace,
        Notify = active_workspace_changed
    );
    qproperty!("diffFiles", Member = diff_files, Constant);
    qproperty!("diffRows", Member = diff_rows, Constant);
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
    qproperty!("diffComments", Member = diff_comments, Constant);
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
    qproperty!("gitFiles", Member = git_files, Constant);
    qproperty!("gitBranches", Member = git_branches, Constant);
    qproperty!("gitCommits", Member = git_commits, Constant);
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
    qproperty!("aiThreads", Member = ai_threads, Constant);
    qproperty!("aiTimeline", Member = ai_timeline, Constant);
    qproperty!("aiAttachments", Member = ai_attachments, Constant);
    qproperty!(
        "aiAttachmentPending",
        Read = ai_attachment_pending,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiModelSupportsImageInputs",
        Read = ai_model_supports_image_inputs,
        Notify = ai_session_state_changed
    );
    qproperty!("aiModels", Member = ai_models, Constant);
    qproperty!("aiEfforts", Member = ai_efforts, Constant);
    qproperty!("aiServiceTiers", Member = ai_service_tiers, Constant);
    qproperty!(
        "aiSelectedModelIndex",
        Read = ai_selected_model_index,
        Notify = ai_session_state_changed
    );
    qproperty!(
        "aiSelectedEffortIndex",
        Read = ai_selected_effort_index,
        Notify = ai_session_state_changed
    );
    qproperty!(
        "aiSelectedServiceTierIndex",
        Read = ai_selected_service_tier_index,
        Notify = ai_session_state_changed
    );
    qproperty!(
        "aiSelectedModelLabel",
        Read = ai_selected_model_label,
        Notify = ai_session_state_changed
    );
    qproperty!(
        "aiSelectedEffortLabel",
        Read = ai_selected_effort_label,
        Notify = ai_session_state_changed
    );
    qproperty!(
        "aiSelectedCollaborationMode",
        Member = ai_selected_collaboration_mode,
        Notify = ai_session_state_changed
    );
    qproperty!(
        "aiSelectedCollaborationLabel",
        Read = ai_selected_collaboration_label,
        Notify = ai_session_state_changed
    );
    qproperty!(
        "aiSelectedServiceTierLabel",
        Read = ai_selected_service_tier_label,
        Notify = ai_session_state_changed
    );
    qproperty!(
        "aiMadMaxMode",
        Member = ai_mad_max_mode,
        Notify = ai_session_state_changed
    );
    qproperty!(
        "aiApprovalPolicyLabel",
        Read = ai_approval_policy_label,
        Notify = ai_session_state_changed
    );
    qproperty!(
        "aiEffortOptionCount",
        Read = ai_effort_option_count,
        Notify = ai_session_state_changed
    );
    qproperty!(
        "aiSessionControlsLocked",
        Read = ai_session_controls_locked,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiContextAvailable",
        Read = ai_context_available,
        Notify = ai_session_state_changed
    );
    qproperty!(
        "aiContextPercentUsed",
        Read = ai_context_percent_used,
        Notify = ai_session_state_changed
    );
    qproperty!(
        "aiContextPercentLeft",
        Read = ai_context_percent_left,
        Notify = ai_session_state_changed
    );
    qproperty!(
        "aiContextTokenSummary",
        Read = ai_context_token_summary,
        Notify = ai_session_state_changed
    );
    qproperty!(
        "aiContextInputTokens",
        Read = ai_context_input_tokens,
        Notify = ai_session_state_changed
    );
    qproperty!(
        "aiContextCachedInputTokens",
        Read = ai_context_cached_input_tokens,
        Notify = ai_session_state_changed
    );
    qproperty!(
        "aiContextOutputTokens",
        Read = ai_context_output_tokens,
        Notify = ai_session_state_changed
    );
    qproperty!(
        "aiContextReasoningTokens",
        Read = ai_context_reasoning_tokens,
        Notify = ai_session_state_changed
    );
    qproperty!(
        "aiContextBillableTokens",
        Read = ai_context_billable_tokens,
        Notify = ai_session_state_changed
    );
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
        "aiActiveTurnId",
        Member = ai_active_turn_id,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiTurnRunning",
        Member = ai_turn_running,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiThreadActionPending",
        Read = ai_thread_action_pending,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiPromptPending",
        Read = ai_prompt_pending,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiPromptAcceptedRevision",
        Member = ai_prompt_accepted_revision,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiQueuedMessageCount",
        Read = ai_queued_message_count,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiActiveQueuedMessageCount",
        Read = ai_active_queued_message_count,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiActiveQueueSending",
        Read = ai_active_queue_sending,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiInterruptPending",
        Read = ai_interrupt_pending,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiPendingRequestCount",
        Read = ai_pending_request_count,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiActiveRequestCount",
        Read = ai_active_request_count,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiRequestId",
        Read = ai_request_id,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiRequestKind",
        Read = ai_request_kind,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiRequestTitle",
        Read = ai_request_title,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiRequestDescription",
        Read = ai_request_description,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiRequestReason",
        Read = ai_request_reason,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiRequestQuestionsJson",
        Read = ai_request_questions_json,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiRequestAnswerable",
        Read = ai_request_answerable,
        Notify = ai_state_changed
    );
    qproperty!(
        "aiRequestResolving",
        Read = ai_request_resolving,
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
    pub(super) fn diff_state_changed(&mut self);

    #[qsignal]
    pub(super) fn diff_comments_state_changed(&mut self);

    #[qsignal]
    pub(super) fn git_state_changed(&mut self);

    #[qsignal]
    pub(super) fn ai_state_changed(&mut self);

    #[qsignal]
    pub(super) fn ai_session_state_changed(&mut self);

    #[qsignal]
    pub(super) fn forge_state_changed(&mut self);

    fn ai_selected_model_index(&self) -> i32 {
        self.ai_selected_model_index_value()
    }

    fn ai_selected_effort_index(&self) -> i32 {
        self.ai_selected_effort_index_value()
    }

    fn ai_selected_service_tier_index(&self) -> i32 {
        self.ai_selected_service_tier_index_value()
    }

    fn ai_selected_model_label(&self) -> String {
        self.ai_selected_model_label_value()
    }

    fn ai_selected_effort_label(&self) -> String {
        self.ai_selected_effort_label_value()
    }

    fn ai_selected_collaboration_label(&self) -> String {
        self.ai_selected_collaboration_label_value()
    }

    fn ai_selected_service_tier_label(&self) -> String {
        self.ai_selected_service_tier_label_value()
    }

    fn ai_approval_policy_label(&self) -> String {
        self.ai_approval_policy_label_value()
    }

    fn ai_effort_option_count(&self) -> i32 {
        self.ai_effort_option_count_value()
    }

    fn ai_session_controls_locked(&self) -> bool {
        self.ai_session_controls_locked_value()
    }

    fn ai_context_available(&self) -> bool {
        self.ai_context_available_value()
    }

    fn ai_context_percent_used(&self) -> i32 {
        self.ai_context_percent_used_value()
    }

    fn ai_context_percent_left(&self) -> i32 {
        self.ai_context_percent_left_value()
    }

    fn ai_context_token_summary(&self) -> String {
        self.ai_context_token_summary_value()
    }

    fn ai_context_input_tokens(&self) -> String {
        self.ai_context_input_tokens_value()
    }

    fn ai_context_cached_input_tokens(&self) -> String {
        self.ai_context_cached_input_tokens_value()
    }

    fn ai_context_output_tokens(&self) -> String {
        self.ai_context_output_tokens_value()
    }

    fn ai_context_reasoning_tokens(&self) -> String {
        self.ai_context_reasoning_tokens_value()
    }

    fn ai_context_billable_tokens(&self) -> String {
        self.ai_context_billable_tokens_value()
    }

    fn ai_attachment_pending(&self) -> bool {
        self.ai_attachment_pending_value()
    }

    fn ai_model_supports_image_inputs(&self) -> bool {
        self.ai_model_supports_image_inputs_value()
    }

    fn ai_thread_action_pending(&self) -> bool {
        self.ai_thread_action_pending_value()
    }

    fn ai_prompt_pending(&self) -> bool {
        self.ai_prompt_pending_value()
    }

    fn ai_queued_message_count(&self) -> i32 {
        self.ai_queued_message_count_value()
    }

    fn ai_active_queued_message_count(&self) -> i32 {
        self.ai_active_queued_message_count_value()
    }

    fn ai_active_queue_sending(&self) -> bool {
        self.ai_active_queue_sending_value()
    }

    fn ai_interrupt_pending(&self) -> bool {
        self.ai_interrupt_pending_value()
    }

    fn ai_pending_request_count(&self) -> i32 {
        self.ai_pending_request_count_value()
    }

    fn ai_active_request_count(&self) -> i32 {
        self.ai_active_request_count_value()
    }

    fn ai_request_id(&self) -> String {
        self.ai_request_id_value()
    }

    fn ai_request_kind(&self) -> String {
        self.ai_request_kind_value()
    }

    fn ai_request_title(&self) -> String {
        self.ai_request_title_value()
    }

    fn ai_request_description(&self) -> String {
        self.ai_request_description_value()
    }

    fn ai_request_reason(&self) -> String {
        self.ai_request_reason_value()
    }

    fn ai_request_questions_json(&self) -> String {
        self.ai_request_questions_json_value()
    }

    fn ai_request_answerable(&self) -> bool {
        self.ai_request_answerable_value()
    }

    fn ai_request_resolving(&self) -> bool {
        self.ai_request_resolving_value()
    }

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
    pub(super) fn refresh_diff(&mut self) {
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
    pub(super) fn refresh_diff_comments(&mut self) {
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
        self.ensure_ai_runtime_started();
        let _ = queue_ai_select_thread(self, thread_id);
        self.ai_state_changed();
    }

    #[qslot]
    fn create_ai_thread(&mut self) {
        self.ensure_ai_runtime_started();
        let _ = queue_ai_create_thread(self);
        self.ai_state_changed();
    }

    #[qslot]
    fn fork_ai_thread(&mut self) -> bool {
        self.ensure_ai_runtime_started();
        let queued = queue_ai_fork_thread(self);
        self.ai_state_changed();
        queued
    }

    #[qslot]
    fn archive_ai_thread(&mut self, thread_id: String) {
        self.ensure_ai_runtime_started();
        let _ = queue_ai_archive_thread(self, thread_id);
        self.ai_state_changed();
    }

    #[qslot]
    fn toggle_ai_thread_bookmark(&mut self, thread_id: String) -> bool {
        let changed = queue_ai_toggle_thread_bookmark(self, thread_id);
        self.ai_state_changed();
        changed
    }

    #[qslot]
    fn complete_ai_bookmark_persist(&mut self, epoch: i32) {
        complete_ai_bookmark_persist(self, epoch);
        self.ai_state_changed();
    }

    #[qslot]
    fn select_ai_model(&mut self, index: i32) -> bool {
        let changed = queue_ai_select_model(self, index);
        self.ai_session_state_changed();
        self.ai_state_changed();
        changed
    }

    #[qslot]
    fn select_ai_effort(&mut self, index: i32) -> bool {
        let changed = queue_ai_select_effort(self, index);
        self.ai_session_state_changed();
        self.ai_state_changed();
        changed
    }

    #[qslot]
    fn select_ai_collaboration_mode(&mut self, mode: String) -> bool {
        let changed = queue_ai_select_collaboration_mode(self, mode);
        self.ai_session_state_changed();
        self.ai_state_changed();
        changed
    }

    #[qslot]
    fn select_ai_service_tier(&mut self, index: i32) -> bool {
        let changed = queue_ai_select_service_tier(self, index);
        self.ai_session_state_changed();
        self.ai_state_changed();
        changed
    }

    #[qslot]
    fn set_ai_mad_max_mode(&mut self, enabled: bool) -> bool {
        let changed = queue_ai_set_mad_max_mode(self, enabled);
        self.ai_session_state_changed();
        self.ai_state_changed();
        changed
    }

    #[qslot]
    fn complete_ai_session_persist(&mut self, epoch: i32) {
        complete_ai_session_persist(self, epoch);
        self.ai_session_state_changed();
        self.ai_state_changed();
    }

    #[qslot]
    fn add_ai_attachments(&mut self, paths_json: String) -> bool {
        let changed = queue_ai_attachments(self, paths_json);
        self.ai_state_changed();
        changed
    }

    #[qslot]
    fn complete_ai_attachment_add(&mut self, epoch: i32) {
        complete_ai_attachment_add(self, epoch);
        self.ai_state_changed();
    }

    #[qslot]
    fn remove_ai_attachment(&mut self, index: i32) -> bool {
        let changed = remove_ai_attachment(self, index);
        self.ai_state_changed();
        changed
    }

    #[qslot]
    fn send_ai_prompt(&mut self, prompt: String) -> bool {
        self.ensure_ai_runtime_started();
        let queued = queue_ai_prompt(self, prompt);
        self.ai_state_changed();
        queued
    }

    #[qslot]
    fn queue_ai_follow_up(&mut self, prompt: String) -> bool {
        let queued = queue_ai_follow_up(self, prompt);
        self.ai_state_changed();
        queued
    }

    #[qslot]
    fn edit_last_ai_queued_prompt(&mut self) -> String {
        let prompt = edit_last_ai_queued_prompt(self);
        self.ai_state_changed();
        prompt
    }

    #[qslot]
    fn take_ai_recovered_prompt(&mut self, thread_id: String) -> String {
        take_ai_recovered_prompt(self, thread_id)
    }

    #[qslot]
    fn interrupt_ai_turn(&mut self) -> bool {
        let queued = queue_ai_interrupt(self);
        self.ai_state_changed();
        queued
    }

    #[qslot]
    fn resolve_ai_approval(&mut self, request_id: String, accept: bool) -> bool {
        let queued = queue_ai_approval(self, request_id, accept);
        self.ai_state_changed();
        queued
    }

    #[qslot]
    fn submit_ai_user_input(&mut self, request_id: String, answers_json: String) -> bool {
        let queued = queue_ai_user_input(self, request_id, answers_json);
        self.ai_state_changed();
        queued
    }

    #[qslot]
    fn ai_request_pending(&self, request_id: String) -> bool {
        self.ai_requests.request_is_pending(request_id.as_str())
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

        clear_ai_message_queue(self);
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
    pub(super) fn refresh_forge_review(&mut self) {
        if !self.git_ready || self.forge_loading || self.forge_busy {
            return;
        }

        let epoch = next_forge_epoch(self);
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
        next_forge_epoch(self);
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

    pub(super) fn ensure_ai_runtime_started(&mut self) {
        if ensure_ai_runtime_started(self) {
            self.ai_state_changed();
        }
    }

    fn send_ai_worker_command(&mut self, command: AiWorkerCommand, status_message: &str) {
        send_ai_worker_command(self, command, status_message);
        self.ai_state_changed();
    }

    fn set_status_message(&mut self, status_message: String) {
        if self.status_message == status_message {
            return;
        }
        self.status_message = status_message;
        self.status_message_changed();
    }
}
