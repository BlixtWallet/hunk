use hunk_domain::diff::{DiffRowKind, SideBySideRow};

pub(crate) const REVIEW_PREVIEW_MAX_RENDER_ROWS: usize = 96;
pub(crate) const REVIEW_PREVIEW_MAX_RENDER_HUNKS: usize = 6;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReviewPreviewSection {
    pub(crate) rendered_row_indices: Vec<usize>,
    pub(crate) total_row_count: usize,
    pub(crate) rendered_row_count: usize,
    pub(crate) total_hunk_count: usize,
    pub(crate) rendered_hunk_count: usize,
}

impl ReviewPreviewSection {
    pub(crate) fn truncated(&self) -> bool {
        self.rendered_row_count < self.total_row_count
    }

    pub(crate) fn hidden_row_count(&self) -> usize {
        self.total_row_count.saturating_sub(self.rendered_row_count)
    }

    pub(crate) fn hidden_hunk_count(&self) -> usize {
        self.total_hunk_count
            .saturating_sub(self.rendered_hunk_count)
    }
}

pub(crate) fn build_review_preview_section(
    row_indices: std::ops::Range<usize>,
    diff_rows: &[SideBySideRow],
) -> ReviewPreviewSection {
    let mut section = ReviewPreviewSection::default();
    let mut rendered_hunks = 0usize;
    let mut truncated = false;

    for row_ix in row_indices {
        let Some(row) = diff_rows.get(row_ix) else {
            continue;
        };

        section.total_row_count = section.total_row_count.saturating_add(1);
        let is_hunk_header = row.kind == DiffRowKind::HunkHeader;
        if is_hunk_header {
            section.total_hunk_count = section.total_hunk_count.saturating_add(1);
        }

        if truncated {
            continue;
        }

        if section.rendered_row_indices.len() >= REVIEW_PREVIEW_MAX_RENDER_ROWS
            || (is_hunk_header && rendered_hunks >= REVIEW_PREVIEW_MAX_RENDER_HUNKS)
        {
            truncated = true;
            continue;
        }

        if is_hunk_header {
            rendered_hunks = rendered_hunks.saturating_add(1);
        }

        section.rendered_row_indices.push(row_ix);
    }

    section.rendered_row_count = section.rendered_row_indices.len();
    section.rendered_hunk_count = rendered_hunks;
    section
}
