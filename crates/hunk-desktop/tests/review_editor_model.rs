extern crate self as hunk_domain;
extern crate self as hunk_editor;

pub mod diff {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DiffCellKind {
        None,
        Added,
        Removed,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DiffRowKind {
        Code,
        Meta,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct DiffCell {
        pub line: Option<u32>,
        pub text: String,
        pub kind: DiffCellKind,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SideBySideRow {
        pub kind: DiffRowKind,
        pub left: DiffCell,
        pub right: DiffCell,
        pub text: String,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OverlayKind {
    DiffAddition,
    DiffDeletion,
    DiffModification,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayDescriptor {
    pub line: usize,
    pub kind: OverlayKind,
    pub message: Option<String>,
}

#[path = "../src/app/review_editor_model.rs"]
mod review_editor_model;

use diff::{DiffCell, DiffCellKind, DiffRowKind, SideBySideRow};
use review_editor_model::{build_review_editor_overlays, build_review_editor_overlays_from_texts};

#[test]
fn review_editor_overlays_mark_modified_and_added_lines() {
    let rows = vec![
        SideBySideRow {
            kind: DiffRowKind::Code,
            left: DiffCell {
                line: Some(4),
                text: "before".to_string(),
                kind: DiffCellKind::Removed,
            },
            right: DiffCell {
                line: Some(4),
                text: "after".to_string(),
                kind: DiffCellKind::Added,
            },
            text: String::new(),
        },
        SideBySideRow {
            kind: DiffRowKind::Code,
            left: DiffCell {
                line: None,
                text: String::new(),
                kind: DiffCellKind::None,
            },
            right: DiffCell {
                line: Some(9),
                text: "new".to_string(),
                kind: DiffCellKind::Added,
            },
            text: String::new(),
        },
    ];

    let (left, right) = build_review_editor_overlays(&rows);

    assert_eq!(left.len(), 1);
    assert_eq!(left[0].line, 3);
    assert_eq!(left[0].kind, OverlayKind::DiffModification);
    assert_eq!(right.len(), 2);
    assert_eq!(right[0].line, 3);
    assert_eq!(right[0].kind, OverlayKind::DiffModification);
    assert_eq!(right[1].line, 8);
    assert_eq!(right[1].kind, OverlayKind::DiffAddition);
}

#[test]
fn review_editor_overlays_mark_removed_only_lines_on_left() {
    let rows = vec![
        SideBySideRow {
            kind: DiffRowKind::Meta,
            left: DiffCell {
                line: Some(1),
                text: "@@".to_string(),
                kind: DiffCellKind::None,
            },
            right: DiffCell {
                line: Some(1),
                text: "@@".to_string(),
                kind: DiffCellKind::None,
            },
            text: String::new(),
        },
        SideBySideRow {
            kind: DiffRowKind::Code,
            left: DiffCell {
                line: Some(12),
                text: "deleted".to_string(),
                kind: DiffCellKind::Removed,
            },
            right: DiffCell {
                line: None,
                text: String::new(),
                kind: DiffCellKind::None,
            },
            text: String::new(),
        },
    ];

    let (left, right) = build_review_editor_overlays(&rows);

    assert_eq!(left.len(), 1);
    assert_eq!(left[0].line, 11);
    assert_eq!(left[0].kind, OverlayKind::DiffDeletion);
    assert!(right.is_empty());
}

#[test]
fn text_overlays_pair_changed_blocks_as_modifications() {
    let left = "alpha\nbeta\ngamma\n";
    let right = "alpha\nbeta changed\ngamma\n";

    let (left_overlays, right_overlays) = build_review_editor_overlays_from_texts(left, right);

    assert_eq!(left_overlays.len(), 1);
    assert_eq!(left_overlays[0].line, 1);
    assert_eq!(left_overlays[0].kind, OverlayKind::DiffModification);
    assert_eq!(right_overlays.len(), 1);
    assert_eq!(right_overlays[0].line, 1);
    assert_eq!(right_overlays[0].kind, OverlayKind::DiffModification);
}

#[test]
fn text_overlays_mark_insertions_and_deletions() {
    let left = "alpha\nbeta\ngamma\n";
    let right = "alpha\ninserted\nbeta\n";

    let (left_overlays, right_overlays) = build_review_editor_overlays_from_texts(left, right);

    assert_eq!(left_overlays.len(), 1);
    assert_eq!(left_overlays[0].line, 2);
    assert_eq!(left_overlays[0].kind, OverlayKind::DiffDeletion);
    assert_eq!(right_overlays.len(), 1);
    assert_eq!(right_overlays[0].line, 1);
    assert_eq!(right_overlays[0].kind, OverlayKind::DiffAddition);
}
