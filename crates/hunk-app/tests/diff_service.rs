use std::collections::{BTreeMap, BTreeSet};

use hunk_app::diff::{
    CachedStyledSegment, DiffCommand, DiffProjectionOptions, DiffSegmentQuality, DiffStreamRowKind,
    SyntaxTokenKind, build_diff_row_segment_cache_from_cells, build_diff_stream_from_patch_map,
    compact_cached_segments_for_render, load_diff_snapshot,
};
use hunk_domain::diff::DiffCellKind;
use hunk_git::git::{ChangedFile, FileStatus};

#[test]
fn historical_turn_command_returns_comparison_and_stable_projection() {
    let patch = "\
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1 +1,2 @@
-old
+new
+extra
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -5 +5 @@
-before
+after
";

    let first = load_diff_snapshot(
        DiffCommand::HistoricalTurn {
            patch: patch.to_string(),
        },
        DiffProjectionOptions::default(),
    )
    .expect("historical Diff snapshot should load");
    let second = load_diff_snapshot(
        DiffCommand::HistoricalTurn {
            patch: patch.to_string(),
        },
        DiffProjectionOptions::default(),
    )
    .expect("repeated historical Diff snapshot should load");

    assert_eq!(first.comparison.files.len(), 2);
    assert_eq!(first.comparison.overall_line_stats.added, 3);
    assert_eq!(first.comparison.overall_line_stats.removed, 2);
    assert_eq!(
        first
            .projection
            .row_metadata
            .iter()
            .map(|row| row.stable_id)
            .collect::<Vec<_>>(),
        second
            .projection
            .row_metadata
            .iter()
            .map(|row| row.stable_id)
            .collect::<Vec<_>>()
    );
}

#[test]
fn historical_turn_command_recovers_add_delete_and_rename_statuses() {
    let patch = "\
diff --git a/src/new.rs b/src/new.rs
new file mode 100644
--- /dev/null
+++ b/src/new.rs
@@ -0,0 +1 @@
+hello
diff --git a/src/old.rs b/src/old.rs
deleted file mode 100644
--- a/src/old.rs
+++ /dev/null
@@ -1 +0,0 @@
-goodbye
diff --git a/src/from.rs b/src/to.rs
rename from src/from.rs
rename to src/to.rs
--- a/src/from.rs
+++ b/src/to.rs
@@ -1 +1 @@
-before
+after
";

    let snapshot = load_diff_snapshot(
        DiffCommand::HistoricalTurn {
            patch: patch.to_string(),
        },
        DiffProjectionOptions::default(),
    )
    .expect("historical Diff snapshot should load");

    assert_eq!(snapshot.comparison.files[0].status, FileStatus::Added);
    assert_eq!(snapshot.comparison.files[1].status, FileStatus::Deleted);
    assert_eq!(snapshot.comparison.files[2].status, FileStatus::Renamed);
    assert_eq!(snapshot.comparison.files[2].path, "src/to.rs");
}

#[test]
fn historical_turn_command_merges_duplicate_and_pathless_sections() {
    let duplicate_patch = "\
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1 +1 @@
-one
+two
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -3 +3 @@
-three
+four
";
    let duplicate = load_diff_snapshot(
        DiffCommand::HistoricalTurn {
            patch: duplicate_patch.to_string(),
        },
        DiffProjectionOptions::default(),
    )
    .expect("duplicate-path historical Diff should load");
    assert_eq!(duplicate.comparison.files.len(), 1);
    assert_eq!(duplicate.comparison.file_line_stats["src/lib.rs"].added, 2);
    assert!(duplicate.comparison.patches_by_path["src/lib.rs"].contains("@@ -3 +3 @@"));

    let pathless = load_diff_snapshot(
        DiffCommand::HistoricalTurn {
            patch: "@@ -1 +1 @@\n-old\n+new\n".to_string(),
        },
        DiffProjectionOptions::default(),
    )
    .expect("pathless historical Diff should load");
    assert_eq!(
        pathless.comparison.files[0].path,
        "historical-turn-diff.patch"
    );
}

#[test]
fn projection_emits_owned_binary_error_rows() {
    let files = vec![ChangedFile {
        path: "assets/image.png".to_string(),
        status: FileStatus::Modified,
        staged: false,
        unstaged: true,
        untracked: false,
    }];
    let stream = build_diff_stream_from_patch_map(
        &files,
        &BTreeSet::new(),
        &BTreeMap::new(),
        &BTreeMap::new(),
        &BTreeSet::new(),
    );

    assert!(stream.row_metadata.iter().any(|row| {
        row.kind == DiffStreamRowKind::FileError
            && row.file_path.as_deref() == Some("assets/image.png")
    }));
    assert!(
        stream
            .rows
            .iter()
            .any(|row| row.text.contains("binary file type"))
    );
}

#[test]
fn detailed_segments_preserve_syntax_and_intra_line_changes() {
    let cache = build_diff_row_segment_cache_from_cells(
        Some("src/main.rs"),
        "let answer = 42;",
        DiffCellKind::Removed,
        "let answer = 7;",
        DiffCellKind::Added,
        DiffSegmentQuality::Detailed,
    );

    assert!(
        cache
            .left
            .iter()
            .any(|segment| segment.syntax == SyntaxTokenKind::Keyword)
    );
    assert!(cache.left.iter().any(|segment| segment.changed));
    assert!(cache.right.iter().any(|segment| segment.changed));
}

#[test]
fn segment_compaction_preserves_text_and_large_identical_pairs() {
    let expected = (0..20).map(|ix| format!("part-{ix}|")).collect::<String>();
    let segments = (0..20)
        .map(|ix| CachedStyledSegment {
            plain_text: format!("part-{ix}|"),
            syntax: if ix % 2 == 0 {
                SyntaxTokenKind::Keyword
            } else {
                SyntaxTokenKind::String
            },
            changed: ix % 3 == 0,
            search_match: false,
        })
        .collect::<Vec<_>>();
    let compacted = compact_cached_segments_for_render(segments, 6);
    assert!(compacted.len() <= 6);
    assert_eq!(
        compacted
            .iter()
            .map(|segment| segment.plain_text.as_str())
            .collect::<String>(),
        expected
    );

    let line = "token ".repeat(8_300);
    let cache = build_diff_row_segment_cache_from_cells(
        Some("src/main.rs"),
        line.as_str(),
        DiffCellKind::Removed,
        line.as_str(),
        DiffCellKind::Added,
        DiffSegmentQuality::Detailed,
    );
    assert!(cache.left.iter().all(|segment| !segment.changed));
    assert!(cache.right.iter().all(|segment| !segment.changed));
}
