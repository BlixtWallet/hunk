impl DiffViewer {
    pub(super) fn request_activate_or_create_bookmark_with_dirty_guard(
        &mut self,
        bookmark_name: String,
        cx: &mut Context<Self>,
    ) {
        let target_bookmark = bookmark_name.trim().to_string();
        if target_bookmark.is_empty() {
            self.git_status_message = Some("Branch name is required.".to_string());
            cx.notify();
            return;
        }
        if self.git_action_loading {
            self.git_status_message =
                Some("Wait for the current workspace action to finish.".to_string());
            cx.notify();
            return;
        }

        let source_bookmark = self
            .checked_out_bookmark_name()
            .unwrap_or(self.branch_name.as_str())
            .to_string();
        if source_bookmark == target_bookmark {
            self.git_status_message = Some(format!("Branch {} is already active.", target_bookmark));
            cx.notify();
            return;
        }

        if !self.files.is_empty() {
            self.git_status_message = Some(format!(
                "Commit or discard {} local files before switching {} -> {}.",
                self.files.len(),
                source_bookmark,
                target_bookmark
            ));
            cx.notify();
            return;
        }

        self.activate_or_create_bookmark(target_bookmark, cx);
    }

    pub(super) fn active_review_action_blocker(&self) -> Option<String> {
        if self.git_action_loading {
            return Some("Another workspace action is in progress.".to_string());
        }
        if !self.can_run_active_bookmark_actions() {
            return Some("Activate a branch before opening PR/MR.".to_string());
        }
        if !self.branch_has_upstream {
            return Some("Publish this branch before opening PR/MR.".to_string());
        }
        None
    }
}
