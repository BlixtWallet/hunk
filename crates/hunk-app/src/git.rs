use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use hunk_git::git::{
    ChangedFile, LineStats, LocalBranch, RepoSnapshotFingerprint, WorkflowSnapshot,
    load_remote_tracking_branches_without_refresh, load_repo_file_line_stats_without_refresh,
    load_workflow_snapshot_if_changed, load_workflow_snapshot_if_changed_without_refresh,
    load_workflow_snapshot_with_fingerprint,
    load_workflow_snapshot_with_fingerprint_without_refresh,
};
use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SnapshotRefreshPriority {
    Background,
    UserInitiated,
}

impl SnapshotRefreshPriority {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::UserInitiated => "user",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SnapshotRefreshBehavior {
    ReadOnly,
    RefreshWorkingCopy,
}

impl SnapshotRefreshBehavior {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::RefreshWorkingCopy => "refresh-working-copy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotRefreshRequest {
    pub force: bool,
    pub priority: SnapshotRefreshPriority,
    pub behavior: SnapshotRefreshBehavior,
}

impl SnapshotRefreshRequest {
    pub const fn user(force: bool) -> Self {
        Self {
            force,
            priority: SnapshotRefreshPriority::UserInitiated,
            behavior: SnapshotRefreshBehavior::RefreshWorkingCopy,
        }
    }

    pub const fn background() -> Self {
        Self {
            force: false,
            priority: SnapshotRefreshPriority::Background,
            behavior: SnapshotRefreshBehavior::ReadOnly,
        }
    }

    pub const fn background_refresh_working_copy() -> Self {
        Self {
            force: false,
            priority: SnapshotRefreshPriority::Background,
            behavior: SnapshotRefreshBehavior::RefreshWorkingCopy,
        }
    }

    pub fn merge(self, other: Self) -> Self {
        Self {
            force: self.force || other.force,
            priority: self.priority.max(other.priority),
            behavior: self.behavior.max(other.behavior),
        }
    }

    pub fn is_more_urgent_than(self, other: Self) -> bool {
        self.priority > other.priority
            || (self.priority == other.priority && self.behavior > other.behavior)
            || (self.priority == other.priority
                && self.behavior == other.behavior
                && self.force
                && !other.force)
    }
}

#[derive(Debug, Clone)]
pub enum SnapshotRefreshResult {
    Unchanged(RepoSnapshotFingerprint),
    Loaded {
        fingerprint: RepoSnapshotFingerprint,
        workflow: Box<WorkflowSnapshot>,
        loaded_without_refresh: bool,
    },
}

pub fn load_snapshot_refresh(
    source_dir: &Path,
    previous_fingerprint: Option<&RepoSnapshotFingerprint>,
    request: SnapshotRefreshRequest,
    prefer_stale_first: bool,
) -> Result<SnapshotRefreshResult> {
    let primary = snapshot_load_path(request.behavior, prefer_stale_first);
    match load_snapshot_for_path(primary, source_dir, previous_fingerprint) {
        Ok(snapshot) => Ok(snapshot),
        Err(primary_error) if request.behavior == SnapshotRefreshBehavior::RefreshWorkingCopy => {
            warn!(
                "snapshot stale-first load failed; retrying with working-copy refresh: {primary_error:#}"
            );
            load_snapshot_for_path(
                snapshot_fallback_load_path(prefer_stale_first),
                source_dir,
                previous_fingerprint,
            )
            .map_err(|fallback_error| {
                primary_error.context(format!("snapshot fallback load failed: {fallback_error:#}"))
            })
        }
        Err(error) => Err(error),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SnapshotLoadPath {
    WithFingerprintWithoutRefresh,
    IfChangedWithoutRefresh,
    WithFingerprintRefreshWorkingCopy,
    IfChangedRefreshWorkingCopy,
}

const fn snapshot_load_path(
    behavior: SnapshotRefreshBehavior,
    prefer_stale_first: bool,
) -> SnapshotLoadPath {
    match (behavior, prefer_stale_first) {
        (SnapshotRefreshBehavior::ReadOnly, true) => {
            SnapshotLoadPath::WithFingerprintWithoutRefresh
        }
        (SnapshotRefreshBehavior::ReadOnly, false) => SnapshotLoadPath::IfChangedWithoutRefresh,
        (SnapshotRefreshBehavior::RefreshWorkingCopy, true) => {
            SnapshotLoadPath::WithFingerprintWithoutRefresh
        }
        (SnapshotRefreshBehavior::RefreshWorkingCopy, false) => {
            SnapshotLoadPath::IfChangedRefreshWorkingCopy
        }
    }
}

const fn snapshot_fallback_load_path(prefer_stale_first: bool) -> SnapshotLoadPath {
    if prefer_stale_first {
        SnapshotLoadPath::WithFingerprintRefreshWorkingCopy
    } else {
        SnapshotLoadPath::IfChangedRefreshWorkingCopy
    }
}

fn load_snapshot_for_path(
    load_path: SnapshotLoadPath,
    source_dir: &Path,
    previous_fingerprint: Option<&RepoSnapshotFingerprint>,
) -> Result<SnapshotRefreshResult> {
    match load_path {
        SnapshotLoadPath::WithFingerprintWithoutRefresh => {
            let (fingerprint, workflow) =
                load_workflow_snapshot_with_fingerprint_without_refresh(source_dir)?;
            Ok(SnapshotRefreshResult::Loaded {
                fingerprint,
                workflow: Box::new(workflow),
                loaded_without_refresh: true,
            })
        }
        SnapshotLoadPath::IfChangedWithoutRefresh => {
            let (fingerprint, workflow) = load_workflow_snapshot_if_changed_without_refresh(
                source_dir,
                previous_fingerprint,
            )?;
            Ok(match workflow {
                Some(workflow) => SnapshotRefreshResult::Loaded {
                    fingerprint,
                    workflow: Box::new(workflow),
                    loaded_without_refresh: true,
                },
                None => SnapshotRefreshResult::Unchanged(fingerprint),
            })
        }
        SnapshotLoadPath::WithFingerprintRefreshWorkingCopy => {
            let (fingerprint, workflow) = load_workflow_snapshot_with_fingerprint(source_dir)?;
            Ok(SnapshotRefreshResult::Loaded {
                fingerprint,
                workflow: Box::new(workflow),
                loaded_without_refresh: false,
            })
        }
        SnapshotLoadPath::IfChangedRefreshWorkingCopy => {
            let (fingerprint, workflow) =
                load_workflow_snapshot_if_changed(source_dir, previous_fingerprint)?;
            Ok(match workflow {
                Some(workflow) => SnapshotRefreshResult::Loaded {
                    fingerprint,
                    workflow: Box::new(workflow),
                    loaded_without_refresh: false,
                },
                None => SnapshotRefreshResult::Unchanged(fingerprint),
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorkspaceRefreshRequest {
    pub root: PathBuf,
    pub refresh_recent_commits: bool,
}

impl GitWorkspaceRefreshRequest {
    pub fn new(root: PathBuf, refresh_recent_commits: bool) -> Self {
        Self {
            root,
            refresh_recent_commits,
        }
    }

    pub fn merge(self, newer: Self) -> Self {
        if self.root == newer.root {
            Self {
                root: newer.root,
                refresh_recent_commits: self.refresh_recent_commits || newer.refresh_recent_commits,
            }
        } else {
            newer
        }
    }
}

#[derive(Debug, Clone)]
pub struct GitWorkspaceRefreshSnapshot {
    pub fingerprint: RepoSnapshotFingerprint,
    pub workflow: Option<WorkflowSnapshot>,
    pub remote_branches: Vec<LocalBranch>,
    pub file_line_stats: BTreeMap<String, LineStats>,
}

pub fn load_git_workspace_refresh(
    root: &Path,
    previous_fingerprint: Option<&RepoSnapshotFingerprint>,
) -> Result<GitWorkspaceRefreshSnapshot> {
    let (fingerprint, workflow) =
        load_workflow_snapshot_if_changed_without_refresh(root, previous_fingerprint)?;
    let remote_branches = load_remote_tracking_branches_without_refresh(root)?;
    let file_line_stats = match workflow.as_ref() {
        Some(workflow) if !workflow.files.is_empty() => {
            load_repo_file_line_stats_without_refresh(root)?
        }
        _ => BTreeMap::new(),
    };

    Ok(GitWorkspaceRefreshSnapshot {
        fingerprint,
        workflow,
        remote_branches,
        file_line_stats,
    })
}

pub const fn repo_watch_refresh_request(
    metadata_changed: bool,
    has_dirty_paths: bool,
) -> Option<SnapshotRefreshRequest> {
    if has_dirty_paths {
        return Some(SnapshotRefreshRequest::background_refresh_working_copy());
    }
    if metadata_changed {
        return Some(SnapshotRefreshRequest::background());
    }
    None
}

pub const fn should_refresh_line_stats_after_snapshot(
    request: SnapshotRefreshRequest,
    diff_state_changed: bool,
) -> bool {
    diff_state_changed
        && !matches!(
            (request.priority, request.behavior),
            (
                SnapshotRefreshPriority::Background,
                SnapshotRefreshBehavior::ReadOnly
            )
        )
}

pub const fn diff_state_changed(
    root_changed: bool,
    working_copy_commit_changed: bool,
    file_list_changed: bool,
) -> bool {
    root_changed || working_copy_commit_changed || file_list_changed
}

pub const fn should_reload_diff_after_snapshot(
    supports_diff_stream: bool,
    diff_state_changed: bool,
    diff_rows_empty: bool,
) -> bool {
    supports_diff_stream && (diff_state_changed || diff_rows_empty)
}

pub const fn should_scroll_selected_after_reload(
    selected_changed: bool,
    diff_rows_empty: bool,
) -> bool {
    selected_changed || diff_rows_empty
}

pub const fn should_reload_repo_tree_after_snapshot(
    root_changed: bool,
    supports_sidebar_tree: bool,
    file_list_changed: bool,
) -> bool {
    root_changed || (supports_sidebar_tree && file_list_changed)
}

pub const fn should_run_cold_start_reconcile(
    cold_start: bool,
    loaded_without_refresh: bool,
    behavior: SnapshotRefreshBehavior,
) -> bool {
    cold_start
        && loaded_without_refresh
        && matches!(behavior, SnapshotRefreshBehavior::RefreshWorkingCopy)
}

pub const fn should_request_startup_git_workspace_refresh(
    selected_target_is_primary: bool,
) -> bool {
    !selected_target_is_primary
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitActionRefreshPlan {
    pub refresh_primary_snapshot: bool,
    pub refresh_git_workspace: bool,
    pub refresh_recent_commits: bool,
}

pub const fn git_action_refresh_plan(
    selected_root_is_primary: bool,
    refresh_recent_commits: bool,
) -> GitActionRefreshPlan {
    GitActionRefreshPlan {
        refresh_primary_snapshot: selected_root_is_primary,
        refresh_git_workspace: !selected_root_is_primary,
        refresh_recent_commits,
    }
}

pub fn post_git_action_refresh_plan(
    action_name: &str,
    selected_root_is_primary: bool,
) -> GitActionRefreshPlan {
    if action_name == "Fetch remote branches" {
        return GitActionRefreshPlan {
            refresh_primary_snapshot: false,
            refresh_git_workspace: true,
            refresh_recent_commits: false,
        };
    }

    git_action_refresh_plan(
        selected_root_is_primary,
        matches!(
            action_name,
            "Activate branch" | "Sync branch" | "Pull branch --rebase"
        ),
    )
}

pub fn missing_line_stat_paths(
    files: &[ChangedFile],
    file_line_stats: &BTreeMap<String, LineStats>,
) -> BTreeSet<String> {
    files
        .iter()
        .filter(|file| !file_line_stats.contains_key(file.path.as_str()))
        .map(|file| file.path.clone())
        .collect()
}

pub fn line_stats_paths_from_dirty_paths(
    files: &[ChangedFile],
    pending_dirty_paths: &BTreeSet<String>,
) -> BTreeSet<String> {
    if pending_dirty_paths.is_empty() {
        return BTreeSet::new();
    }

    files
        .iter()
        .filter(|file| {
            pending_dirty_paths.iter().any(|dirty_path| {
                file.path == *dirty_path
                    || file
                        .path
                        .strip_prefix(dirty_path.as_str())
                        .is_some_and(|suffix| suffix.starts_with('/'))
            })
        })
        .map(|file| file.path.clone())
        .collect()
}
