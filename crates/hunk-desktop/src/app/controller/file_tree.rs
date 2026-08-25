fn review_mode_selected_path(
    review_selected_path: Option<&str>,
    review_files: &[ChangedFile],
) -> Option<String> {
    review_selected_path
        .map(str::to_string)
        .or_else(|| review_files.first().map(|file| file.path.clone()))
}

impl DiffViewer {
    pub(crate) fn current_tree_selected_path(&self) -> Option<String> {
        self.current_review_path()
    }

    fn preferred_review_workspace_path(&self) -> Option<String> {
        if let Some(session) = self.review_workspace_session.as_ref() {
            return preferred_review_workspace_path_for_session(
                self.current_review_editor_path().as_deref(),
                self.current_review_surface_row()
                    .and_then(|row_ix| session.path_at_surface_row(row_ix)),
                self.current_review_file_range().map(|range| range.path).as_deref(),
                self.review_surface.selected_path.as_deref(),
                session,
            );
        }

        review_mode_selected_path(
            self.review_surface.selected_path.as_deref(),
            &self.review_files,
        )
    }

    pub(super) fn toggle_sidebar_tree_action(
        &mut self,
        _: &ToggleSidebarTree,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_active_sidebar(cx);
    }

    fn sidebar_collapsed_for_mode(&self, mode: WorkspaceViewMode) -> Option<bool> {
        mode.collapsible_sidebar_kind()
            .map(|kind| self.sidebar_collapsed_for_kind(kind))
    }

    fn sidebar_collapsed_for_kind(&self, kind: WorkspaceSidebarKind) -> bool {
        match kind {
            WorkspaceSidebarKind::Review => self.review_sidebar_collapsed,
            WorkspaceSidebarKind::AiThreads => self.ai_thread_sidebar_collapsed,
        }
    }

    fn set_sidebar_collapsed_for_kind(&mut self, kind: WorkspaceSidebarKind, collapsed: bool) {
        match kind {
            WorkspaceSidebarKind::Review => self.review_sidebar_collapsed = collapsed,
            WorkspaceSidebarKind::AiThreads => self.ai_thread_sidebar_collapsed = collapsed,
        }
    }

    pub(crate) fn active_sidebar_collapsed(&self) -> Option<bool> {
        self.sidebar_collapsed_for_mode(self.workspace_view_mode)
    }

    pub(crate) fn active_sidebar_label(&self) -> Option<&'static str> {
        self.workspace_view_mode
            .collapsible_sidebar_kind()
            .map(WorkspaceSidebarKind::label)
    }

    pub(super) fn toggle_active_sidebar(&mut self, cx: &mut Context<Self>) {
        if let Some(kind) = self.workspace_view_mode.collapsible_sidebar_kind() {
            let collapsed = !self.sidebar_collapsed_for_kind(kind);
            self.set_sidebar_collapsed_for_kind(kind, collapsed);
            if kind.uses_changed_files() && !collapsed && self.repo_tree.rows.is_empty() {
                self.request_repo_tree_reload(cx);
            }
            cx.notify();
        }
    }

    pub(super) fn switch_to_review_view_action(
        &mut self,
        _: &SwitchToReviewView,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_handle.focus(window, cx);
        self.set_workspace_view_mode(WorkspaceSwitchAction::Review.target_mode(), cx);
    }

    pub(super) fn switch_to_git_view_action(
        &mut self,
        _: &SwitchToGitView,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_handle.focus(window, cx);
        self.set_workspace_view_mode(WorkspaceSwitchAction::Git.target_mode(), cx);
    }

    pub(super) fn switch_to_ai_view_action(
        &mut self,
        _: &SwitchToAiView,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_ai_workspace(window, cx);
    }

    pub(super) fn activate_ai_workspace(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_handle.focus(window, cx);
        self.set_workspace_view_mode(WorkspaceSwitchAction::Ai.target_mode(), cx);
        self.focus_ai_composer_input(window, cx);
        self.maybe_prepare_ai_desktop_notifications(cx);
    }

    pub(super) fn set_workspace_view_mode(&mut self, mode: WorkspaceViewMode, cx: &mut Context<Self>) {
        let previous_mode = self.workspace_view_mode;
        if previous_mode == mode {
            if mode == WorkspaceViewMode::Diff && self.repo_tree.rows.is_empty() {
                self.request_repo_tree_reload(cx);
            }
            return;
        }

        if previous_mode == WorkspaceViewMode::Diff {
            self.review_surface.selected_path = self.current_review_path();
        }

        self.workspace_view_mode = mode;
        self.workspace_text_context_menu = None;
        if mode != WorkspaceViewMode::Diff {
            self.comments_preview_open = false;
        }

        if mode == WorkspaceViewMode::Diff {
            let next_path = self.preferred_review_workspace_path();
            let next_status = next_path
                .as_deref()
                .and_then(|selected| self.status_for_path(selected));
            self.set_review_selected_file(next_path, next_status);
            self.request_repo_tree_reload(cx);
            if self.should_reuse_loaded_review_compare() {
                self.scroll_selected_after_reload = false;
                self.prime_diff_surface_visible_state(false, cx);
            } else {
                self.scroll_selected_after_reload = true;
                self.request_selected_diff_reload(cx);
            }
        } else if mode == WorkspaceViewMode::Ai {
            self.refresh_ai_repo_thread_catalog(cx);
            self.ensure_ai_runtime_started(cx);
            self.maybe_prepare_ai_desktop_notifications(cx);
        }

        if self.editor_search_visible {
            self.sync_editor_search_query(cx);
        }
        cx.notify();
    }

    pub(super) fn select_repo_tree_file(&mut self, path: String, cx: &mut Context<Self>) {
        let status = self.status_for_path(path.as_str());
        self.set_review_selected_file(Some(path.clone()), status);
        self.scroll_to_file_start(&path);
        self.review_surface.clear_workspace_surface_snapshot();
        self.review_surface.last_prefetched_visible_row_range = None;
        self.review_surface.last_diff_scroll_offset = None;
        self.last_scroll_activity_at = Instant::now();
        cx.notify();
    }

    pub(super) fn request_repo_tree_reload(&mut self, cx: &mut Context<Self>) {
        self.rebuild_repo_tree_for_changed_files();
        cx.notify();
    }

    fn rebuild_repo_tree_for_changed_files(&mut self) {
        self.capture_sidebar_repo_scroll_anchor();
        self.repo_tree.rows = build_changed_file_rows(self.active_diff_files());
        self.repo_tree.file_count = self.repo_tree.rows.len();
    }
}
