use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use hunk_app::diff::DiffCommentAnchor;
use hunk_domain::state::{AiCollaborationModeSelection, AiServiceTierSelection, AppStateStore};
use hunk_forge::{ForgeReviewOutcome, ForgeReviewWorkspace, GitHubDeviceAuthorization};
use qtbridge::QObjectHolder;

use crate::ai_bookmarks::{AiBookmarkPersistResult, AiBookmarkTasks};
use crate::ai_models::AiThreadListModel;
use crate::ai_requests::AiPendingRequestProjection;
use crate::ai_runtime::AiRuntimeSlot;
use crate::ai_session::{
    AiSessionCatalogProjection, AiSessionChoiceListModel, AiSessionPersistResult,
    AiSessionPreferences, AiSessionTasks, initial_choice_models,
};
use crate::ai_thread_actions::AiThreadActionReceipt;
use crate::ai_timeline_models::AiTimelineListModel;
use crate::comment_models::{DiffCommentListModel, DiffCommentProjection};
use crate::diff_models::{DiffFileSummary, DiffRowListModel, DiffSnapshotPayload};
use crate::forge::ForgeSnapshotPayload;
use crate::git_models::{
    GitBranchListModel, GitCommitListModel, GitFileListModel, GitSnapshotPayload,
};
use crate::{AiMessageQueue, AiPromptReceipt};

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
    pub(super) ai_timeline: Rc<RefCell<AiTimelineListModel>>,
    pub(super) ai_models: Rc<RefCell<AiSessionChoiceListModel>>,
    pub(super) ai_efforts: Rc<RefCell<AiSessionChoiceListModel>>,
    pub(super) ai_service_tiers: Rc<RefCell<AiSessionChoiceListModel>>,
    pub(super) ai_session_catalog: AiSessionCatalogProjection,
    pub(super) ai_session_preferences: AiSessionPreferences,
    pub(super) ai_selected_model: String,
    pub(super) ai_selected_effort: String,
    pub(super) ai_selected_collaboration_mode: String,
    pub(super) ai_selected_service_tier: String,
    pub(super) ai_mad_max_mode: bool,
    pub(super) ai_session_epoch: i32,
    pub(super) ai_session_current_epoch: Arc<AtomicI32>,
    pub(super) ai_session_results: Arc<Mutex<HashMap<i32, AiSessionPersistResult>>>,
    pub(super) ai_session_tasks: AiSessionTasks,
    pub(super) ai_message_queue: AiMessageQueue,
    pub(super) ai_bookmarked_thread_ids: BTreeSet<String>,
    pub(super) ai_bookmark_epoch: i32,
    pub(super) ai_bookmark_current_epoch: Arc<AtomicI32>,
    pub(super) ai_bookmark_results: Arc<Mutex<HashMap<i32, AiBookmarkPersistResult>>>,
    pub(super) ai_bookmark_tasks: AiBookmarkTasks,
    pub(super) ai_runtime: AiRuntimeSlot,
    pub(super) ai_epoch: i32,
    pub(super) ai_ready: bool,
    pub(super) ai_loading: bool,
    pub(super) ai_requires_authentication: bool,
    pub(super) ai_connection_state: String,
    pub(super) ai_workspace_root: String,
    pub(super) ai_active_thread_id: String,
    pub(super) ai_active_thread_title: String,
    pub(super) ai_active_thread_cwd: String,
    pub(super) ai_active_turn_id: String,
    pub(super) ai_turn_running: bool,
    pub(super) ai_prompt_receipt: Option<AiPromptReceipt>,
    pub(super) ai_thread_action: Option<AiThreadActionReceipt>,
    pub(super) ai_prompt_accepted_revision: i32,
    pub(super) ai_interrupt_thread_id: String,
    pub(super) ai_interrupt_turn_id: String,
    pub(super) ai_requests: AiPendingRequestProjection,
    pub(super) ai_request_resolving_id: String,
    pub(super) ai_thread_count: i32,
    pub(super) ai_running_thread_count: i32,
    pub(super) ai_timeline_total_turn_count: i32,
    pub(super) ai_timeline_visible_turn_count: i32,
    pub(super) ai_timeline_hidden_turn_count: i32,
    pub(super) ai_timeline_total_row_count: i32,
    pub(super) ai_timeline_hidden_row_count: i32,
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
        let initial = initial_qt_state();
        let git_root = initial.git_root;
        let ai_bookmarked_thread_ids = initial.ai_bookmarked_thread_ids;
        let ai_session_preferences = initial.ai_session_preferences;
        let workspace_key = git_root.to_string_lossy().to_string();
        let initial_session = ai_session_preferences.resolved_session(None, Some(&workspace_key));
        let ai_selected_collaboration_mode = match initial_session.collaboration_mode {
            AiCollaborationModeSelection::Default => "code",
            AiCollaborationModeSelection::Plan => "plan",
        }
        .to_owned();
        let ai_selected_service_tier = match initial_session.service_tier.unwrap_or_default() {
            AiServiceTierSelection::Standard => "standard",
            AiServiceTierSelection::Fast => "fast",
            AiServiceTierSelection::Flex => "flex",
        }
        .to_owned();
        let ai_mad_max_mode = ai_session_preferences.workspace_mad_max(&workspace_key);
        let (ai_models, ai_efforts, ai_service_tiers) = initial_choice_models();
        let ai_runtime = AiRuntimeSlot::default();
        ai_runtime
            .mailbox
            .set_bookmarked_thread_ids(ai_bookmarked_thread_ids.clone());
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
            ai_timeline: AiTimelineListModel::default_with_attached_qobject(),
            ai_models,
            ai_efforts,
            ai_service_tiers,
            ai_session_catalog: AiSessionCatalogProjection::default(),
            ai_session_preferences,
            ai_selected_model: initial_session.model.unwrap_or_default(),
            ai_selected_effort: initial_session.effort.unwrap_or_default(),
            ai_selected_collaboration_mode,
            ai_selected_service_tier,
            ai_mad_max_mode,
            ai_session_epoch: 0,
            ai_session_current_epoch: Arc::new(AtomicI32::new(0)),
            ai_session_results: Arc::new(Mutex::new(HashMap::new())),
            ai_session_tasks: AiSessionTasks::default(),
            ai_message_queue: AiMessageQueue::default(),
            ai_bookmarked_thread_ids,
            ai_bookmark_epoch: 0,
            ai_bookmark_current_epoch: Arc::new(AtomicI32::new(0)),
            ai_bookmark_results: Arc::new(Mutex::new(HashMap::new())),
            ai_bookmark_tasks: AiBookmarkTasks::default(),
            ai_runtime,
            ai_epoch: 0,
            ai_ready: false,
            ai_loading: false,
            ai_requires_authentication: false,
            ai_connection_state: "disconnected".to_owned(),
            ai_workspace_root: String::new(),
            ai_active_thread_id: String::new(),
            ai_active_thread_title: String::new(),
            ai_active_thread_cwd: String::new(),
            ai_active_turn_id: String::new(),
            ai_turn_running: false,
            ai_prompt_receipt: None,
            ai_thread_action: None,
            ai_prompt_accepted_revision: 0,
            ai_interrupt_thread_id: String::new(),
            ai_interrupt_turn_id: String::new(),
            ai_requests: AiPendingRequestProjection::default(),
            ai_request_resolving_id: String::new(),
            ai_thread_count: 0,
            ai_running_thread_count: 0,
            ai_timeline_total_turn_count: 0,
            ai_timeline_visible_turn_count: 0,
            ai_timeline_hidden_turn_count: 0,
            ai_timeline_total_row_count: 0,
            ai_timeline_hidden_row_count: 0,
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

struct InitialQtState {
    git_root: PathBuf,
    ai_bookmarked_thread_ids: BTreeSet<String>,
    ai_session_preferences: AiSessionPreferences,
}

fn initial_qt_state() -> InitialQtState {
    let state = AppStateStore::new()
        .and_then(|store| store.load_or_default())
        .unwrap_or_default();
    let git_root = state
        .active_project_path()
        .cloned()
        .filter(|path| path.is_dir())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    InitialQtState {
        git_root,
        ai_bookmarked_thread_ids: state.ai_bookmarked_thread_ids.clone(),
        ai_session_preferences: AiSessionPreferences::from_state(&state),
    }
}

pub(super) fn persist_active_project(root: PathBuf) -> anyhow::Result<()> {
    let _guard = app_state_write_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let store = AppStateStore::new()?;
    let mut state = store.load_or_default()?;
    state.activate_workspace_project(root);
    store.save(&state)
}

pub(super) fn app_state_write_lock() -> &'static Mutex<()> {
    static APP_STATE_WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    APP_STATE_WRITE_LOCK.get_or_init(|| Mutex::new(()))
}

pub(super) fn next_forge_epoch(backend: &mut Backend) -> i32 {
    backend.forge_epoch = backend.forge_epoch.wrapping_add(1).max(1);
    backend
        .forge_current_epoch
        .store(backend.forge_epoch, Ordering::Release);
    backend.forge_epoch
}
