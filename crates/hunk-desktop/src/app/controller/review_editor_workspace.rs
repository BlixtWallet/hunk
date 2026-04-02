const REVIEW_EDITOR_PREFETCH_RADIUS: usize = 0;

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

    pub(super) fn request_review_preview_segment_prefetch_for_visible_files(
        &mut self,
        visible_range: std::ops::Range<usize>,
        force_upgrade: bool,
        cx: &mut Context<Self>,
    ) {
        if self.workspace_view_mode != WorkspaceViewMode::Diff || self.file_row_ranges.is_empty() {
            return;
        }

        let start_ix = visible_range.start.min(self.file_row_ranges.len());
        let end_ix = visible_range.end.min(self.file_row_ranges.len());
        if !force_upgrade && self.last_review_visible_file_range == Some((start_ix, end_ix)) {
            return;
        }
        self.last_review_visible_file_range = Some((start_ix, end_ix));
        for ix in start_ix..end_ix {
            let Some(path) = self.review_files.get(ix).map(|file| file.path.as_str()) else {
                continue;
            };
            let Some((first_row, last_row)) = self
                .review_preview_sections
                .get(path)
                .map(|section| {
                    (
                        section.rendered_row_indices.first().copied(),
                        section.rendered_row_indices.last().copied(),
                    )
                })
            else {
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

    fn request_review_editor_reload(&mut self, force: bool, cx: &mut Context<Self>) {
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

        self.review_editor_list_state.scroll_to_reveal_item(center_ix);
        self.request_review_editor_prefetch_around_index(center_ix, REVIEW_EDITOR_PREFETCH_RADIUS, force, cx);
        cx.notify();
    }
}
