const REVIEW_EDITOR_PREFETCH_RADIUS: usize = 0;
const REVIEW_EDITOR_VISIBLE_PREFETCH_OVERSCAN: usize = 2;
const REVIEW_EDITOR_SESSION_RETENTION_RADIUS: usize = 12;
const REVIEW_EDITOR_SESSION_SOFT_LIMIT: usize = 64;

fn review_editor_prefetch_range(
    file_count: usize,
    center_ix: usize,
    radius: usize,
) -> std::ops::RangeInclusive<usize> {
    if file_count == 0 {
        return 0..=0;
    }
    let center_ix = center_ix.min(file_count.saturating_sub(1));
    let start = center_ix.saturating_sub(radius);
    let end = center_ix
        .saturating_add(radius)
        .min(file_count.saturating_sub(1));
    start..=end
}

impl DiffViewer {
    pub(super) fn request_review_preview_segment_prefetch_for_visible_files(
        &mut self,
        visible_range: std::ops::Range<usize>,
        force_upgrade: bool,
        cx: &mut Context<Self>,
    ) {
        if self.workspace_view_mode != WorkspaceViewMode::Diff || self.file_row_ranges.is_empty() {
            return;
        }

        let start_ix = visible_range.start.min(self.review_files.len());
        let end_ix = visible_range.end.min(self.review_files.len());
        if start_ix >= end_ix {
            return;
        }
        if !force_upgrade && self.last_review_visible_file_range == Some((start_ix, end_ix)) {
            return;
        }

        for ix in start_ix..end_ix {
            let Some(path) = self.review_files.get(ix).map(|file| file.path.as_str()) else {
                continue;
            };
            let Some((first_row, last_row)) = self.review_preview_sections.get(path).map(|section| {
                (
                    section.rendered_row_indices.first().copied(),
                    section.rendered_row_indices.last().copied(),
                )
            }) else {
                continue;
            };

            if let Some(first_row) = first_row {
                self.request_visible_row_segment_prefetch(first_row, force_upgrade, cx);
            }
            if force_upgrade {
                continue;
            }
            if let Some(last_row) = last_row && first_row != Some(last_row) {
                self.request_visible_row_segment_prefetch(last_row, false, cx);
            }
        }
    }

    fn update_review_visible_file_range(&mut self, visible_range: std::ops::Range<usize>) {
        let start_ix = visible_range.start.min(self.review_files.len());
        let end_ix = visible_range.end.min(self.review_files.len());
        self.last_review_visible_file_range = Some((start_ix, end_ix));
        for ix in start_ix..end_ix {
            let Some(path) = self.review_files.get(ix).map(|file| file.path.clone()) else {
                continue;
            };
            self.touch_review_editor_session(path.as_str());
        }
        if let Some(selected_path) = self.selected_path.clone() {
            self.touch_review_editor_session(selected_path.as_str());
        }
    }

    fn prune_review_editor_sessions_after_scroll_settles(&mut self) {
        if self.workspace_view_mode != WorkspaceViewMode::Diff {
            return;
        }
        let session_count = self.review_editor_sessions.len();
        if session_count <= REVIEW_EDITOR_SESSION_SOFT_LIMIT {
            return;
        }
        let Some((start_ix, end_ix)) = self.last_review_visible_file_range else {
            return;
        };
        let keep_start_ix = start_ix.saturating_sub(REVIEW_EDITOR_SESSION_RETENTION_RADIUS);
        let keep_end_ix = end_ix
            .saturating_add(REVIEW_EDITOR_SESSION_RETENTION_RADIUS)
            .min(self.review_files.len());
        let mut keep_paths = std::collections::BTreeSet::new();
        if let Some(selected_path) = self.selected_path.as_deref() {
            keep_paths.insert(selected_path.to_string());
        }
        for ix in keep_start_ix..keep_end_ix {
            if let Some(path) = self.review_files.get(ix).map(|file| file.path.clone()) {
                keep_paths.insert(path);
            }
        }

        let mut removable_paths: Vec<(String, Instant)> = self
            .review_editor_sessions
            .iter()
            .filter_map(|(path, session)| {
                if keep_paths.contains(path)
                    || session.loading
                    || session.presentation_loading
                    || session.save_loading
                {
                    return None;
                }
                Some((path.clone(), session.last_touched_at))
            })
            .collect();

        let target_evict_count = session_count.saturating_sub(REVIEW_EDITOR_SESSION_SOFT_LIMIT);
        if target_evict_count == 0 || removable_paths.is_empty() {
            return;
        }

        removable_paths.sort_by_key(|(_, last_touched_at)| *last_touched_at);
        let evict_paths: Vec<String> = removable_paths
            .into_iter()
            .take(target_evict_count)
            .map(|(path, _)| path)
            .collect();
        let evicted_count = evict_paths.len();
        for path in evict_paths {
            if let Some(mut session) = self.review_editor_sessions.remove(path.as_str()) {
                self.review_editor_evicted_paths.insert(path);
                Self::reset_review_editor_file_session(&mut session);
            }
        }
        if evicted_count > 0 {
            debug!(
                visible_start_ix = start_ix,
                visible_end_ix = end_ix,
                keep_start_ix,
                keep_end_ix,
                before_sessions = session_count,
                evicted_count,
                remaining_sessions = self.review_editor_sessions.len(),
                session_soft_limit = REVIEW_EDITOR_SESSION_SOFT_LIMIT,
                "review editor sessions pruned"
            );
        }
    }

    fn request_review_editor_prefetch_for_visible_files(
        &mut self,
        visible_range: std::ops::Range<usize>,
        _max_requests: usize,
        cx: &mut Context<Self>,
    ) {
        if self.workspace_view_mode != WorkspaceViewMode::Diff || self.review_files.is_empty() {
            return;
        }

        let start_ix = visible_range.start.min(self.review_files.len());
        let end_ix = visible_range.end.min(self.review_files.len());
        if start_ix >= end_ix {
            return;
        }
        let Some(selected_path) = self.selected_path.clone() else {
            return;
        };
        let Some(selected_ix) = self
            .review_files
            .iter()
            .position(|file| file.path == selected_path)
        else {
            return;
        };
        let prefetch_start_ix = start_ix.saturating_sub(REVIEW_EDITOR_VISIBLE_PREFETCH_OVERSCAN);
        let prefetch_end_ix = end_ix
            .saturating_add(REVIEW_EDITOR_VISIBLE_PREFETCH_OVERSCAN)
            .min(self.review_files.len());
        if selected_ix < prefetch_start_ix || selected_ix >= prefetch_end_ix {
            return;
        }

        self.request_review_editor_reload_for_path(selected_path.as_str(), false, cx);
    }

    fn rebuild_review_preview_sections(&mut self) {
        self.review_preview_sections = self
            .file_row_ranges
            .iter()
            .map(|range| {
                let start = range.start_row.saturating_add(1).min(range.end_row);
                let section = crate::app::review_preview_model::build_review_preview_section(
                    start..range.end_row,
                    &self.diff_rows,
                );
                (range.path.clone(), section)
            })
            .collect();
    }

    fn request_review_editor_prefetch_around_index(
        &mut self,
        center_ix: usize,
        radius: usize,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        if self.review_files.is_empty() {
            return;
        }

        let range = review_editor_prefetch_range(self.review_files.len(), center_ix, radius);
        let start_ix = *range.start();
        let end_ix = *range.end();
        let mut requested = 0usize;
        for ix in range {
            let Some(path) = self.review_files.get(ix).map(|file| file.path.clone()) else {
                continue;
            };
            let already_ready = self
                .review_editor_session(path.as_str())
                .is_some_and(|session| {
                    session.left_source_id == self.review_left_source_id
                        && session.right_source_id == self.review_right_source_id
                        && session.error.is_none()
                        && (session.loading
                            || session.left_editor.borrow().current_text().is_some()
                            || session.right_editor.borrow().current_text().is_some())
                });
            if !force && already_ready {
                continue;
            }
            requested = requested.saturating_add(1);
            self.request_review_editor_reload_for_path(path.as_str(), force, cx);
        }
        if requested > 0 {
            debug!(
                center_ix,
                start_ix,
                end_ix,
                force,
                requested,
                total_files = self.review_files.len(),
                "review editor prefetch scheduled"
            );
        }
    }

    fn request_review_editor_workspace_reload(&mut self, force: bool, cx: &mut Context<Self>) {
        if self.workspace_view_mode != WorkspaceViewMode::Diff {
            self.clear_review_editor_session();
            return;
        }

        self.sync_review_editor_sessions_to_files();
        if self.review_files.is_empty() {
            return;
        }

        let center_ix = self
            .selected_path
            .as_deref()
            .and_then(|path| self.review_files.iter().position(|file| file.path == path))
            .unwrap_or(0);

        let view = cx.entity().downgrade();
        cx.defer(move |cx| {
            let Some(view) = view.upgrade() else {
                return;
            };
            view.update(cx, |this, cx| {
                if this.workspace_view_mode != WorkspaceViewMode::Diff {
                    return;
                }
                this.request_review_editor_prefetch_around_index(
                    center_ix,
                    REVIEW_EDITOR_PREFETCH_RADIUS,
                    force,
                    cx,
                );
            });
        });
    }

    fn request_review_editor_reload_with_options(
        &mut self,
        force: bool,
        reveal_selected: bool,
        cx: &mut Context<Self>,
    ) {
        if self.workspace_view_mode != WorkspaceViewMode::Diff {
            self.clear_review_editor_session();
            return;
        }

        self.sync_review_editor_sessions_to_files();
        let Some(path) = self.selected_path.clone() else {
            self.clear_review_editor_session();
            return;
        };
        let Some(center_ix) = self.review_files.iter().position(|file| file.path == path) else {
            self.clear_review_editor_session();
            return;
        };

        if reveal_selected {
            self.review_editor_list_state.scroll_to_reveal_item(center_ix);
        }
        self.request_review_editor_prefetch_around_index(center_ix, REVIEW_EDITOR_PREFETCH_RADIUS, force, cx);
        cx.notify();
    }

    fn request_review_editor_reload(&mut self, force: bool, cx: &mut Context<Self>) {
        self.request_review_editor_reload_with_options(force, true, cx);
    }

    pub(super) fn request_review_editor_reload_preserving_scroll(
        &mut self,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        self.request_review_editor_reload_with_options(force, false, cx);
    }
}
