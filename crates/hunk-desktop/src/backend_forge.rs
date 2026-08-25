use std::path::PathBuf;
use std::sync::Arc;

use hunk_forge::{ForgeCredentialKind, ForgeProvider, ForgeReviewWorkspace};
use qtbridge::{QObjectHolder, invoke_method};

use crate::Backend;
use crate::backend_state::{ForgeAsyncPayload, next_forge_epoch};
use crate::forge::{
    ForgeSnapshotPayload, create_or_find_review, load_forge_snapshot, provider_label,
    review_kind_label, review_short_label, review_state_label, run_github_device_flow,
    save_forge_token,
};

impl Backend {
    pub(super) fn refresh_forge_review_impl(&mut self) {
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
            .name("hunk-desktop-forge-refresh".to_owned())
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

    pub(super) fn save_forge_personal_access_token_impl(&mut self, token: String) {
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

    pub(super) fn create_forge_review_impl(
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
            .name("hunk-desktop-forge-review".to_owned())
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

    pub(super) fn start_github_device_flow_impl(&mut self) {
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
            .name("hunk-desktop-github-auth".to_owned())
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

    pub(super) fn cancel_github_device_flow_impl(&mut self) {
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

    pub(super) fn apply_github_device_authorization_impl(&mut self, epoch: i32) {
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

    pub(super) fn apply_forge_result_impl(&mut self, epoch: i32) {
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
            Some(Ok(ForgeAsyncPayload::Snapshot(payload))) => self.apply_forge_payload(*payload),
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

    pub(super) fn begin_forge_action(&mut self, label: String) -> i32 {
        let epoch = next_forge_epoch(self);
        self.forge_busy = true;
        self.forge_error.clear();
        self.forge_status_message.clear();
        self.forge_action_label = label;
        self.forge_state_changed();
        epoch
    }

    pub(super) fn run_save_forge_token(
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
            .name("hunk-desktop-forge-credential".to_owned())
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

    pub(super) fn fail_forge_spawn(&mut self, operation: &str, error: std::io::Error) {
        self.forge_loading = false;
        self.forge_busy = false;
        self.forge_action_label.clear();
        self.forge_error = format!("Failed to start {operation}: {error}");
        self.forge_state_changed();
    }
}
