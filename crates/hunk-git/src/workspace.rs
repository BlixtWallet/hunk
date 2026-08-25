use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

use crate::git::{
    ChangedFile, LineStats, LocalBranch, load_remote_tracking_branches_without_refresh,
    load_repo_file_line_stats_without_refresh, load_workflow_snapshot_if_changed_without_refresh,
};
use crate::history::{
    DEFAULT_RECENT_AUTHORED_COMMIT_LIMIT, RecentCommitSummary, load_recent_authored_commits,
};
use crate::mutation::{
    activate_or_create_branch, commit_index_with_details, restore_working_copy_paths, stage_paths,
    unstage_paths,
};
use crate::network::{
    fetch_remote_branches, pull_current_branch_with_rebase, push_current_branch,
    sync_current_branch,
};

#[derive(Debug, Clone)]
pub struct GitWorkspaceSnapshot {
    pub root: PathBuf,
    pub branch_name: String,
    pub branch_has_upstream: bool,
    pub branch_ahead_count: usize,
    pub branch_behind_count: usize,
    pub files: Vec<ChangedFile>,
    pub branches: Vec<LocalBranch>,
    pub remote_branches: Vec<LocalBranch>,
    pub file_line_stats: BTreeMap<String, LineStats>,
    pub recent_commits: Vec<RecentCommitSummary>,
    pub last_commit_subject: Option<String>,
}

pub fn load_git_workspace(root: &Path) -> Result<GitWorkspaceSnapshot> {
    let (_, workflow) = load_workflow_snapshot_if_changed_without_refresh(root, None)?;
    let workflow = workflow.ok_or_else(|| {
        anyhow!("Git workspace load completed without returning a repository snapshot")
    })?;
    let remote_branches = load_remote_tracking_branches_without_refresh(root)?;
    let file_line_stats = if workflow.files.is_empty() {
        BTreeMap::new()
    } else {
        load_repo_file_line_stats_without_refresh(root)?
    };
    let recent_commits = load_recent_authored_commits(
        workflow.root.as_path(),
        DEFAULT_RECENT_AUTHORED_COMMIT_LIMIT,
    )?
    .commits;

    Ok(GitWorkspaceSnapshot {
        root: workflow.root,
        branch_name: workflow.branch_name,
        branch_has_upstream: workflow.branch_has_upstream,
        branch_ahead_count: workflow.branch_ahead_count,
        branch_behind_count: workflow.branch_behind_count,
        files: workflow.files,
        branches: workflow.branches,
        remote_branches,
        file_line_stats,
        recent_commits,
        last_commit_subject: workflow.last_commit_subject,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitWorkspaceCommand {
    StagePaths(Vec<String>),
    UnstagePaths(Vec<String>),
    RestorePaths(Vec<String>),
    CommitStaged { message: String },
    ActivateBranch { name: String },
    FetchRemoteBranches,
    PublishBranch { name: String },
    PushBranch { name: String },
    SyncBranch { name: String },
    PullBranchWithRebase { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorkspaceCommandOutcome {
    pub message: String,
}

pub fn execute_git_workspace_command(
    root: &Path,
    command: GitWorkspaceCommand,
) -> Result<GitWorkspaceCommandOutcome> {
    let message = match command {
        GitWorkspaceCommand::StagePaths(paths) => {
            let count = paths.len();
            stage_paths(root, paths.as_slice())?;
            format!("Staged {count} {}", item_label(count, "file", "files"))
        }
        GitWorkspaceCommand::UnstagePaths(paths) => {
            let count = paths.len();
            unstage_paths(root, paths.as_slice())?;
            format!("Unstaged {count} {}", item_label(count, "file", "files"))
        }
        GitWorkspaceCommand::RestorePaths(paths) => {
            let count = restore_working_copy_paths(root, paths.as_slice())?;
            format!(
                "Discarded changes in {count} {}",
                item_label(count, "file", "files")
            )
        }
        GitWorkspaceCommand::CommitStaged { message } => {
            let commit = commit_index_with_details(root, message.as_str())?;
            format!("Committed {}", commit.subject)
        }
        GitWorkspaceCommand::ActivateBranch { name } => {
            activate_or_create_branch(root, name.as_str(), false)?;
            format!("Activated branch {name}")
        }
        GitWorkspaceCommand::FetchRemoteBranches => {
            let count = fetch_remote_branches(root)?;
            format!(
                "Fetched remote branches from {count} {}",
                item_label(count, "remote", "remotes")
            )
        }
        GitWorkspaceCommand::PublishBranch { name } => {
            push_current_branch(root, name.as_str(), false)?;
            format!("Published branch {name}")
        }
        GitWorkspaceCommand::PushBranch { name } => {
            push_current_branch(root, name.as_str(), true)?;
            format!("Pushed branch {name}")
        }
        GitWorkspaceCommand::SyncBranch { name } => {
            sync_current_branch(root, name.as_str())?;
            format!("Synced branch {name}")
        }
        GitWorkspaceCommand::PullBranchWithRebase { name } => {
            pull_current_branch_with_rebase(root, name.as_str())?;
            format!("Rebased branch {name} onto upstream")
        }
    };

    Ok(GitWorkspaceCommandOutcome { message })
}

const fn item_label<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 { singular } else { plural }
}
