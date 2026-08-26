use crate::ai_attachments::ai_attachment_pending;
use crate::ai_session::{ai_session_controls_locked, compact_token_count, exact_token_count};
use crate::backend_ai::{
    ai_active_queue_sending, ai_active_queued_message_count, ai_active_request_count,
    ai_interrupt_pending, ai_pending_request_count, ai_prompt_pending, ai_queued_message_count,
    ai_request_answerable, ai_request_description, ai_request_id, ai_request_kind,
    ai_request_questions_json, ai_request_reason, ai_request_resolving, ai_request_title,
    ai_thread_action_pending,
};
use crate::backend_state::Backend;

impl Backend {
    pub(super) fn ai_attachment_pending_value(&self) -> bool {
        ai_attachment_pending(self)
    }

    pub(super) fn ai_model_supports_image_inputs_value(&self) -> bool {
        self.ai_session_catalog
            .model_supports_image_inputs(Some(self.ai_selected_model.as_str()))
    }

    pub(super) fn ai_prompt_pending_value(&self) -> bool {
        ai_prompt_pending(self)
    }

    pub(super) fn ai_queued_message_count_value(&self) -> i32 {
        ai_queued_message_count(self)
    }

    pub(super) fn ai_active_queued_message_count_value(&self) -> i32 {
        ai_active_queued_message_count(self)
    }

    pub(super) fn ai_active_queue_sending_value(&self) -> bool {
        ai_active_queue_sending(self)
    }

    pub(super) fn ai_interrupt_pending_value(&self) -> bool {
        ai_interrupt_pending(self)
    }

    pub(super) fn ai_thread_action_pending_value(&self) -> bool {
        ai_thread_action_pending(self)
    }

    pub(super) fn ai_pending_request_count_value(&self) -> i32 {
        ai_pending_request_count(self)
    }

    pub(super) fn ai_account_summary_value(&self) -> String {
        self.ai_account.summary.clone()
    }

    pub(super) fn ai_account_connected_value(&self) -> bool {
        self.ai_account.connected
    }

    pub(super) fn ai_login_pending_value(&self) -> bool {
        self.ai_account.login_pending
    }

    pub(super) fn ai_approval_request_count_value(&self) -> i32 {
        self.ai_requests.approval_count
    }

    pub(super) fn ai_input_request_count_value(&self) -> i32 {
        self.ai_requests.input_count
    }

    pub(super) fn ai_five_hour_limit_available_value(&self) -> bool {
        self.ai_account.five_hour_limit.available
    }

    pub(super) fn ai_five_hour_limit_remaining_percent_value(&self) -> i32 {
        self.ai_account.five_hour_limit.remaining_percent
    }

    pub(super) fn ai_five_hour_limit_reset_label_value(&self) -> String {
        self.ai_account.five_hour_limit.reset_label.clone()
    }

    pub(super) fn ai_weekly_limit_available_value(&self) -> bool {
        self.ai_account.weekly_limit.available
    }

    pub(super) fn ai_weekly_limit_remaining_percent_value(&self) -> i32 {
        self.ai_account.weekly_limit.remaining_percent
    }

    pub(super) fn ai_weekly_limit_reset_label_value(&self) -> String {
        self.ai_account.weekly_limit.reset_label.clone()
    }

    pub(super) fn ai_active_request_count_value(&self) -> i32 {
        ai_active_request_count(self)
    }

    pub(super) fn ai_request_id_value(&self) -> String {
        ai_request_id(self)
    }

    pub(super) fn ai_request_kind_value(&self) -> String {
        ai_request_kind(self)
    }

    pub(super) fn ai_request_title_value(&self) -> String {
        ai_request_title(self)
    }

    pub(super) fn ai_request_description_value(&self) -> String {
        ai_request_description(self)
    }

    pub(super) fn ai_request_reason_value(&self) -> String {
        ai_request_reason(self)
    }

    pub(super) fn ai_request_questions_json_value(&self) -> String {
        ai_request_questions_json(self)
    }

    pub(super) fn ai_request_answerable_value(&self) -> bool {
        ai_request_answerable(self)
    }

    pub(super) fn ai_request_resolving_value(&self) -> bool {
        ai_request_resolving(self)
    }

    pub(super) fn ai_selected_model_index_value(&self) -> i32 {
        self.ai_models
            .borrow()
            .index_of(self.ai_selected_model.as_str())
    }

    pub(super) fn ai_selected_effort_index_value(&self) -> i32 {
        self.ai_efforts
            .borrow()
            .index_of(self.ai_selected_effort.as_str())
    }

    pub(super) fn ai_selected_service_tier_index_value(&self) -> i32 {
        self.ai_service_tiers
            .borrow()
            .index_of(self.ai_selected_service_tier.as_str())
    }

    pub(super) fn ai_selected_model_label_value(&self) -> String {
        self.ai_models
            .borrow()
            .label_for_value(self.ai_selected_model.as_str())
    }

    pub(super) fn ai_selected_effort_label_value(&self) -> String {
        self.ai_efforts
            .borrow()
            .label_for_value(self.ai_selected_effort.as_str())
    }

    pub(super) fn ai_selected_collaboration_label_value(&self) -> String {
        if self.ai_selected_collaboration_mode == "plan" {
            "Plan".to_owned()
        } else {
            "Code".to_owned()
        }
    }

    pub(super) fn ai_selected_service_tier_label_value(&self) -> String {
        self.ai_service_tiers
            .borrow()
            .label_for_value(self.ai_selected_service_tier.as_str())
    }

    pub(super) fn ai_approval_policy_label_value(&self) -> String {
        if self.ai_mad_max_mode {
            "Full access".to_owned()
        } else {
            "Ask for approvals".to_owned()
        }
    }

    pub(super) fn ai_effort_option_count_value(&self) -> i32 {
        self.ai_efforts.borrow().item_count()
    }

    pub(super) fn ai_session_controls_locked_value(&self) -> bool {
        ai_session_controls_locked(self)
    }

    pub(super) fn ai_context_available_value(&self) -> bool {
        self.ai_session_catalog.context_usage.available
    }

    pub(super) fn ai_context_percent_used_value(&self) -> i32 {
        self.ai_session_catalog.context_usage.percent_used
    }

    pub(super) fn ai_context_percent_left_value(&self) -> i32 {
        self.ai_session_catalog.context_usage.percent_left
    }

    pub(super) fn ai_context_token_summary_value(&self) -> String {
        let usage = &self.ai_session_catalog.context_usage;
        if !usage.available {
            return "No context usage yet".to_owned();
        }
        format!(
            "{} / {} tokens",
            compact_token_count(usage.context_tokens),
            compact_token_count(usage.context_window_tokens),
        )
    }

    pub(super) fn ai_context_input_tokens_value(&self) -> String {
        exact_token_count(self.ai_session_catalog.context_usage.input_tokens)
    }

    pub(super) fn ai_context_cached_input_tokens_value(&self) -> String {
        exact_token_count(self.ai_session_catalog.context_usage.cached_input_tokens)
    }

    pub(super) fn ai_context_output_tokens_value(&self) -> String {
        exact_token_count(self.ai_session_catalog.context_usage.output_tokens)
    }

    pub(super) fn ai_context_reasoning_tokens_value(&self) -> String {
        exact_token_count(self.ai_session_catalog.context_usage.reasoning_tokens)
    }

    pub(super) fn ai_context_billable_tokens_value(&self) -> String {
        exact_token_count(self.ai_session_catalog.context_usage.billable_tokens)
    }
}
