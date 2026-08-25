use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use hunk_domain::state::AppStateStore;
use hunk_forge::{ForgeCredentialKind, ForgeProvider, ForgeReviewWorkspace};
use hunk_git::workspace::{GitWorkspaceCommand, execute_git_workspace_command, load_git_workspace};
use qtbridge::{QObjectHolder, invoke_method, qobject, qtbridge_type_lib::QString};

use crate::backend_state::ForgeAsyncPayload;
pub use crate::backend_state::{Backend, Workspace};
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
    fn git_state_changed(&mut self);

    #[qsignal]
    fn forge_state_changed(&mut self);

    #[qslot]
    fn select_workspace(&mut self, workspace: String) {
        let Some(workspace) = Workspace::parse(&workspace) else {
            self.set_status_message(format!("Unknown workspace: {workspace}"));
            return;
        };
        if self.active_workspace == workspace.as_str() {
            return;
        }

        self.active_workspace = workspace.as_str().to_owned();
        self.active_workspace_changed();
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
        self.reset_forge_state();
        self.git_state_changed();
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

    fn set_status_message(&mut self, status_message: String) {
        if self.status_message == status_message {
            return;
        }
        self.status_message = status_message;
        self.status_message_changed();
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

    fn apply_git_payload(&mut self, payload: GitSnapshotPayload) {
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
        if self.git_root_pending_persist {
            self.git_root_pending_persist = false;
            if let Err(error) = persist_active_project(PathBuf::from(self.git_root.as_str())) {
                self.git_status_message =
                    format!("Repository loaded; failed to save selection: {error:#}");
            }
        }
        self.git_state_changed();
        if forge_context_changed {
            self.reset_forge_state();
            self.forge_state_changed();
            self.refresh_forge_review();
        }
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
