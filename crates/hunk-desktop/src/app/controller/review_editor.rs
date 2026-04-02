const REVIEW_EDITOR_SAVE_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(250);
const REVIEW_EDITOR_CONTEXT_LINES: usize = 3;

impl DiffViewer {
    pub(super) fn active_review_editor_path(&self) -> Option<&str> {
        self.selected_path
            .as_deref()
            .filter(|path| self.review_editor_sessions.contains_key(*path))
    }

    pub(super) fn review_editor_session(&self, path: &str) -> Option<&ReviewEditorFileSession> {
        self.review_editor_sessions.get(path)
    }

    fn review_editor_session_mut(&mut self, path: &str) -> Option<&mut ReviewEditorFileSession> {
        self.review_editor_sessions.get_mut(path)
    }

    fn active_review_editor_session(&self) -> Option<&ReviewEditorFileSession> {
        self.active_review_editor_path()
            .and_then(|path| self.review_editor_session(path))
    }

    fn active_review_editor_session_mut(&mut self) -> Option<&mut ReviewEditorFileSession> {
        let path = self.selected_path.clone()?;
        self.review_editor_session_mut(path.as_str())
    }

    fn ensure_review_editor_session(&mut self, path: &str) -> &mut ReviewEditorFileSession {
        self.review_editor_sessions
            .entry(path.to_string())
            .or_insert_with(|| ReviewEditorFileSession::new(path.to_string()))
    }

    fn next_review_editor_epoch(&mut self, path: &str) -> usize {
        let session = self.ensure_review_editor_session(path);
        session.load_epoch = session.load_epoch.saturating_add(1);
        session.load_epoch
    }

    fn next_review_editor_save_epoch(&mut self, path: &str) -> usize {
        let session = self.ensure_review_editor_session(path);
        session.save_epoch = session.save_epoch.saturating_add(1);
        session.save_epoch
    }

    fn next_review_editor_presentation_epoch(&mut self, path: &str) -> usize {
        let session = self.ensure_review_editor_session(path);
        session.presentation_epoch = session.presentation_epoch.saturating_add(1);
        session.presentation_epoch
    }

    fn cancel_review_editor_save_task_for_path(&mut self, path: &str) {
        let Some(session) = self.review_editor_session_mut(path) else {
            return;
        };
        let previous_task = std::mem::replace(&mut session.save_task, Task::ready(()));
        drop(previous_task);
    }

    fn cancel_review_editor_presentation_task_for_path(&mut self, path: &str) {
        let Some(session) = self.review_editor_session_mut(path) else {
            return;
        };
        let previous_task = std::mem::replace(&mut session.presentation_task, Task::ready(()));
        drop(previous_task);
    }

    fn reset_review_editor_file_session(session: &mut ReviewEditorFileSession) {
        session.load_epoch = session.load_epoch.saturating_add(1);
        session.presentation_epoch = session.presentation_epoch.saturating_add(1);
        session.save_epoch = session.save_epoch.saturating_add(1);
        session.loading = false;
        session.presentation_loading = false;
        session.save_loading = false;
        session.error = None;
        session.left_source_id = None;
        session.right_source_id = None;
        session.left_present = false;
        session.right_present = false;
        session.load_task = Task::ready(());
        session.presentation_task = Task::ready(());
        session.save_task = Task::ready(());
        session.last_saved_text = None;
        session.right_hunk_lines.clear();
        session.right_to_left_line_map.clear();
        session.pending_target_right_line = None;
        session.left_editor.borrow_mut().shutdown();
        session.right_editor.borrow_mut().shutdown();
    }

    fn sync_review_editor_sessions_to_files(&mut self) {
        let valid_paths = self
            .review_files
            .iter()
            .map(|file| file.path.clone())
            .collect::<BTreeSet<_>>();
        let stale_paths = self
            .review_editor_sessions
            .keys()
            .filter(|path| !valid_paths.contains(*path))
            .cloned()
            .collect::<Vec<_>>();
        for path in stale_paths {
            if let Some(mut session) = self.review_editor_sessions.remove(path.as_str()) {
                Self::reset_review_editor_file_session(&mut session);
            }
        }
        self.review_editor_list_state.reset(self.review_files.len());
        if self
            .selected_path
            .as_ref()
            .is_some_and(|path| !valid_paths.contains(path))
        {
            self.selected_path = self.review_files.first().map(|file| file.path.clone());
            self.selected_status = self
                .selected_path
                .as_deref()
                .and_then(|path| self.status_for_path(path));
            self.active_review_editor_comment_line = None;
        }
    }

    fn clear_review_editor_session(&mut self) {
        for session in self.review_editor_sessions.values_mut() {
            Self::reset_review_editor_file_session(session);
        }
        self.review_editor_sessions.clear();
        self.review_editor_list_state.reset(0);
    }

    fn apply_review_editor_presentation_for_path(
        &mut self,
        path: &str,
        presentation: crate::app::review_editor_model::ReviewEditorPresentation,
    ) {
        let Some(session) = self.review_editor_session_mut(path) else {
            return;
        };
        session
            .left_editor
            .borrow_mut()
            .set_manual_spacers(presentation.left_spacers);
        session
            .right_editor
            .borrow_mut()
            .set_manual_spacers(presentation.right_spacers);
        session
            .left_editor
            .borrow_mut()
            .set_manual_overlays(presentation.left_overlays);
        session
            .right_editor
            .borrow_mut()
            .set_manual_overlays(presentation.right_overlays);
        session
            .left_editor
            .borrow_mut()
            .set_folded_regions(presentation.left_folds);
        session
            .right_editor
            .borrow_mut()
            .set_folded_regions(presentation.right_folds);
        session.right_hunk_lines = presentation.right_hunk_lines;
        session.right_to_left_line_map = presentation.right_to_left_line_map;
        self.apply_pending_review_editor_navigation_target_for_path(path);
    }

    fn request_review_editor_presentation_refresh_for_path(
        &mut self,
        path: &str,
        show_loading: bool,
        cx: &mut Context<Self>,
    ) {
        let Some((left_text, right_text, left_source_id, right_source_id, pinned_right_line)) = self
            .review_editor_session(path)
            .map(|session| {
                (
                    session.left_editor.borrow().current_text(),
                    session.right_editor.borrow().current_text(),
                    session.left_source_id.clone(),
                    session.right_source_id.clone(),
                    session
                        .right_editor
                        .borrow()
                        .selection()
                        .map(|selection| selection.head.line),
                )
            })
        else {
            return;
        };
        let (Some(left_text), Some(right_text)) = (left_text, right_text) else {
            return;
        };

        let started_at = std::time::Instant::now();
        let path = path.to_string();
        let presentation_epoch = self.next_review_editor_presentation_epoch(path.as_str());
        self.cancel_review_editor_presentation_task_for_path(path.as_str());
        if let Some(session) = self.review_editor_session_mut(path.as_str()) {
            session.presentation_loading = show_loading;
            session.presentation_task = cx.spawn(async move |this, cx| {
                let presentation = cx.background_executor().spawn(async move {
                    build_review_editor_presentation_from_texts(
                        left_text.as_str(),
                        right_text.as_str(),
                        REVIEW_EDITOR_CONTEXT_LINES,
                        pinned_right_line,
                    )
                });
                let presentation = presentation.await;

                if let Some(this) = this.upgrade() {
                    this.update(cx, |this, cx| {
                        let should_apply = this
                            .review_editor_session(path.as_str())
                            .is_some_and(|session| {
                                presentation_epoch == session.presentation_epoch
                                    && session.left_source_id == left_source_id
                                    && session.right_source_id == right_source_id
                            });
                        if !should_apply {
                            return;
                        }
                        if let Some(session) = this.review_editor_session_mut(path.as_str()) {
                            session.presentation_loading = false;
                        }
                        this.apply_review_editor_presentation_for_path(path.as_str(), presentation);
                        debug!(
                            path = path.as_str(),
                            left = left_source_id.as_deref().unwrap_or("unknown"),
                            right = right_source_id.as_deref().unwrap_or("unknown"),
                            hunks = this
                                .review_editor_session(path.as_str())
                                .map_or(0, |session| session.right_hunk_lines.len()),
                            elapsed_ms = started_at.elapsed().as_millis(),
                            "review editor presentation refreshed"
                        );
                        cx.notify();
                    });
                }
            });
        }
    }

    fn sync_review_editor_viewports_from_right_for_path(&mut self, path: &str) {
        let Some(session) = self.review_editor_session(path) else {
            return;
        };
        let right_first_visible_source_line = session
            .right_editor
            .borrow()
            .first_visible_source_line()
            .unwrap_or(0);
        let mapped_left_source_line = nearest_mapped_review_editor_left_line(
            &session.right_to_left_line_map,
            right_first_visible_source_line,
        )
        .unwrap_or(0);
        if let Some(session) = self.review_editor_session_mut(path) {
            session
                .left_editor
                .borrow_mut()
                .set_first_visible_source_line(mapped_left_source_line);
        }
    }

    fn sync_review_editor_viewports_from_right(&mut self) {
        let Some(path) = self.selected_path.clone() else {
            return;
        };
        self.sync_review_editor_viewports_from_right_for_path(path.as_str());
    }

    fn apply_pending_review_editor_navigation_target_for_path(&mut self, path: &str) {
        let Some(session) = self.review_editor_session_mut(path) else {
            return;
        };
        let Some(target_line) = session.pending_target_right_line.take() else {
            return;
        };
        session.right_editor.borrow_mut().set_caret_line(target_line);
    }

    fn jump_review_editor_to_line(&mut self, target_line: usize, cx: &mut Context<Self>) -> bool {
        let Some(session) = self.active_review_editor_session_mut() else {
            return false;
        };
        if !session.right_editor.borrow_mut().set_caret_line(target_line) {
            return false;
        }
        let path = session.path.clone();
        self.sync_review_editor_viewports_from_right_for_path(path.as_str());
        cx.notify();
        true
    }

    fn navigate_review_editor_hunk_relative(
        &mut self,
        direction: isize,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(session) = self.active_review_editor_session() else {
            return false;
        };
        let current_line = session
            .right_editor
            .borrow()
            .selection()
            .map(|selection| selection.head.line)
            .or_else(|| session.right_editor.borrow().first_visible_source_line())
            .unwrap_or(0);
        let Some(target_line) =
            find_wrapped_review_editor_hunk_line(&session.right_hunk_lines, current_line, direction)
        else {
            return false;
        };
        self.jump_review_editor_to_line(target_line, cx)
    }

    fn current_review_editor_text_for_path(&self, path: &str) -> anyhow::Result<String> {
        self.review_editor_session(path)
            .and_then(|session| session.right_editor.borrow().current_text())
            .ok_or_else(|| anyhow::anyhow!("no active review editor buffer"))
    }

    fn schedule_review_editor_save_for_path(&mut self, path: &str, cx: &mut Context<Self>) {
        let Some(repo_root) = self.project_path.clone() else {
            return;
        };
        let Ok(current_text) = self.current_review_editor_text_for_path(path) else {
            return;
        };
        if self
            .review_editor_session(path)
            .and_then(|session| session.last_saved_text.as_deref())
            .is_some_and(|saved| saved == current_text.as_str())
        {
            return;
        }

        let path = path.to_string();
        let text_to_write = current_text.clone();
        let saved_text = current_text;
        let save_epoch = self.next_review_editor_save_epoch(path.as_str());
        self.cancel_review_editor_save_task_for_path(path.as_str());
        if let Some(session) = self.review_editor_session_mut(path.as_str()) {
            session.save_loading = true;
            session.save_task = cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(REVIEW_EDITOR_SAVE_DEBOUNCE)
                    .await;
                let path_for_write = path.clone();
                let result = cx.background_executor().spawn(async move {
                    save_file_editor_document(
                        &repo_root,
                        path_for_write.as_str(),
                        text_to_write.as_str(),
                    )
                });
                let result = result.await;

                if let Some(this) = this.upgrade() {
                    this.update(cx, |this, cx| {
                        if this
                            .review_editor_session(path.as_str())
                            .is_none_or(|session| save_epoch != session.save_epoch)
                        {
                            return;
                        }
                        if let Some(session) = this.review_editor_session_mut(path.as_str()) {
                            session.save_loading = false;
                        }
                        match result {
                            Ok(()) => {
                                if let Some(session) = this.review_editor_session_mut(path.as_str()) {
                                    session.last_saved_text = Some(saved_text.clone());
                                    session.right_editor.borrow_mut().mark_saved();
                                }
                                this.git_status_message = Some(format!("Saved {}", path));
                                this.request_review_editor_presentation_refresh_for_path(
                                    path.as_str(),
                                    false,
                                    cx,
                                );
                                this.request_snapshot_refresh_workflow_only(false, cx);
                            }
                            Err(err) => {
                                this.git_status_message =
                                    Some(format!("Save failed for {}: {err:#}", path));
                            }
                        }

                        cx.notify();
                    });
                }
            });
        }
    }

    pub(super) fn review_editor_copy_selection(&self, cx: &mut Context<Self>) -> bool {
        let Some(text) = self
            .active_review_editor_session()
            .and_then(|session| session.right_editor.borrow().copy_selection_text())
        else {
            return false;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        true
    }

    pub(super) fn review_editor_cut_selection(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(path) = self.selected_path.clone() else {
            return false;
        };
        let Some(text) = self
            .review_editor_session_mut(path.as_str())
            .and_then(|session| session.right_editor.borrow_mut().cut_selection_text())
        else {
            return false;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.schedule_review_editor_save_for_path(path.as_str(), cx);
        cx.notify();
        true
    }

    pub(super) fn review_editor_paste_from_clipboard(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(path) = self.selected_path.clone() else {
            return false;
        };
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return false;
        };
        let pasted = self
            .review_editor_session_mut(path.as_str())
            .is_some_and(|session| session.right_editor.borrow_mut().paste_text(text.as_str()));
        if !pasted {
            return false;
        }
        self.schedule_review_editor_save_for_path(path.as_str(), cx);
        cx.notify();
        true
    }

    pub(super) fn review_editor_handle_keystroke(
        &mut self,
        keystroke: &gpui::Keystroke,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(path) = self.selected_path.clone() else {
            return false;
        };
        let handled = self
            .review_editor_session_mut(path.as_str())
            .is_some_and(|session| session.right_editor.borrow_mut().handle_keystroke(keystroke));
        if !handled {
            return false;
        }
        self.schedule_review_editor_save_for_path(path.as_str(), cx);
        cx.notify();
        true
    }

    pub(super) fn review_editor_activate_path(
        &mut self,
        path: String,
        window: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        if self.selected_path.as_deref() == Some(path.as_str())
            && self.selected_status == self.status_for_path(path.as_str())
        {
            if let Some(window) = window {
                self.review_editor_focus_handle.focus(window, cx);
            }
            return;
        }
        self.selected_path = Some(path.clone());
        self.selected_status = self.status_for_path(path.as_str());
        self.active_review_editor_comment_line = None;
        if let Some(window) = window {
            self.review_editor_focus_handle.focus(window, cx);
        }
        self.review_editor_list_state
            .scroll_to_reveal_item(
                self.review_files
                    .iter()
                    .position(|file| file.path == path)
                    .unwrap_or(0),
            );
        self.request_review_editor_reload(false, cx);
        cx.notify();
    }

    fn handle_review_editor_motion(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        apply: impl FnOnce(&mut crate::app::native_files_editor::FilesEditor) -> bool,
    ) {
        if !self.review_editor_focus_handle.is_focused(window) {
            return;
        }
        let changed = self
            .active_review_editor_session_mut()
            .is_some_and(|session| session.right_editor.borrow_mut().apply_motion_action(apply));
        if changed {
            cx.notify();
        }
    }

    pub(super) fn review_editor_copy_action(
        &mut self,
        _: &FilesEditorCopy,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.review_editor_focus_handle.is_focused(window) {
            return;
        }
        let _ = self.review_editor_copy_selection(cx);
    }

    pub(super) fn review_editor_cut_action(
        &mut self,
        _: &FilesEditorCut,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.review_editor_focus_handle.is_focused(window) {
            return;
        }
        let _ = self.review_editor_cut_selection(cx);
    }

    pub(super) fn review_editor_paste_action(
        &mut self,
        _: &FilesEditorPaste,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.review_editor_focus_handle.is_focused(window) {
            return;
        }
        let _ = self.review_editor_paste_from_clipboard(cx);
    }

    pub(super) fn review_editor_move_up_action(
        &mut self,
        _: &FilesEditorMoveUp,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_vertical_action(false, false));
    }

    pub(super) fn review_editor_move_down_action(
        &mut self,
        _: &FilesEditorMoveDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_vertical_action(true, false));
    }

    pub(super) fn review_editor_select_up_action(
        &mut self,
        _: &FilesEditorSelectUp,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_vertical_action(false, true));
    }

    pub(super) fn review_editor_select_down_action(
        &mut self,
        _: &FilesEditorSelectDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_vertical_action(true, true));
    }

    pub(super) fn review_editor_move_left_action(
        &mut self,
        _: &FilesEditorMoveLeft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_horizontal_action(false, false));
    }

    pub(super) fn review_editor_move_right_action(
        &mut self,
        _: &FilesEditorMoveRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_horizontal_action(true, false));
    }

    pub(super) fn review_editor_select_left_action(
        &mut self,
        _: &FilesEditorSelectLeft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_horizontal_action(false, true));
    }

    pub(super) fn review_editor_select_right_action(
        &mut self,
        _: &FilesEditorSelectRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_horizontal_action(true, true));
    }

    pub(super) fn review_editor_move_to_beginning_of_line_action(
        &mut self,
        _: &FilesEditorMoveToBeginningOfLine,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_to_line_boundary_action(true, false));
    }

    pub(super) fn review_editor_move_to_end_of_line_action(
        &mut self,
        _: &FilesEditorMoveToEndOfLine,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_to_line_boundary_action(false, false));
    }

    pub(super) fn review_editor_move_to_beginning_of_document_action(
        &mut self,
        _: &FilesEditorMoveToBeginningOfDocument,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_to_document_boundary_action(true, false));
    }

    pub(super) fn review_editor_move_to_end_of_document_action(
        &mut self,
        _: &FilesEditorMoveToEndOfDocument,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_to_document_boundary_action(false, false));
    }

    pub(super) fn review_editor_select_to_beginning_of_line_action(
        &mut self,
        _: &FilesEditorSelectToBeginningOfLine,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_to_line_boundary_action(true, true));
    }

    pub(super) fn review_editor_select_to_end_of_line_action(
        &mut self,
        _: &FilesEditorSelectToEndOfLine,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_to_line_boundary_action(false, true));
    }

    pub(super) fn review_editor_select_to_beginning_of_document_action(
        &mut self,
        _: &FilesEditorSelectToBeginningOfDocument,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_to_document_boundary_action(true, true));
    }

    pub(super) fn review_editor_select_to_end_of_document_action(
        &mut self,
        _: &FilesEditorSelectToEndOfDocument,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_to_document_boundary_action(false, true));
    }

    pub(super) fn review_editor_move_to_previous_word_start_action(
        &mut self,
        _: &FilesEditorMoveToPreviousWordStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_word_action(false, false));
    }

    pub(super) fn review_editor_move_to_next_word_end_action(
        &mut self,
        _: &FilesEditorMoveToNextWordEnd,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_word_action(true, false));
    }

    pub(super) fn review_editor_select_to_previous_word_start_action(
        &mut self,
        _: &FilesEditorSelectToPreviousWordStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_word_action(false, true));
    }

    pub(super) fn review_editor_select_to_next_word_end_action(
        &mut self,
        _: &FilesEditorSelectToNextWordEnd,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.move_word_action(true, true));
    }

    pub(super) fn review_editor_page_up_action(
        &mut self,
        _: &FilesEditorPageUp,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.page_scroll_action(crate::app::native_files_editor::ScrollDirection::Backward));
    }

    pub(super) fn review_editor_page_down_action(
        &mut self,
        _: &FilesEditorPageDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_review_editor_motion(window, cx, |editor| editor.page_scroll_action(crate::app::native_files_editor::ScrollDirection::Forward));
    }

    fn request_review_editor_reload_for_path(
        &mut self,
        path: &str,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        if self.workspace_view_mode != WorkspaceViewMode::Diff || !self.active_diff_contains_path(path) {
            return;
        }
        let Some(project_root) = self.project_path.clone() else {
            return;
        };
        let Some((left_source, right_source)) = self.selected_review_compare_sources() else {
            return;
        };
        self.ensure_review_editor_session(path);
        let previous_path = self.review_editor_session(path).map(|session| session.path.clone());
        let previous_left_source_id = self
            .review_editor_session(path)
            .and_then(|session| session.left_source_id.clone());
        let previous_right_source_id = self
            .review_editor_session(path)
            .and_then(|session| session.right_source_id.clone());
        let left_source_id = self.review_left_source_id.clone();
        let right_source_id = self.review_right_source_id.clone();

        if !force
            && self
                .review_editor_session(path)
                .is_some_and(|session| {
                    session.left_source_id == left_source_id
                        && session.right_source_id == right_source_id
                        && session.error.is_none()
                        && (session.loading
                            || session.left_editor.borrow().current_text().is_some()
                            || session.right_editor.borrow().current_text().is_some())
                })
        {
            return;
        }

        let path = path.to_string();
        let load_started_at = std::time::Instant::now();
        let epoch = self.next_review_editor_epoch(path.as_str());
        self.next_review_editor_presentation_epoch(path.as_str());
        self.cancel_review_editor_presentation_task_for_path(path.as_str());
        if let Some(session) = self.review_editor_session_mut(path.as_str()) {
            session.loading = true;
            session.presentation_loading = false;
            session.error = None;
            session.left_source_id = left_source_id.clone();
            session.right_source_id = right_source_id.clone();
            session.load_task = cx.spawn(async move |this, cx| {
                let project_root_for_load = project_root.clone();
                let path_for_load = path.clone();
                let result = cx.background_executor().spawn(async move {
                    load_compare_file_document(
                        &project_root_for_load,
                        &left_source,
                        &right_source,
                        path_for_load.as_str(),
                    )
                });
                let result = result.await;

                if let Some(this) = this.upgrade() {
                    this.update(cx, |this, cx| {
                        let Some(load_epoch) = this
                            .review_editor_session(path.as_str())
                            .map(|session| session.load_epoch)
                        else {
                            return;
                        };
                        if epoch != load_epoch {
                            return;
                        }

                        if let Some(session) = this.review_editor_session_mut(path.as_str()) {
                            session.loading = false;
                        }
                        match result {
                            Ok(document) => {
                                let absolute_path = project_root.join(document.path.as_str());
                                let right_is_dirty = this
                                    .review_editor_session(path.as_str())
                                    .is_some_and(|session| session.right_editor.borrow().is_dirty());
                                let preserve_dirty_right = should_preserve_dirty_review_editor_right(
                                    previous_path.as_deref(),
                                    previous_left_source_id.as_deref(),
                                    previous_right_source_id.as_deref(),
                                    document.path.as_str(),
                                    left_source_id.as_deref(),
                                    right_source_id.as_deref(),
                                    right_is_dirty,
                                );
                                let (left_result, right_result) = if let Some(session) =
                                    this.review_editor_session_mut(path.as_str())
                                {
                                    let left_result = session.left_editor.borrow_mut().sync_document(
                                        &absolute_path,
                                        document.left_text.as_str(),
                                        true,
                                    );
                                    let right_result = if preserve_dirty_right {
                                        Ok(())
                                    } else {
                                        session.right_editor.borrow_mut().sync_document(
                                            &absolute_path,
                                            document.right_text.as_str(),
                                            true,
                                        )
                                    };
                                    (left_result, right_result)
                                } else {
                                    return;
                                };

                                match left_result.and(right_result) {
                                    Ok(()) => {
                                        if let Some(session) =
                                            this.review_editor_session_mut(path.as_str())
                                        {
                                            session.left_present = document.left_present;
                                            session.right_present =
                                                document.right_present || preserve_dirty_right;
                                            if !preserve_dirty_right {
                                                session.save_loading = false;
                                                session.last_saved_text =
                                                    Some(document.right_text.clone());
                                            }
                                            session.error = None;
                                        }
                                        debug!(
                                            path = path.as_str(),
                                            left = left_source_id.as_deref().unwrap_or("unknown"),
                                            right = right_source_id.as_deref().unwrap_or("unknown"),
                                            preserve_dirty_right,
                                            elapsed_ms = load_started_at.elapsed().as_millis(),
                                            "review editor document loaded"
                                        );
                                        this.request_review_editor_presentation_refresh_for_path(
                                            path.as_str(),
                                            false,
                                            cx,
                                        );
                                    }
                                    Err(err) => {
                                        if let Some(session) =
                                            this.review_editor_session_mut(path.as_str())
                                        {
                                            session.error = Some(format!(
                                                "Review editor preview unavailable: {err:#}"
                                            ));
                                            session.left_present = false;
                                            session.right_present = false;
                                            session.presentation_loading = false;
                                            session.save_loading = false;
                                            session.last_saved_text = None;
                                            session.right_hunk_lines.clear();
                                            session.right_to_left_line_map.clear();
                                            session.left_editor.borrow_mut().clear();
                                            session.right_editor.borrow_mut().clear();
                                        }
                                        error!(
                                            path = path.as_str(),
                                            left = left_source_id.as_deref().unwrap_or("unknown"),
                                            right = right_source_id.as_deref().unwrap_or("unknown"),
                                            elapsed_ms = load_started_at.elapsed().as_millis(),
                                            "review editor document apply failed: {err:#}"
                                        );
                                    }
                                }
                            }
                            Err(err) => {
                                if let Some(session) = this.review_editor_session_mut(path.as_str()) {
                                    session.error = Some(format!(
                                        "Review editor preview unavailable: {err:#}"
                                    ));
                                    session.left_present = false;
                                    session.right_present = false;
                                    session.presentation_loading = false;
                                    session.save_loading = false;
                                    session.last_saved_text = None;
                                    session.right_hunk_lines.clear();
                                    session.right_to_left_line_map.clear();
                                    session.left_editor.borrow_mut().clear();
                                    session.right_editor.borrow_mut().clear();
                                }
                                error!(
                                    path = path.as_str(),
                                    left = left_source_id.as_deref().unwrap_or("unknown"),
                                    right = right_source_id.as_deref().unwrap_or("unknown"),
                                    elapsed_ms = load_started_at.elapsed().as_millis(),
                                    "review editor document load failed: {err:#}"
                                );
                            }
                        }

                        cx.notify();
                    });
                }
            });
        }
    }

}
