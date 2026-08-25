use std::collections::{BTreeSet, HashMap};

use hunk_app::diff::{
    DiffCommentAnchor, DiffCommentScope, DiffCommentStoreCommand, DiffCommentStoreSnapshot,
};
use hunk_domain::comments::CommentLineSide;
use hunk_domain::db::{CommentRecord, CommentStatus};
use hunk_qt::DiffCommentProjection;

fn scope() -> DiffCommentScope {
    DiffCommentScope {
        repo_root: "/repo".to_owned(),
        branch_name: "feature".to_owned(),
    }
}

fn comment(id: &str, path: &str, status: CommentStatus, line: u32) -> CommentRecord {
    CommentRecord {
        id: id.to_owned(),
        repo_root: "/repo".to_owned(),
        branch_name: "feature".to_owned(),
        created_head_commit: None,
        status,
        file_path: path.to_owned(),
        line_side: CommentLineSide::Right,
        old_line: Some(line),
        new_line: Some(line),
        row_stable_id: Some(u64::from(line)),
        hunk_header: Some(format!("@@ -{line} +{line} @@")),
        line_text: format!("+let value_{line} = true;"),
        context_before: " let before = true;".to_owned(),
        context_after: " let after = true;".to_owned(),
        anchor_hash: format!("hash-{line}"),
        comment_text: format!("review {id}"),
        stale_reason: None,
        created_at_unix_ms: 1,
        updated_at_unix_ms: 1,
        last_seen_at_unix_ms: None,
        resolved_at_unix_ms: None,
    }
}

fn anchor(path: &str, line: u32) -> DiffCommentAnchor {
    DiffCommentAnchor {
        stable_id: u64::from(line),
        file_path: path.to_owned(),
        line_side: CommentLineSide::Right,
        old_line: Some(line),
        new_line: Some(line),
        hunk_header: Some(format!("@@ -{line} +{line} @@")),
        line_text: format!("+let value_{line} = true;"),
        context_before: " let before = true;".to_owned(),
        context_after: " let after = true;".to_owned(),
        anchor_hash: format!("hash-{line}"),
    }
}

fn projection(
    records: Vec<CommentRecord>,
    anchors: Vec<Option<DiffCommentAnchor>>,
    changed_paths: &[&str],
) -> DiffCommentProjection {
    DiffCommentProjection::from_store_snapshot(
        scope(),
        DiffCommentStoreSnapshot {
            comments: records,
            status_message: None,
        },
        anchors.as_slice(),
        changed_paths
            .iter()
            .map(|path| (*path).to_owned())
            .collect(),
        false,
    )
}

#[test]
fn projection_counts_matches_filters_and_clipboard_payloads() {
    let projection = projection(
        vec![
            comment("open", "src/current.rs", CommentStatus::Open, 10),
            comment("stale", "src/other.rs", CommentStatus::Stale, 20),
            comment("resolved", "src/removed.rs", CommentStatus::Resolved, 30),
        ],
        vec![Some(anchor("src/current.rs", 10))],
        &["src/current.rs", "src/other.rs"],
    );

    assert_eq!(projection.open_count, 1);
    assert_eq!(projection.stale_count, 1);
    assert_eq!(projection.resolved_count, 1);
    assert_eq!(projection.row_count(0), 1);
    assert_eq!(projection.row_count(20), 0);
    assert_eq!(projection.visible_items(false).len(), 1);
    assert_eq!(projection.visible_items(true).len(), 3);
    assert!(projection.visible_items(false)[0].can_jump);
    assert!(projection.all_open_clipboard_text().contains("review open"));
}

#[test]
fn visible_comment_items_are_bounded_while_counts_cover_every_record() {
    let records = (0..70)
        .map(|line| {
            let id = format!("comment-{line}");
            comment(id.as_str(), "src/current.rs", CommentStatus::Open, line + 1)
        })
        .collect();
    let projection = projection(records, Vec::new(), &["src/current.rs"]);

    assert_eq!(projection.open_count, 70);
    assert_eq!(projection.visible_items(false).len(), 64);
}

#[test]
fn reconciliation_uses_two_misses_and_defers_unloaded_changed_files() {
    let mut unmatched_current = comment("stale", "src/current.rs", CommentStatus::Open, 100);
    unmatched_current.hunk_header = Some("@@ -100 +100 @@".to_owned());
    unmatched_current.line_text = "+completely unrelated content".to_owned();
    unmatched_current.context_before = " unrelated before".to_owned();
    unmatched_current.context_after = " unrelated after".to_owned();
    unmatched_current.anchor_hash = "unmatched-anchor".to_owned();
    let projection = projection(
        vec![
            comment("seen", "src/current.rs", CommentStatus::Open, 10),
            unmatched_current,
            comment("deferred", "src/other.rs", CommentStatus::Open, 12),
            comment("resolved", "src/removed.rs", CommentStatus::Open, 13),
        ],
        vec![Some(anchor("src/current.rs", 10))],
        &["src/current.rs", "src/other.rs"],
    );
    let mut miss_streaks = HashMap::new();

    let first = projection
        .reconcile_command(Some("src/current.rs"), &mut miss_streaks)
        .expect("matched comments should be touched immediately");
    let DiffCommentStoreCommand::Reconcile {
        seen_ids,
        stale_ids,
        resolved_ids,
    } = first
    else {
        panic!("expected reconcile command");
    };
    assert_eq!(seen_ids, ["seen"]);
    assert!(stale_ids.is_empty());
    assert!(resolved_ids.is_empty());
    assert_eq!(miss_streaks.get("stale"), Some(&1));
    assert_eq!(miss_streaks.get("resolved"), Some(&1));
    assert!(!miss_streaks.contains_key("deferred"));

    let second = projection
        .reconcile_command(Some("src/current.rs"), &mut miss_streaks)
        .expect("second misses should update comment status");
    let DiffCommentStoreCommand::Reconcile {
        seen_ids,
        stale_ids,
        resolved_ids,
    } = second
    else {
        panic!("expected reconcile command");
    };
    assert_eq!(seen_ids, ["seen"]);
    assert_eq!(stale_ids, ["stale"]);
    assert_eq!(resolved_ids, ["resolved"]);
    assert!(miss_streaks.is_empty());
}

#[test]
fn reconciliation_waits_until_a_changed_diff_is_loaded() {
    let projection = projection(
        vec![comment(
            "waiting",
            "src/current.rs",
            CommentStatus::Open,
            10,
        )],
        Vec::new(),
        &["src/current.rs"],
    );
    let mut miss_streaks = HashMap::new();

    assert!(
        projection
            .reconcile_command(None, &mut miss_streaks)
            .is_none()
    );
    assert!(miss_streaks.is_empty());
}

#[test]
fn reconciliation_preserves_old_path_comments_while_a_rename_is_present() {
    let projection = DiffCommentProjection::from_store_snapshot(
        scope(),
        DiffCommentStoreSnapshot {
            comments: vec![comment(
                "old-path",
                "src/old_name.rs",
                CommentStatus::Open,
                10,
            )],
            status_message: None,
        },
        &[],
        BTreeSet::from(["src/new_name.rs".to_owned()]),
        true,
    );
    let mut miss_streaks = HashMap::new();

    assert!(
        projection
            .reconcile_command(Some("src/new_name.rs"), &mut miss_streaks)
            .is_none()
    );
    assert!(miss_streaks.is_empty());
}
