use std::collections::BTreeSet;

use anyhow::{Result, bail};
use hunk_domain::db::{CommentRecord, CommentStatus, DatabaseStore, NewComment, now_unix_ms};

use super::DiffCommentAnchor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffCommentScope {
    pub repo_root: String,
    pub branch_name: String,
}

#[derive(Debug, Clone)]
pub enum DiffCommentStoreCommand {
    Load {
        prune_before_unix_ms: Option<i64>,
    },
    Create {
        anchor: DiffCommentAnchor,
        comment_text: String,
    },
    SetStatus {
        id: String,
        status: CommentStatus,
    },
    Delete {
        id: String,
    },
    Reconcile {
        seen_ids: Vec<String>,
        stale_ids: Vec<String>,
        resolved_ids: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub struct DiffCommentStoreSnapshot {
    pub comments: Vec<CommentRecord>,
    pub status_message: Option<String>,
}

pub fn execute_diff_comment_store_command(
    store: &DatabaseStore,
    scope: &DiffCommentScope,
    command: DiffCommentStoreCommand,
) -> Result<DiffCommentStoreSnapshot> {
    if scope.repo_root.trim().is_empty() {
        bail!("comment scope requires a repository root");
    }
    if scope.branch_name.trim().is_empty() {
        bail!("comment scope requires a branch name");
    }

    let status_message = match command {
        DiffCommentStoreCommand::Load {
            prune_before_unix_ms,
        } => {
            if let Some(cutoff) = prune_before_unix_ms {
                store.prune_non_open_comments(cutoff)?;
            }
            None
        }
        DiffCommentStoreCommand::Create {
            anchor,
            comment_text,
        } => {
            let comment_text = comment_text.trim();
            if comment_text.is_empty() {
                bail!("comment text cannot be empty");
            }
            store.create_comment(&NewComment {
                repo_root: scope.repo_root.clone(),
                branch_name: scope.branch_name.clone(),
                created_head_commit: None,
                file_path: anchor.file_path,
                line_side: anchor.line_side,
                old_line: anchor.old_line,
                new_line: anchor.new_line,
                row_stable_id: Some(anchor.stable_id),
                hunk_header: anchor.hunk_header,
                line_text: anchor.line_text,
                context_before: anchor.context_before,
                context_after: anchor.context_after,
                anchor_hash: anchor.anchor_hash,
                comment_text: comment_text.to_owned(),
            })?;
            Some("Comment added.".to_owned())
        }
        DiffCommentStoreCommand::SetStatus { id, status } => {
            ensure_comment_in_scope(store, scope, id.as_str())?;
            if !store.mark_comment_status(id.as_str(), status, None, now_unix_ms())? {
                bail!("comment {id} was not found while updating its status");
            }
            Some(
                match status {
                    CommentStatus::Open => "Comment reopened.",
                    CommentStatus::Stale => "Comment marked stale.",
                    CommentStatus::Resolved => "Comment resolved.",
                }
                .to_owned(),
            )
        }
        DiffCommentStoreCommand::Delete { id } => {
            ensure_comment_in_scope(store, scope, id.as_str())?;
            if !store.delete_comment(id.as_str())? {
                bail!("comment {id} was not found while deleting it");
            }
            Some("Comment deleted.".to_owned())
        }
        DiffCommentStoreCommand::Reconcile {
            seen_ids,
            stale_ids,
            resolved_ids,
        } => {
            ensure_reconcile_ids_in_scope(store, scope, [&seen_ids, &stale_ids, &resolved_ids])?;
            let now = now_unix_ms();
            store.touch_many_comment_seen(&seen_ids, now)?;
            store.mark_many_comment_status(
                &stale_ids,
                CommentStatus::Stale,
                Some("anchor_not_found"),
                now,
            )?;
            store.mark_many_comment_status(&resolved_ids, CommentStatus::Resolved, None, now)?;
            None
        }
    };

    Ok(DiffCommentStoreSnapshot {
        comments: store.list_comments(
            scope.repo_root.as_str(),
            scope.branch_name.as_str(),
            true,
        )?,
        status_message,
    })
}

fn ensure_comment_in_scope(
    store: &DatabaseStore,
    scope: &DiffCommentScope,
    id: &str,
) -> Result<()> {
    let Some(comment) = store.get_comment(id)? else {
        bail!("comment {id} was not found");
    };
    if comment.repo_root != scope.repo_root || comment.branch_name != scope.branch_name {
        bail!("comment {id} is outside the active repository and branch scope");
    }
    Ok(())
}

fn ensure_reconcile_ids_in_scope(
    store: &DatabaseStore,
    scope: &DiffCommentScope,
    groups: [&Vec<String>; 3],
) -> Result<()> {
    let scoped_ids = store
        .list_comments(scope.repo_root.as_str(), scope.branch_name.as_str(), true)?
        .into_iter()
        .map(|comment| comment.id)
        .collect::<BTreeSet<_>>();
    let mut requested = BTreeSet::new();
    for id in groups.into_iter().flatten() {
        if !requested.insert(id.as_str()) {
            bail!("comment {id} appears in more than one reconcile group");
        }
        if !scoped_ids.contains(id) {
            bail!("comment {id} is outside the active repository and branch scope");
        }
    }
    Ok(())
}
