extern crate self as hunk_domain;

pub mod diff {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DiffRowKind {
        Code,
        HunkHeader,
        Meta,
        Empty,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SideBySideRow {
        pub kind: DiffRowKind,
    }
}

#[path = "../src/app/review_preview_model.rs"]
mod review_preview_model;

use diff::{DiffRowKind, SideBySideRow};
use review_preview_model::{
    REVIEW_PREVIEW_MAX_RENDER_HUNKS, REVIEW_PREVIEW_MAX_RENDER_ROWS, build_review_preview_section,
};

fn row(kind: DiffRowKind) -> SideBySideRow {
    SideBySideRow { kind }
}

#[test]
fn preview_section_keeps_small_files_untruncated() {
    let rows = vec![
        row(DiffRowKind::Meta),
        row(DiffRowKind::HunkHeader),
        row(DiffRowKind::Code),
        row(DiffRowKind::Code),
    ];

    let section = build_review_preview_section(0..rows.len(), &rows);

    assert_eq!(section.total_row_count, 4);
    assert_eq!(section.rendered_row_count, 4);
    assert_eq!(section.total_hunk_count, 1);
    assert_eq!(section.rendered_hunk_count, 1);
    assert!(!section.truncated());
}

#[test]
fn preview_section_caps_large_files_to_excerpt_budget() {
    let mut rows = Vec::new();
    for _ in 0..(REVIEW_PREVIEW_MAX_RENDER_HUNKS + 2) {
        rows.push(row(DiffRowKind::HunkHeader));
        for _ in 0..20 {
            rows.push(row(DiffRowKind::Code));
        }
    }

    let section = build_review_preview_section(0..rows.len(), &rows);

    assert_eq!(
        section.total_hunk_count,
        REVIEW_PREVIEW_MAX_RENDER_HUNKS + 2
    );
    assert!(section.truncated());
    assert!(section.rendered_row_count <= REVIEW_PREVIEW_MAX_RENDER_ROWS);
    assert!(section.rendered_hunk_count <= REVIEW_PREVIEW_MAX_RENDER_HUNKS);
    assert!(section.hidden_row_count() > 0);
    assert!(section.hidden_hunk_count() > 0);
}
