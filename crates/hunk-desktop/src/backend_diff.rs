use std::sync::Arc;

use crate::Backend;
use crate::diff_models::DiffFileSummary;

impl Backend {
    pub(super) fn clear_diff_search_results(&mut self) {
        self.diff_search_matches.clear();
        self.diff_search_match_count = 0;
        self.diff_search_match_index = -1;
        self.diff_search_target_row = -1;
    }

    pub(super) fn rebuild_diff_search_results(&mut self) {
        self.diff_search_matches = self
            .diff_rows
            .borrow()
            .matching_rows(self.diff_search_query.as_str());
        self.diff_search_match_count =
            i32::try_from(self.diff_search_matches.len()).unwrap_or(i32::MAX);
        if let Some(target) = self.diff_search_matches.first().copied() {
            self.diff_search_match_index = 0;
            self.diff_search_target_row = i32::try_from(target).unwrap_or(i32::MAX);
        } else {
            self.diff_search_match_index = -1;
            self.diff_search_target_row = -1;
        }
    }

    pub(super) fn replace_diff_files(
        &mut self,
        files: Vec<crate::git_models::GitFileItem>,
        summaries: Vec<DiffFileSummary>,
    ) {
        self.diff_epoch = self.diff_epoch.wrapping_add(1).max(1);
        let previous_path = self.diff_selected_path.clone();
        self.diff_loading = false;
        self.diff_ready = false;
        self.diff_error.clear();
        self.diff_rows.borrow_mut().replace(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Arc::new(Vec::new()),
        );
        self.clear_diff_comment_row_state();
        self.clear_diff_search_results();
        self.diff_files.borrow_mut().replace(files);
        self.diff_file_summaries = summaries
            .into_iter()
            .map(|summary| (summary.path.clone(), summary))
            .collect();

        let selected = self
            .diff_file_summaries
            .get(previous_path.as_str())
            .cloned()
            .or_else(|| {
                self.diff_file_summaries
                    .values()
                    .min_by(|left, right| left.path.cmp(&right.path))
                    .cloned()
            });
        if let Some(summary) = selected {
            self.apply_diff_selection(&summary);
        } else {
            self.diff_selected_path.clear();
            self.diff_status_tag.clear();
            self.diff_additions = 0;
            self.diff_removals = 0;
            self.diff_ready = true;
            self.diff_state_changed();
        }
        self.diff_comments_state_changed();
    }

    pub(super) fn apply_diff_selection(&mut self, summary: &DiffFileSummary) {
        self.diff_epoch = self.diff_epoch.wrapping_add(1).max(1);
        self.diff_loading = false;
        self.diff_rows.borrow_mut().replace(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Arc::new(Vec::new()),
        );
        self.clear_diff_comment_row_state();
        self.clear_diff_search_results();
        self.diff_selected_path = summary.path.clone();
        self.diff_status_tag = summary.status.tag().to_owned();
        self.diff_additions = i32::try_from(summary.line_stats.added).unwrap_or(i32::MAX);
        self.diff_removals = i32::try_from(summary.line_stats.removed).unwrap_or(i32::MAX);
        self.diff_ready = false;
        self.diff_error.clear();
        self.diff_state_changed();
        self.diff_comments_state_changed();
    }

    pub(super) fn reset_diff_state(&mut self) {
        self.diff_epoch = self.diff_epoch.wrapping_add(1).max(1);
        self.diff_files.borrow_mut().replace(Vec::new());
        self.diff_rows.borrow_mut().replace(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Arc::new(Vec::new()),
        );
        self.clear_diff_comment_row_state();
        self.diff_selected_path.clear();
        self.diff_status_tag.clear();
        self.diff_additions = 0;
        self.diff_removals = 0;
        self.diff_ready = false;
        self.diff_loading = false;
        self.diff_error.clear();
        self.diff_file_summaries.clear();
        self.diff_search_query.clear();
        self.clear_diff_search_results();
        self.diff_state_changed();
        self.diff_comments_state_changed();
    }
}
