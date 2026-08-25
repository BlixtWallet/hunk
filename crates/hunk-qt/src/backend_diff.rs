use crate::Backend;

impl Backend {
    pub(super) fn clear_diff_search_results(&mut self) {
        self.diff_search_matches.clear();
        self.diff_search_match_count = 0;
        self.diff_search_match_index = -1;
        self.diff_search_target_row = -1;
    }
}
