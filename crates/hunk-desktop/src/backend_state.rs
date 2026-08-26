use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::AtomicI32;
use std::sync::{Arc, Mutex, OnceLock};

use hunk_app::diff::DiffCommentAnchor;
use hunk_domain::state::{
    AiCollaborationModeSelection, AiServiceTierSelection, AppStateStore,
    ReviewCompareSelectionState,
};
use qtbridge::QObjectHolder;

use crate::ai_attachments::{
    AiAttachmentDrafts, AiAttachmentListModel, AiAttachmentTasks, AiAttachmentValidationResults,
};
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
use crate::browser::BrowserBridge;
use crate::comment_models::{DiffCommentListModel, DiffCommentProjection};
use crate::compare_models::{DiffCompareSnapshotPayload, DiffCompareSourceListModel};
use crate::diff_models::{DiffFileSummary, DiffRowListModel, DiffSnapshotPayload};
use crate::git_models::GitSnapshotPayload;
use crate::terminal::TerminalRuntimeState;
use crate::terminal_models::{TerminalRowListModel, TerminalTabListModel};
use crate::updater::UpdateBridge;
use crate::{AiMessageQueue, AiPromptReceipt, GitFileListModel};

pub(super) type GitRefreshResult = Result<GitSnapshotPayload, String>;
pub(super) type DiffRefreshResult = Result<DiffSnapshotPayload, String>;
pub(super) type DiffCompareRefreshResult = Result<DiffCompareSnapshotPayload, String>;
pub(super) type DiffCommentAsyncResult = Result<DiffCommentAsyncPayload, String>;
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
    Ai,
}

impl Workspace {
    pub const ALL: [Self; 2] = [Self::Diff, Self::Ai];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Diff => "diff",
            Self::Ai => "ai",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "diff" => Some(Self::Diff),
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
    pub(super) diff_compare_sources: Rc<RefCell<DiffCompareSourceListModel>>,
    pub(super) diff_compare_left_source_id: String,
    pub(super) diff_compare_right_source_id: String,
    pub(super) diff_compare_left_label: String,
    pub(super) diff_compare_right_label: String,
    pub(super) diff_compare_left_index: i32,
    pub(super) diff_compare_right_index: i32,
    pub(super) diff_compare_file_count: i32,
    pub(super) diff_compare_epoch: i32,
    pub(super) diff_compare_results: Arc<Mutex<HashMap<i32, DiffCompareRefreshResult>>>,
    pub(super) diff_compare_patches: HashMap<String, String>,
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
    pub(super) git_root: String,
    pub(super) git_repository_name: String,
    pub(super) git_branch_name: String,
    pub(super) git_changed_file_count: i32,
    pub(super) git_ready: bool,
    pub(super) git_loading: bool,
    pub(super) git_error: String,
    pub(super) git_epoch: i32,
    pub(super) git_refresh_results: Arc<Mutex<HashMap<i32, GitRefreshResult>>>,
    pub(super) git_root_pending_persist: bool,
    pub(super) terminal_tabs: Rc<RefCell<TerminalTabListModel>>,
    pub(super) terminal_rows: Rc<RefCell<TerminalRowListModel>>,
    pub(super) terminal_runtime: TerminalRuntimeState,
    pub(super) terminal_open: bool,
    pub(super) terminal_active_tab_id: i32,
    pub(super) terminal_active_tab_index: i32,
    pub(super) terminal_shell_label: String,
    pub(super) terminal_status: String,
    pub(super) terminal_status_message: String,
    pub(super) terminal_cwd: String,
    pub(super) terminal_display_offset: i32,
    pub(super) terminal_mouse_mode: bool,
    pub(super) terminal_cursor_row: i32,
    pub(super) terminal_cursor_column: i32,
    pub(super) terminal_cursor_shape: String,
    pub(super) terminal_cursor_visible: bool,
    pub(super) terminal_screen_revision: i32,
    pub(super) terminal_focus_revision: i32,
    pub(super) browser: Rc<RefCell<BrowserBridge>>,
    pub(super) updates: Rc<RefCell<UpdateBridge>>,
    pub(super) ai_threads: Rc<RefCell<AiThreadListModel>>,
    pub(super) ai_timeline: Rc<RefCell<AiTimelineListModel>>,
    pub(super) ai_attachments: Rc<RefCell<AiAttachmentListModel>>,
    pub(super) ai_attachment_drafts: AiAttachmentDrafts,
    pub(super) ai_attachment_epoch: i32,
    pub(super) ai_attachment_pending_threads: BTreeMap<i32, String>,
    pub(super) ai_attachment_validation_epochs: Arc<Mutex<BTreeSet<i32>>>,
    pub(super) ai_attachment_results: AiAttachmentValidationResults,
    pub(super) ai_attachment_tasks: AiAttachmentTasks,
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
            diff_compare_sources: DiffCompareSourceListModel::default_with_attached_qobject(),
            diff_compare_left_source_id: String::new(),
            diff_compare_right_source_id: String::new(),
            diff_compare_left_label: String::new(),
            diff_compare_right_label: String::new(),
            diff_compare_left_index: -1,
            diff_compare_right_index: -1,
            diff_compare_file_count: 0,
            diff_compare_epoch: 0,
            diff_compare_results: Arc::new(Mutex::new(HashMap::new())),
            diff_compare_patches: HashMap::new(),
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
            git_root: git_root.display().to_string(),
            git_repository_name: "Repository".to_owned(),
            git_branch_name: String::new(),
            git_changed_file_count: 0,
            git_ready: false,
            git_loading: false,
            git_error: String::new(),
            git_epoch: 0,
            git_refresh_results: Arc::new(Mutex::new(HashMap::new())),
            git_root_pending_persist: false,
            terminal_tabs: TerminalTabListModel::default_with_attached_qobject(),
            terminal_rows: TerminalRowListModel::default_with_attached_qobject(),
            terminal_runtime: TerminalRuntimeState::default(),
            terminal_open: false,
            terminal_active_tab_id: 1,
            terminal_active_tab_index: 0,
            terminal_shell_label: "shell".to_owned(),
            terminal_status: "idle".to_owned(),
            terminal_status_message: String::new(),
            terminal_cwd: String::new(),
            terminal_display_offset: 0,
            terminal_mouse_mode: false,
            terminal_cursor_row: -1,
            terminal_cursor_column: -1,
            terminal_cursor_shape: "hidden".to_owned(),
            terminal_cursor_visible: false,
            terminal_screen_revision: 0,
            terminal_focus_revision: 0,
            browser: BrowserBridge::default_with_attached_qobject(),
            updates: UpdateBridge::default_with_attached_qobject(),
            ai_threads: AiThreadListModel::default_with_attached_qobject(),
            ai_timeline: AiTimelineListModel::default_with_attached_qobject(),
            ai_attachments: AiAttachmentListModel::default_with_attached_qobject(),
            ai_attachment_drafts: AiAttachmentDrafts::default(),
            ai_attachment_epoch: 0,
            ai_attachment_pending_threads: BTreeMap::new(),
            ai_attachment_validation_epochs: Arc::new(Mutex::new(BTreeSet::new())),
            ai_attachment_results: Arc::new(Mutex::new(HashMap::new())),
            ai_attachment_tasks: AiAttachmentTasks::default(),
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

pub(super) fn load_review_compare_selection(
    repo_root: &str,
) -> Option<ReviewCompareSelectionState> {
    AppStateStore::new()
        .and_then(|store| store.load_or_default())
        .ok()
        .and_then(|state| {
            state
                .review_compare_selection_by_repo
                .get(repo_root)
                .cloned()
        })
}

pub(super) fn persist_review_compare_selection(
    repo_root: &str,
    left_source_id: &str,
    right_source_id: &str,
) -> anyhow::Result<()> {
    let _guard = app_state_write_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let store = AppStateStore::new()?;
    let mut state = store.load_or_default()?;
    state.review_compare_selection_by_repo.insert(
        repo_root.to_owned(),
        ReviewCompareSelectionState {
            left_source_id: Some(left_source_id.to_owned()),
            right_source_id: Some(right_source_id.to_owned()),
        },
    );
    store.save(&state)
}

pub(super) fn app_state_write_lock() -> &'static Mutex<()> {
    static APP_STATE_WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    APP_STATE_WRITE_LOCK.get_or_init(|| Mutex::new(()))
}
