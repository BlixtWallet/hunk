use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::AtomicI32;
use std::sync::{Arc, Mutex};

use hunk_domain::state::AppStateStore;
use hunk_forge::{ForgeReviewOutcome, ForgeReviewWorkspace, GitHubDeviceAuthorization};
use qtbridge::QObjectHolder;

use crate::forge::ForgeSnapshotPayload;
use crate::git_models::{
    GitBranchListModel, GitCommitListModel, GitFileListModel, GitSnapshotPayload,
};

pub(super) type GitRefreshResult = Result<GitSnapshotPayload, String>;
pub(super) type ForgeAsyncResult = Result<ForgeAsyncPayload, String>;
type GitHubDeviceStartResult = Result<GitHubDeviceAuthorization, String>;

pub(super) enum ForgeAsyncPayload {
    Snapshot(Box<ForgeSnapshotPayload>),
    Review(ForgeReviewOutcome),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Workspace {
    Diff,
    Git,
    Ai,
}

impl Workspace {
    pub const ALL: [Self; 3] = [Self::Diff, Self::Git, Self::Ai];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Diff => "diff",
            Self::Git => "git",
            Self::Ai => "ai",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "diff" => Some(Self::Diff),
            "git" => Some(Self::Git),
            "ai" => Some(Self::Ai),
            _ => None,
        }
    }
}

pub struct Backend {
    pub(super) active_workspace: String,
    pub(super) ready: bool,
    pub(super) status_message: String,
    pub(super) bootstrap_started: bool,
    pub(super) git_files: Rc<RefCell<GitFileListModel>>,
    pub(super) git_branches: Rc<RefCell<GitBranchListModel>>,
    pub(super) git_commits: Rc<RefCell<GitCommitListModel>>,
    pub(super) git_root: String,
    pub(super) git_repository_name: String,
    pub(super) git_branch_name: String,
    pub(super) git_branch_has_upstream: bool,
    pub(super) git_branch_ahead_count: i32,
    pub(super) git_branch_behind_count: i32,
    pub(super) git_changed_file_count: i32,
    pub(super) git_staged_file_count: i32,
    pub(super) git_unstaged_file_count: i32,
    pub(super) git_last_commit_subject: String,
    pub(super) git_ready: bool,
    pub(super) git_loading: bool,
    pub(super) git_busy: bool,
    pub(super) git_error: String,
    pub(super) git_status_message: String,
    pub(super) git_action_label: String,
    pub(super) git_epoch: i32,
    pub(super) git_refresh_results: Arc<Mutex<HashMap<i32, GitRefreshResult>>>,
    pub(super) git_root_pending_persist: bool,
    pub(super) git_staged_paths: Vec<String>,
    pub(super) git_unstaged_paths: Vec<String>,
    pub(super) forge_available: bool,
    pub(super) forge_provider_label: String,
    pub(super) forge_review_kind_label: String,
    pub(super) forge_host: String,
    pub(super) forge_repository_path: String,
    pub(super) forge_authenticated: bool,
    pub(super) forge_account_label: String,
    pub(super) forge_auth_mode: String,
    pub(super) forge_ready: bool,
    pub(super) forge_loading: bool,
    pub(super) forge_busy: bool,
    pub(super) forge_error: String,
    pub(super) forge_status_message: String,
    pub(super) forge_action_label: String,
    pub(super) forge_default_target_branch: String,
    pub(super) forge_review_exists: bool,
    pub(super) forge_review_number: i32,
    pub(super) forge_review_title: String,
    pub(super) forge_review_url: String,
    pub(super) forge_review_state: String,
    pub(super) forge_review_draft: bool,
    pub(super) forge_device_flow_active: bool,
    pub(super) forge_device_user_code: String,
    pub(super) forge_device_verification_url: String,
    pub(super) forge_context: Option<ForgeReviewWorkspace>,
    pub(super) forge_token: Option<String>,
    pub(super) forge_epoch: i32,
    pub(super) forge_current_epoch: Arc<AtomicI32>,
    pub(super) forge_results: Arc<Mutex<HashMap<i32, ForgeAsyncResult>>>,
    pub(super) forge_device_start_results: Arc<Mutex<HashMap<i32, GitHubDeviceStartResult>>>,
}

impl Default for Backend {
    fn default() -> Self {
        let git_root = initial_git_root();
        Self {
            active_workspace: Workspace::Diff.as_str().to_owned(),
            ready: false,
            status_message: "Connecting Rust application services…".to_owned(),
            bootstrap_started: false,
            git_files: GitFileListModel::default_with_attached_qobject(),
            git_branches: GitBranchListModel::default_with_attached_qobject(),
            git_commits: GitCommitListModel::default_with_attached_qobject(),
            git_root: git_root.display().to_string(),
            git_repository_name: "Repository".to_owned(),
            git_branch_name: String::new(),
            git_branch_has_upstream: false,
            git_branch_ahead_count: 0,
            git_branch_behind_count: 0,
            git_changed_file_count: 0,
            git_staged_file_count: 0,
            git_unstaged_file_count: 0,
            git_last_commit_subject: String::new(),
            git_ready: false,
            git_loading: false,
            git_busy: false,
            git_error: String::new(),
            git_status_message: String::new(),
            git_action_label: String::new(),
            git_epoch: 0,
            git_refresh_results: Arc::new(Mutex::new(HashMap::new())),
            git_root_pending_persist: false,
            git_staged_paths: Vec::new(),
            git_unstaged_paths: Vec::new(),
            forge_available: false,
            forge_provider_label: String::new(),
            forge_review_kind_label: String::new(),
            forge_host: String::new(),
            forge_repository_path: String::new(),
            forge_authenticated: false,
            forge_account_label: String::new(),
            forge_auth_mode: String::new(),
            forge_ready: false,
            forge_loading: false,
            forge_busy: false,
            forge_error: String::new(),
            forge_status_message: String::new(),
            forge_action_label: String::new(),
            forge_default_target_branch: String::new(),
            forge_review_exists: false,
            forge_review_number: 0,
            forge_review_title: String::new(),
            forge_review_url: String::new(),
            forge_review_state: String::new(),
            forge_review_draft: false,
            forge_device_flow_active: false,
            forge_device_user_code: String::new(),
            forge_device_verification_url: String::new(),
            forge_context: None,
            forge_token: None,
            forge_epoch: 0,
            forge_current_epoch: Arc::new(AtomicI32::new(0)),
            forge_results: Arc::new(Mutex::new(HashMap::new())),
            forge_device_start_results: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

fn initial_git_root() -> PathBuf {
    AppStateStore::new()
        .and_then(|store| store.load_or_default())
        .ok()
        .and_then(|state| state.active_project_path().cloned())
        .filter(|path| path.is_dir())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}
