use std::path::PathBuf;

use crate::ai_projects::ai_project_catalog_json;
use crate::backend_state::persist_active_project;
use crate::git_models::GitSnapshotPayload;
use crate::{Backend, Workspace};

impl Backend {
    pub(super) fn apply_git_payload(&mut self, payload: GitSnapshotPayload) {
        let diff_files = payload.diff_files;
        let diff_file_summaries = payload.diff_file_summaries;
        let comment_scope_changed =
            self.git_root != payload.root || self.git_branch_name != payload.branch_name;
        self.git_root = payload.root;
        crate::terminal::reconcile_terminal_root(self);
        self.git_repository_name = payload.repository_name;
        self.git_branch_name = payload.branch_name;
        self.git_changed_file_count = payload.changed_file_count;
        self.ai_completion_paths = payload.visible_file_paths;
        self.git_ready = true;
        self.git_error.clear();
        if comment_scope_changed {
            self.reset_diff_comment_state();
            self.diff_comments_state_changed();
        }
        let compare_ready = self.configure_diff_compare(payload.compare_catalog);
        if !compare_ready {
            self.diff_compare_patches.clear();
            self.diff_compare_file_count = i32::try_from(diff_files.len()).unwrap_or(i32::MAX);
            self.replace_diff_files(diff_files, diff_file_summaries);
        }
        if self.git_root_pending_persist {
            self.git_root_pending_persist = false;
            let root = PathBuf::from(self.git_root.as_str());
            match persist_active_project(root.clone()) {
                Ok(paths) => {
                    self.ai_project_catalog_json =
                        ai_project_catalog_json(paths.as_slice(), root.as_path());
                }
                Err(error) => {
                    self.git_error =
                        format!("Repository loaded; failed to save selection: {error:#}");
                }
            }
        }
        self.git_state_changed();
        if compare_ready {
            self.start_diff_compare_refresh();
        } else {
            self.refresh_diff();
        }
        if self.active_workspace == Workspace::Ai.as_str() {
            self.ensure_ai_runtime_started();
        }
    }
}
