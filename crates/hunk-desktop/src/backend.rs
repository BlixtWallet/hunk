use std::path::{Path, PathBuf};
use std::sync::Arc;

use hunk_app::ai::AiWorkerCommand;
use hunk_app::diff::DiffCommentStoreCommand;
use hunk_domain::db::CommentStatus;
use hunk_git::workspace::load_git_workspace;
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
use crate::backend_state::DiffCommentRequestKind;
pub use crate::backend_state::{Backend, Workspace};
use crate::comments::DiffCommentStartOutcome;
use crate::diff_models::DiffSnapshotPayload;
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
        "diffCompareSources",
        Member = diff_compare_sources,
        Constant
    );
    qproperty!(
        "diffCompareLeftLabel",
        Member = diff_compare_left_label,
        Notify = diff_state_changed
    );
    qproperty!(
        "diffCompareRightLabel",
        Member = diff_compare_right_label,
        Notify = diff_state_changed
    );
    qproperty!(
        "diffCompareLeftIndex",
        Member = diff_compare_left_index,
        Notify = diff_state_changed
    );
    qproperty!(
        "diffCompareRightIndex",
        Member = diff_compare_right_index,
        Notify = diff_state_changed
    );
    qproperty!(
        "diffCompareFileCount",
        Member = diff_compare_file_count,
        Notify = diff_state_changed
    );
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
        "gitChangedFileCount",
        Member = git_changed_file_count,
        Notify = git_state_changed
    );
    qproperty!("gitReady", Member = git_ready, Notify = git_state_changed);
    qproperty!(
        "gitLoading",
        Member = git_loading,
        Notify = git_state_changed
    );
    qproperty!("gitError", Member = git_error, Notify = git_state_changed);
    qproperty!("terminalTabs", Member = terminal_tabs, Constant);
    qproperty!("terminalRows", Member = terminal_rows, Constant);
    qproperty!("browser", Member = browser, Constant);
    qproperty!(
        "terminalOpen",
        Member = terminal_open,
        Notify = terminal_state_changed
    );
    qproperty!(
        "terminalActiveTabId",
        Member = terminal_active_tab_id,
        Notify = terminal_state_changed
    );
    qproperty!(
        "terminalActiveTabIndex",
        Member = terminal_active_tab_index,
        Notify = terminal_state_changed
    );
    qproperty!(
        "terminalShellLabel",
        Member = terminal_shell_label,
        Notify = terminal_state_changed
    );
    qproperty!(
        "terminalStatus",
        Member = terminal_status,
        Notify = terminal_state_changed
    );
    qproperty!(
        "terminalStatusMessage",
        Member = terminal_status_message,
        Notify = terminal_state_changed
    );
    qproperty!(
        "terminalCwd",
        Member = terminal_cwd,
        Notify = terminal_state_changed
    );
    qproperty!(
        "terminalDisplayOffset",
        Member = terminal_display_offset,
        Notify = terminal_screen_changed
    );
    qproperty!(
        "terminalMouseMode",
        Member = terminal_mouse_mode,
        Notify = terminal_screen_changed
    );
    qproperty!(
        "terminalCursorRow",
        Member = terminal_cursor_row,
        Notify = terminal_screen_changed
    );
    qproperty!(
        "terminalCursorColumn",
        Member = terminal_cursor_column,
        Notify = terminal_screen_changed
    );
    qproperty!(
        "terminalCursorShape",
        Member = terminal_cursor_shape,
        Notify = terminal_screen_changed
    );
    qproperty!(
        "terminalCursorVisible",
        Member = terminal_cursor_visible,
        Notify = terminal_screen_changed
    );
    qproperty!(
        "terminalScreenRevision",
        Member = terminal_screen_revision,
        Notify = terminal_screen_changed
    );
    qproperty!(
        "terminalFocusRevision",
        Member = terminal_focus_revision,
        Notify = terminal_focus_changed
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
    qproperty!("ready", Member = ready, Notify = ready_changed);
    qproperty!("updates", Member = updates, Constant);
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
    pub(super) fn terminal_state_changed(&mut self);

    #[qsignal]
    pub(super) fn terminal_screen_changed(&mut self);

    #[qsignal]
    pub(super) fn terminal_focus_changed(&mut self);

    #[qsignal]
    pub(super) fn ai_state_changed(&mut self);

    #[qsignal]
    pub(super) fn ai_session_state_changed(&mut self);

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
        crate::terminal::configure_terminal(self);

        let invoker = self.get_qml_method_invoker();
        let spawn_result = std::thread::Builder::new()
            .name("hunk-desktop-bootstrap".to_owned())
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
    fn toggle_terminal(&mut self) -> bool {
        crate::terminal::toggle_terminal(self)
    }

    #[qslot]
    fn set_terminal_open(&mut self, open: bool) -> bool {
        crate::terminal::set_terminal_open(self, open)
    }

    #[qslot]
    fn new_terminal_tab(&mut self) -> bool {
        crate::terminal::new_terminal_tab(self)
    }

    #[qslot]
    fn select_terminal_tab(&mut self, tab_id: i32) -> bool {
        crate::terminal::select_terminal_tab(self, tab_id)
    }

    #[qslot]
    fn close_terminal_tab(&mut self, tab_id: i32) -> bool {
        crate::terminal::close_terminal_tab(self, tab_id)
    }

    #[qslot]
    fn move_terminal_tab(&mut self, direction: i32) -> bool {
        crate::terminal::move_terminal_tab(self, direction)
    }

    #[qslot]
    fn resize_terminal(&mut self, rows: i32, cols: i32) -> bool {
        crate::terminal::resize_terminal(self, rows, cols)
    }

    #[qslot]
    fn send_terminal_key(
        &mut self,
        key: String,
        text: String,
        shift: bool,
        control: bool,
        alt: bool,
        platform: bool,
    ) -> bool {
        crate::terminal::send_terminal_key(self, key, text, shift, control, alt, platform)
    }

    #[qslot]
    fn write_terminal_text(&mut self, text: String) -> bool {
        crate::terminal::write_terminal_text(self, text)
    }

    #[qslot]
    fn paste_terminal_text(&mut self, text: String) -> bool {
        crate::terminal::paste_terminal_text(self, text)
    }

    #[qslot]
    fn report_terminal_focus(&mut self, focused: bool) -> bool {
        crate::terminal::report_terminal_focus(self, focused)
    }

    #[qslot]
    #[allow(clippy::too_many_arguments)]
    fn terminal_pointer_button(
        &mut self,
        row: i32,
        column: i32,
        button: i32,
        pressed: bool,
        shift: bool,
        control: bool,
        alt: bool,
    ) -> bool {
        crate::terminal::send_terminal_pointer_button(
            self, row, column, button, pressed, shift, control, alt,
        )
    }

    #[qslot]
    fn terminal_pointer_move(
        &mut self,
        row: i32,
        column: i32,
        button: i32,
        shift: bool,
        control: bool,
        alt: bool,
    ) -> bool {
        crate::terminal::send_terminal_pointer_move(self, row, column, button, shift, control, alt)
    }

    #[qslot]
    fn terminal_wheel(
        &mut self,
        row: i32,
        column: i32,
        lines: i32,
        shift: bool,
        control: bool,
        alt: bool,
    ) -> bool {
        crate::terminal::send_terminal_wheel(self, row, column, lines, shift, control, alt)
    }

    #[qslot]
    fn clear_terminal_screen(&mut self) -> bool {
        crate::terminal::clear_terminal_screen(self)
    }

    #[qslot]
    fn scroll_terminal(&mut self, direction: String) -> bool {
        crate::terminal::scroll_terminal(self, direction)
    }

    #[qslot]
    fn terminal_selection_text(
        &self,
        anchor_row: i32,
        anchor_column: i32,
        head_row: i32,
        head_column: i32,
    ) -> String {
        crate::terminal::selected_terminal_text(
            self,
            anchor_row,
            anchor_column,
            head_row,
            head_column,
        )
    }

    #[qslot]
    fn run_terminal_command(&mut self, command: String, cwd: String) -> bool {
        crate::terminal::run_terminal_command(self, command, cwd)
    }

    #[qslot]
    fn apply_terminal_events(&mut self, _tab_id: i32) {
        crate::terminal::apply_terminal_events(self);
    }

    #[qslot]
    fn complete_terminal_start(&mut self, tab_id: i32, generation: i32) {
        crate::terminal::complete_terminal_start(self, tab_id, generation);
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
    fn select_diff_compare_source(&mut self, side: String, index: i32) {
        self.update_diff_compare_source(side.as_str(), index);
    }

    #[qslot]
    fn refresh_diff_compare(&mut self) {
        self.start_diff_compare_refresh();
    }

    #[qslot]
    fn apply_diff_compare_snapshot(&mut self, epoch: i32) {
        self.complete_diff_compare_snapshot(epoch);
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

        if let Some(patch) = self
            .diff_compare_patches
            .get(self.diff_selected_path.as_str())
            .cloned()
        {
            let payload = DiffSnapshotPayload::from_patch(&summary, patch.as_str());
            self.apply_diff_snapshot_payload(payload);
            return;
        }

        self.diff_epoch = self.diff_epoch.wrapping_add(1).max(1);
        let epoch = self.diff_epoch;
        self.diff_loading = true;
        self.diff_ready = false;
        self.diff_error.clear();
        self.diff_rows.borrow_mut().defer_replace(
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
            .name("hunk-desktop-diff-refresh".to_owned())
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
        match result {
            Some(Ok(payload)) if payload.path == self.diff_selected_path => {
                self.apply_diff_snapshot_payload(payload);
                return;
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
        if self.git_loading {
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
            .name("hunk-desktop-git-refresh".to_owned())
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
        self.git_changed_file_count = 0;
        self.git_ready = false;
        self.git_error.clear();
        self.git_root_pending_persist = true;
        self.reset_diff_state();
        self.reset_diff_comment_state();
        self.git_state_changed();
        self.ai_state_changed();
        self.diff_comments_state_changed();
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

    pub(super) fn ensure_ai_runtime_started(&mut self) {
        if ensure_ai_runtime_started(self) {
            self.ai_state_changed();
        }
    }

    fn send_ai_worker_command(&mut self, command: AiWorkerCommand, status_message: &str) {
        send_ai_worker_command(self, command, status_message);
        self.ai_state_changed();
    }

    pub(super) fn set_status_message(&mut self, status_message: String) {
        if self.status_message == status_message {
            return;
        }
        self.status_message = status_message;
        self.status_message_changed();
    }
}
