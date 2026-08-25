use hunk_git::git::{FileStatus, LineStats};
use hunk_qt::{DiffFileSummary, DiffSnapshotPayload};

#[test]
fn side_by_side_payload_preserves_lines_and_cell_kinds() {
    let summary = DiffFileSummary {
        path: "src/main.rs".to_owned(),
        status: FileStatus::Modified,
        line_stats: LineStats {
            added: 1,
            removed: 1,
        },
    };
    let payload =
        DiffSnapshotPayload::from_patch(&summary, "@@ -1 +1 @@\n-let old = 1;\n+let new = 2;\n");

    assert_eq!(payload.path, "src/main.rs");
    assert_eq!(payload.rows[0].row_kind, "hunk");
    assert!(!payload.rows[0].stable_id.is_empty());
    assert_eq!(payload.rows[1].left_kind, "removed");
    assert_eq!(payload.rows[1].right_kind, "added");
    assert_eq!(payload.rows[1].left_line, 1);
    assert_eq!(payload.rows[1].right_line, 1);
}

#[test]
fn binary_patch_becomes_a_single_explanatory_row() {
    let summary = DiffFileSummary {
        path: "image.png".to_owned(),
        status: FileStatus::Modified,
        line_stats: LineStats::default(),
    };
    let payload = DiffSnapshotPayload::from_patch(
        &summary,
        "Binary files a/image.png and b/image.png differ\n",
    );

    assert_eq!(payload.rows.len(), 1);
    assert_eq!(payload.rows[0].row_kind, "meta");
    assert!(payload.rows[0].text.contains("binary"));
}
