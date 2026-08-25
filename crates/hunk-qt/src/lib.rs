mod ai_attachments;
mod ai_bookmarks;
mod ai_composer;
mod ai_models;
mod ai_queue;
mod ai_requests;
mod ai_runtime;
mod ai_session;
mod ai_thread_actions;
mod ai_timeline_models;
mod backend;
mod backend_ai;
mod backend_ai_accessors;
mod backend_diff;
mod backend_forge;
mod backend_git;
mod backend_state;
mod comment_models;
mod comments;
mod diff_models;
mod forge;
mod git_models;
mod path;

#[cfg(debug_assertions)]
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use qtbridge::QApp;
#[cfg(not(debug_assertions))]
use qtbridge::include_bytes_qml;
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

pub use ai_composer::AiPromptReceipt;
pub use ai_models::{AiThreadCatalogProjection, AiThreadItem, AiThreadListModel};
pub use ai_queue::{
    AI_MESSAGE_QUEUE_MAX_ITEMS, AI_MESSAGE_QUEUE_MAX_PROMPT_BYTES,
    AI_MESSAGE_QUEUE_MAX_RETAINED_BYTES, AiMessageQueue, AiQueueProjection,
    AiQueueThreadProjection, AiQueuedMessageCommand, AiRecoveredDraft,
};
pub use ai_requests::{
    AiPendingOption, AiPendingQuestion, AiPendingRequest, AiPendingRequestProjection,
};
pub use ai_runtime::{AiEventMailbox, AiProjectedSnapshot, AiRuntimeEvent};
pub use ai_session::{
    AiContextUsageProjection, AiSessionCatalogProjection, AiSessionChoiceItem,
    AiSessionChoiceListModel, AiSessionPreferences,
};
pub use ai_thread_actions::{AiThreadActionKind, AiThreadActionReceipt};
pub use ai_timeline_models::{
    AI_TIMELINE_MAX_VISIBLE_ROWS, AiTimelineItem, AiTimelineListModel, AiTimelineProjection,
};
pub use backend::{Backend, Workspace};
pub use comment_models::{DiffCommentItem, DiffCommentListModel, DiffCommentProjection};
pub use diff_models::{DiffFileSummary, DiffRowListModel, DiffSnapshotPayload};
pub use git_models::{GitBranchListModel, GitCommitListModel, GitFileListModel};
pub use path::{local_path_from_qml_file_url, local_path_from_qml_folder_url};

#[cfg(debug_assertions)]
const QML_MODULE_DIRECTORY: &str = "Hunk";
#[cfg(debug_assertions)]
const QML_ENTRY_FILE: &str = "Main.qml";

pub fn run() -> Result<()> {
    initialize_logging()?;
    install_panic_hook();

    let mut app = QApp::new();
    app.application_name("Hunk")
        .register::<AiThreadListModel>()
        .register::<AiTimelineListModel>()
        .register::<AiAttachmentListModel>()
        .register::<AiSessionChoiceListModel>()
        .register::<DiffRowListModel>()
        .register::<DiffCommentListModel>()
        .register::<GitFileListModel>()
        .register::<GitBranchListModel>()
        .register::<GitCommitListModel>()
        .register::<Backend>();
    load_qml(&mut app)?;

    let exit_code = app.run();
    if exit_code != 0 {
        bail!("Qt event loop exited with status {exit_code}");
    }
    Ok(())
}

fn initialize_logging() -> Result<()> {
    let default_level = if cfg!(debug_assertions) {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };
    let filter = EnvFilter::builder()
        .with_default_directive(default_level.into())
        .from_env_lossy();

    if let Err(error) = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .without_time()
        .try_init()
    {
        bail!("failed to initialize Qt frontend logging: {error}");
    }
    Ok(())
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        tracing::error!(%panic_info, "Qt frontend panicked");
        previous(panic_info);
    }));
}

#[cfg(debug_assertions)]
fn load_qml(app: &mut QApp) -> Result<()> {
    let qml_root = std::env::var_os("HUNK_QML_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/qml"));
    let entry_path = qml_root.join(QML_MODULE_DIRECTORY).join(QML_ENTRY_FILE);
    if !entry_path.is_file() {
        bail!(
            "Qt frontend entry point is missing: {}",
            entry_path.display()
        );
    }

    let import_path = qml_root.to_string_lossy();
    let entry_url = file_url(&entry_path);
    app.add_import_path(&import_path)
        .load_qml_from_file(&entry_url);
    Ok(())
}

#[cfg(not(debug_assertions))]
fn load_qml(app: &mut QApp) -> Result<()> {
    include_bytes_qml!("qml");
    app.add_import_path("qrc:/qml")
        .load_qml_from_file("qrc:/qml/Hunk/Main.qml");
    Ok(())
}

#[cfg(debug_assertions)]
fn file_url(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    if normalized.starts_with('/') {
        format!("file://{normalized}")
    } else {
        format!("file:///{normalized}")
    }
}
pub use ai_attachments::{
    AI_PROMPT_MAX_ATTACHMENTS, AiAttachmentAddOutcome, AiAttachmentDrafts, AiAttachmentItem,
    AiAttachmentListModel, attachment_paths_from_qml_json,
};
