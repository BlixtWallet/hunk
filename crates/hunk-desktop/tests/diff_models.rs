use hunk_desktop::{DiffFileSummary, DiffSnapshotPayload};
use hunk_domain::comments::CommentLineSide;
use hunk_git::git::{FileStatus, LineStats};

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
    assert!(payload.rows[1].left_markup.contains("@keyword@"));
    assert!(payload.rows[1].left_markup.contains("&nbsp;"));
    assert_eq!(payload.matching_rows(" NEW "), vec![1]);
    assert_eq!(payload.matching_rows("@@"), vec![0]);
    assert!(payload.matching_rows("missing").is_empty());
    assert_eq!(payload.selection_text(0, 1), "-let old = 1;\n+let new = 2;");
    assert_eq!(payload.selection_text(1, 0), "-let old = 1;\n+let new = 2;");
    assert!(payload.selection_text(-1, -1).is_empty());
    let anchor = payload
        .comment_anchor(1)
        .expect("changed row should carry its comment anchor");
    assert_eq!(anchor.stable_id.to_string(), payload.rows[1].stable_id);
    assert_eq!(anchor.file_path, "src/main.rs");
    assert_eq!(anchor.line_side, CommentLineSide::Right);
    assert_eq!(anchor.hunk_header.as_deref(), Some("@@ -1 +1 @@"));
    assert!(payload.comment_anchor(0).is_none());
    assert!(payload.comment_anchor(-1).is_none());
}

#[test]
fn hunk_navigation_wraps_in_both_directions() {
    let summary = DiffFileSummary {
        path: "src/main.rs".to_owned(),
        status: FileStatus::Modified,
        line_stats: LineStats {
            added: 2,
            removed: 2,
        },
    };
    let payload = DiffSnapshotPayload::from_patch(
        &summary,
        "@@ -1 +1 @@\n-let one = 1;\n+let one = 2;\n@@ -10 +10 @@\n-let ten = 10;\n+let ten = 11;\n",
    );

    assert_eq!(payload.hunk_target(-1, 1), 0);
    assert_eq!(payload.hunk_target(0, 1), 2);
    assert_eq!(payload.hunk_target(2, 1), 0);
    assert_eq!(payload.hunk_target(-1, -1), 2);
    assert_eq!(payload.hunk_target(2, -1), 0);
    assert_eq!(payload.hunk_target(0, -1), 2);
}

#[test]
fn syntax_markup_escapes_code_before_qml_renders_it() {
    let summary = DiffFileSummary {
        path: "src/main.rs".to_owned(),
        status: FileStatus::Modified,
        line_stats: LineStats {
            added: 1,
            removed: 1,
        },
    };
    let payload = DiffSnapshotPayload::from_patch(
        &summary,
        "@@ -1 +1 @@\n-let old = a < b && c > d;\n+let new = \"@keyword@\";\n",
    );

    assert!(payload.rows[1].left_markup.contains("&lt;"));
    assert!(payload.rows[1].left_markup.contains("&amp;"));
    assert!(payload.rows[1].left_markup.contains("&gt;"));
    assert!(
        payload.rows[1]
            .right_markup
            .contains("&quot;&#64;keyword&#64;&quot;")
    );
    assert!(!payload.rows[1].left_markup.contains("a < b"));
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
