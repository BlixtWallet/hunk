use crate::Backend;
use crate::backend_state::next_forge_epoch;
use crate::forge::{ForgeSnapshotPayload, provider_label, review_kind_label, review_state_label};

impl Backend {
    pub(super) fn apply_review_summary(&mut self, review: Option<hunk_forge::OpenReviewSummary>) {
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

    pub(super) fn apply_forge_payload(&mut self, payload: ForgeSnapshotPayload) {
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

    pub(super) fn reset_forge_state(&mut self) {
        next_forge_epoch(self);
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
}
