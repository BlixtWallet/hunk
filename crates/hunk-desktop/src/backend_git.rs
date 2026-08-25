use std::path::PathBuf;

use hunk_git::workspace::{GitWorkspaceCommand, execute_git_workspace_command};
use qtbridge::{QObjectHolder, invoke_method, qtbridge_type_lib::QString};

use crate::backend_state::persist_active_project;
use crate::git_models::GitSnapshotPayload;
use crate::{Backend, Workspace};

impl Backend {
    pub(super) fn notify_git_state_changed(&mut self) {
        self.git_state_changed();
    }

    pub(super) fn run_git_command(&mut self, label: &str, command: GitWorkspaceCommand) {
        if self.git_loading || self.git_busy {
            return;
        }
        self.git_busy = true;
        self.git_error.clear();
        self.git_action_label = label.to_owned();
        self.notify_git_state_changed();

        let root = PathBuf::from(self.git_root.clone());
        let invoker = self.get_qml_method_invoker();
        let spawn_result = std::thread::Builder::new()
            .name("hunk-desktop-git-command".to_owned())
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
            self.notify_git_state_changed();
        }
    }

    pub(super) fn apply_git_payload(&mut self, payload: GitSnapshotPayload) {
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
        crate::terminal::reconcile_terminal_root(self);
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
}
