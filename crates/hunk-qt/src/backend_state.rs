use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::AtomicI32;
use std::sync::{Arc, Mutex};

use hunk_app::ai::AiSnapshot;
use hunk_app::diff::DiffCommentAnchor;
use hunk_domain::state::AppStateStore;
use hunk_forge::{ForgeReviewOutcome, ForgeReviewWorkspace, GitHubDeviceAuthorization};
use qtbridge::QObjectHolder;

use crate::ai_models::AiThreadListModel;
use crate::ai_runtime::AiRuntimeSlot;
use crate::comment_models::{DiffCommentListModel, DiffCommentProjection};
use crate::diff_models::{DiffFileSummary, DiffRowListModel, DiffSnapshotPayload};
use crate::forge::ForgeSnapshotPayload;
use crate::git_models::{
    GitBranchListModel, GitCommitListModel, GitFileListModel, GitSnapshotPayload,
};

pub(super) type GitRefreshResult = Result<GitSnapshotPayload, String>;
pub(super) type DiffRefreshResult = Result<DiffSnapshotPayload, String>;
pub(super) type DiffCommentAsyncResult = Result<DiffCommentAsyncPayload, String>;
pub(super) type ForgeAsyncResult = Result<ForgeAsyncPayload, String>;
type GitHubDeviceStartResult = Result<GitHubDeviceAuthorization, String>;

pub(super) enum ForgeAsyncPayload {
    Snapshot(Box<ForgeSnapshotPayload>),
    Review(ForgeReviewOutcome),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DiffCommentRequestKind {
    Load,
    Mutation,
    Reconcile,
}

pub(super) struct DiffCommentAsyncPayload {
    pub(super) kind: DiffCommentRequestKind,
    pub(super) diff_epoch: i32,
    pub(super) projection: DiffCommentProjection,
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
    pub(super) diff_files: Rc<RefCell<GitFileListModel>>,
    pub(super) diff_rows: Rc<RefCell<DiffRowListModel>>,
    pub(super) diff_selected_path: String,
    pub(super) diff_status_tag: String,
    pub(super) diff_additions: i32,
    pub(super) diff_removals: i32,
    pub(super) diff_ready: bool,
    pub(super) diff_loading: bool,
    pub(super) diff_error: String,
    pub(super) diff_search_query: String,
    pub(super) diff_search_match_count: i32,
    pub(super) diff_search_match_index: i32,
    pub(super) diff_search_target_row: i32,
    pub(super) diff_search_matches: Vec<usize>,
    pub(super) diff_epoch: i32,
    pub(super) diff_file_summaries: HashMap<String, DiffFileSummary>,
    pub(super) diff_refresh_results: Arc<Mutex<HashMap<i32, DiffRefreshResult>>>,
    pub(super) diff_comments: Rc<RefCell<DiffCommentListModel>>,
    pub(super) diff_comment_projection: Option<DiffCommentProjection>,
    pub(super) diff_comment_anchors: Arc<Vec<Option<DiffCommentAnchor>>>,
    pub(super) diff_comments_ready: bool,
    pub(super) diff_comments_loading: bool,
    pub(super) diff_comments_busy: bool,
    pub(super) diff_comments_error: String,
    pub(super) diff_comments_status_message: String,
    pub(super) diff_comments_show_non_open: bool,
    pub(super) diff_comments_open_count: i32,
    pub(super) diff_comments_stale_count: i32,
    pub(super) diff_comments_resolved_count: i32,
    pub(super) diff_comments_version: i32,
    pub(super) diff_comment_target_row: i32,
    pub(super) diff_comment_target_revision: i32,
    pub(super) diff_comment_epoch: i32,
    pub(super) diff_comment_results: Arc<Mutex<HashMap<i32, DiffCommentAsyncResult>>>,
    pub(super) diff_comment_refresh_pending: bool,
    pub(super) diff_comment_initial_prune_done: bool,
    pub(super) diff_comment_miss_streaks: HashMap<String, u8>,
    pub(super) diff_comment_pending_jump_id: Option<String>,
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
    pub(super) ai_threads: Rc<RefCell<AiThreadListModel>>,
    pub(super) ai_snapshot: Option<AiSnapshot>,
    pub(super) ai_runtime: AiRuntimeSlot,
    pub(super) ai_epoch: i32,
    pub(super) ai_ready: bool,
    pub(super) ai_loading: bool,
    pub(super) ai_requires_authentication: bool,
    pub(super) ai_connection_state: String,
    pub(super) ai_workspace_root: String,
    pub(super) ai_active_thread_id: String,
    pub(super) ai_thread_count: i32,
    pub(super) ai_running_thread_count: i32,
    pub(super) ai_error: String,
    pub(super) ai_status_message: String,
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
            diff_files: GitFileListModel::default_with_attached_qobject(),
            diff_rows: DiffRowListModel::default_with_attached_qobject(),
            diff_selected_path: String::new(),
            diff_status_tag: String::new(),
            diff_additions: 0,
            diff_removals: 0,
            diff_ready: false,
            diff_loading: false,
            diff_error: String::new(),
            diff_search_query: String::new(),
            diff_search_match_count: 0,
            diff_search_match_index: -1,
            diff_search_target_row: -1,
            diff_search_matches: Vec::new(),
            diff_epoch: 0,
            diff_file_summaries: HashMap::new(),
            diff_refresh_results: Arc::new(Mutex::new(HashMap::new())),
            diff_comments: DiffCommentListModel::default_with_attached_qobject(),
            diff_comment_projection: None,
            diff_comment_anchors: Arc::new(Vec::new()),
            diff_comments_ready: false,
            diff_comments_loading: false,
            diff_comments_busy: false,
            diff_comments_error: String::new(),
            diff_comments_status_message: String::new(),
            diff_comments_show_non_open: false,
            diff_comments_open_count: 0,
            diff_comments_stale_count: 0,
            diff_comments_resolved_count: 0,
            diff_comments_version: 0,
            diff_comment_target_row: -1,
            diff_comment_target_revision: 0,
            diff_comment_epoch: 0,
            diff_comment_results: Arc::new(Mutex::new(HashMap::new())),
            diff_comment_refresh_pending: false,
            diff_comment_initial_prune_done: false,
            diff_comment_miss_streaks: HashMap::new(),
            diff_comment_pending_jump_id: None,
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
            ai_threads: AiThreadListModel::default_with_attached_qobject(),
            ai_snapshot: None,
            ai_runtime: AiRuntimeSlot::default(),
            ai_epoch: 0,
            ai_ready: false,
            ai_loading: false,
            ai_requires_authentication: false,
            ai_connection_state: "disconnected".to_owned(),
            ai_workspace_root: String::new(),
            ai_active_thread_id: String::new(),
            ai_thread_count: 0,
            ai_running_thread_count: 0,
            ai_error: String::new(),
            ai_status_message: String::new(),
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
