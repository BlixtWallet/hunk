use crate::Backend;
use crate::forge::review_state_label;

impl Backend {
    pub(super) fn apply_review_summary(&mut self, review: Option<hunk_forge::OpenReviewSummary>) {
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
}
