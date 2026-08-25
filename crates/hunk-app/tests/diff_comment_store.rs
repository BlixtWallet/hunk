#![cfg(feature = "comment-store")]

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use hunk_app::diff::{
    DiffCommentAnchor, DiffCommentScope, DiffCommentStoreCommand,
    execute_diff_comment_store_command,
};
use hunk_domain::comments::CommentLineSide;
use hunk_domain::db::{CommentStatus, DatabaseStore};

struct TempDb {
    path: PathBuf,
    store: DatabaseStore,
}

impl TempDb {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "hunk-comment-service-{}-{unique}.db",
            std::process::id()
        ));
        Self {
            store: DatabaseStore::from_path(path.clone()),
            path,
        }
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_file(self.path.with_extension("db-shm"));
        let _ = fs::remove_file(self.path.with_extension("db-wal"));
    }
}

fn scope(repo_root: &str, branch_name: &str) -> DiffCommentScope {
    DiffCommentScope {
        repo_root: repo_root.to_owned(),
        branch_name: branch_name.to_owned(),
    }
}

fn anchor(path: &str, stable_id: u64) -> DiffCommentAnchor {
    DiffCommentAnchor {
        stable_id,
        file_path: path.to_owned(),
        line_side: CommentLineSide::Right,
        old_line: Some(10),
        new_line: Some(11),
        hunk_header: Some("@@ -10,2 +11,3 @@".to_owned()),
        line_text: "+let value = 1;".to_owned(),
        context_before: " let value = 0;".to_owned(),
        context_after: " return value;".to_owned(),
        anchor_hash: format!("anchor-{stable_id}"),
    }
}

#[test]
fn store_commands_create_update_delete_and_reload_the_active_scope() {
    let fixture = TempDb::new();
    let active_scope = scope("/repo", "feature");

    let created = execute_diff_comment_store_command(
        &fixture.store,
        &active_scope,
        DiffCommentStoreCommand::Create {
            anchor: anchor("src/lib.rs", 42),
            comment_text: "  explain this change  ".to_owned(),
        },
    )
    .expect("create comment through service");
    assert_eq!(created.status_message.as_deref(), Some("Comment added."));
    assert_eq!(created.comments.len(), 1);
    assert_eq!(created.comments[0].comment_text, "explain this change");
    assert_eq!(created.comments[0].row_stable_id, Some(42));

    let id = created.comments[0].id.clone();
    let resolved = execute_diff_comment_store_command(
        &fixture.store,
        &active_scope,
        DiffCommentStoreCommand::SetStatus {
            id: id.clone(),
            status: CommentStatus::Resolved,
        },
    )
    .expect("resolve comment through service");
    assert_eq!(resolved.comments[0].status, CommentStatus::Resolved);

    let deleted = execute_diff_comment_store_command(
        &fixture.store,
        &active_scope,
        DiffCommentStoreCommand::Delete { id },
    )
    .expect("delete comment through service");
    assert!(deleted.comments.is_empty());
}

#[test]
fn mutations_reject_comment_ids_outside_the_active_scope() {
    let fixture = TempDb::new();
    let source_scope = scope("/repo", "main");
    let created = execute_diff_comment_store_command(
        &fixture.store,
        &source_scope,
        DiffCommentStoreCommand::Create {
            anchor: anchor("src/lib.rs", 7),
            comment_text: "keep scoped".to_owned(),
        },
    )
    .expect("create scoped comment");

    let error = execute_diff_comment_store_command(
        &fixture.store,
        &scope("/repo", "other"),
        DiffCommentStoreCommand::Delete {
            id: created.comments[0].id.clone(),
        },
    )
    .expect_err("cross-branch delete must be rejected");
    assert!(error.to_string().contains("outside the active"));
    assert_eq!(
        fixture
            .store
            .list_comments("/repo", "main", true)
            .expect("reload original scope")
            .len(),
        1
    );
}

#[test]
fn reconcile_updates_seen_stale_and_resolved_comments_by_scope() {
    let fixture = TempDb::new();
    let active_scope = scope("/repo", "main");
    let mut ids = Vec::new();
    for stable_id in 1..=3 {
        let snapshot = execute_diff_comment_store_command(
            &fixture.store,
            &active_scope,
            DiffCommentStoreCommand::Create {
                anchor: anchor(format!("src/{stable_id}.rs").as_str(), stable_id),
                comment_text: format!("comment {stable_id}"),
            },
        )
        .expect("create reconcile fixture");
        ids.push(
            snapshot
                .comments
                .iter()
                .find(|comment| comment.row_stable_id == Some(stable_id))
                .expect("created comment should be listed")
                .id
                .clone(),
        );
    }

    let reconciled = execute_diff_comment_store_command(
        &fixture.store,
        &active_scope,
        DiffCommentStoreCommand::Reconcile {
            seen_ids: vec![ids[0].clone()],
            stale_ids: vec![ids[1].clone()],
            resolved_ids: vec![ids[2].clone()],
        },
    )
    .expect("reconcile comment statuses");

    assert!(
        reconciled
            .comments
            .iter()
            .find(|comment| comment.id == ids[0])
            .and_then(|comment| comment.last_seen_at_unix_ms)
            .is_some()
    );
    assert_eq!(
        reconciled
            .comments
            .iter()
            .find(|comment| comment.id == ids[1])
            .map(|comment| comment.status),
        Some(CommentStatus::Stale)
    );
    assert_eq!(
        reconciled
            .comments
            .iter()
            .find(|comment| comment.id == ids[2])
            .map(|comment| comment.status),
        Some(CommentStatus::Resolved)
    );
}
