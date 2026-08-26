use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, invoke_method, qobject};

use crate::backend_state::Backend;
use crate::local_path_from_qml_file_url;

pub const AI_PROMPT_MAX_ATTACHMENTS: usize = 16;
const AI_ATTACHMENT_MAX_CANDIDATES: usize = 64;
const AI_ATTACHMENT_SELECTION_MAX_BYTES: usize = 256 * 1024;
const AI_ATTACHMENT_MAX_PENDING_VALIDATIONS: usize = 4;
const AI_ATTACHMENT_PATH_MAX_BYTES: usize = 32 * 1024;
const AI_ATTACHMENT_NAME_MAX_BYTES: usize = 256;
const AI_ATTACHMENT_DRAFT_MAX_RETAINED_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Default, Eq, PartialEq, QModelItem)]
pub struct AiAttachmentItem {
    pub path: String,
    pub display_name: String,
}

impl AiAttachmentItem {
    fn from_path(path: PathBuf) -> Option<Self> {
        let path = path.to_str()?.to_owned();
        if path.len() > AI_ATTACHMENT_PATH_MAX_BYTES {
            return None;
        }
        let display_name = attachment_display_name(Path::new(path.as_str()));
        Some(Self { path, display_name })
    }
}

#[qobject(Base = QListModel)]
mod attachment_model {
    use qtbridge::QObjectHolder;

    use super::{AiAttachmentItem, QListModel, QListModelBase};

    #[derive(Default)]
    pub struct AiAttachmentListModel {
        items: Vec<AiAttachmentItem>,
        replacement: Option<Vec<AiAttachmentItem>>,
        deferred_replacement: Option<Vec<AiAttachmentItem>>,
        deferred_update_scheduled: bool,
    }

    impl AiAttachmentListModel {
        pub fn replace(&mut self, items: Vec<AiAttachmentItem>) {
            if self.items == items {
                return;
            }
            self.replacement = Some(items);
            self.reset();
        }

        pub fn defer_replace(&mut self, items: Vec<AiAttachmentItem>) {
            let current = self.deferred_replacement.as_ref().unwrap_or(&self.items);
            if current == &items {
                return;
            }
            self.deferred_replacement = Some(items);
            if self.deferred_update_scheduled {
                return;
            }
            self.deferred_update_scheduled = true;
            if !self
                .get_qml_method_invoker()
                .invoke_method("apply_deferred_replacement")
            {
                self.deferred_update_scheduled = false;
            }
        }

        #[qslot]
        fn apply_deferred_replacement(&mut self) {
            self.deferred_update_scheduled = false;
            let Some(items) = self.deferred_replacement.take() else {
                return;
            };
            self.replace(items);
        }
    }

    impl QListModel for AiAttachmentListModel {
        type Item = AiAttachmentItem;

        fn len(&self) -> usize {
            self.items.len()
        }

        fn get(&self, index: usize) -> Option<&Self::Item> {
            self.items.get(index)
        }

        fn set_unnotified(&mut self, index: usize, value: Self::Item) -> bool {
            let Some(item) = self.items.get_mut(index) else {
                return false;
            };
            *item = value;
            true
        }

        fn reset_unnotified(&mut self) {
            self.items = self.replacement.take().unwrap_or_default();
        }
    }
}

pub use attachment_model::AiAttachmentListModel;

pub(super) struct AiAttachmentValidationResult {
    thread_id: String,
    items: Vec<AiAttachmentItem>,
    skipped: usize,
}

pub(super) type AiAttachmentValidationResults =
    Arc<Mutex<HashMap<i32, AiAttachmentValidationResult>>>;

#[derive(Default)]
pub(super) struct AiAttachmentTasks {
    tasks: Vec<JoinHandle<()>>,
}

impl AiAttachmentTasks {
    fn push(&mut self, task: JoinHandle<()>) {
        self.tasks.retain(|task| !task.is_finished());
        self.tasks.push(task);
    }

    fn discard_finished(&mut self) {
        self.tasks.retain(|task| !task.is_finished());
    }
}

impl Drop for AiAttachmentTasks {
    fn drop(&mut self) {
        for task in self.tasks.drain(..) {
            let _ = task.join();
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AiAttachmentAddOutcome {
    pub added: usize,
    pub skipped: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AiAttachmentDrafts {
    by_thread: BTreeMap<String, Vec<AiAttachmentItem>>,
}

impl AiAttachmentDrafts {
    pub fn add_paths<I>(&mut self, thread_id: &str, paths: I) -> AiAttachmentAddOutcome
    where
        I: IntoIterator<Item = PathBuf>,
    {
        let validation = validate_attachment_paths(paths);
        self.add_validated(thread_id, validation.items, validation.skipped)
    }

    fn add_validated(
        &mut self,
        thread_id: &str,
        items: Vec<AiAttachmentItem>,
        skipped: usize,
    ) -> AiAttachmentAddOutcome {
        let mut retained_bytes = self.retained_bytes();
        let attachments = self.by_thread.entry(thread_id.to_owned()).or_default();
        let mut outcome = AiAttachmentAddOutcome { added: 0, skipped };

        for item in items {
            if attachments.len() >= AI_PROMPT_MAX_ATTACHMENTS {
                outcome.skipped = outcome.skipped.saturating_add(1);
                continue;
            }
            if attachments
                .iter()
                .any(|existing| existing.path == item.path)
            {
                outcome.skipped = outcome.skipped.saturating_add(1);
                continue;
            }
            let item_bytes = attachment_retained_bytes(&item);
            if retained_bytes.saturating_add(item_bytes) > AI_ATTACHMENT_DRAFT_MAX_RETAINED_BYTES {
                outcome.skipped = outcome.skipped.saturating_add(1);
                continue;
            }
            attachments.push(item);
            retained_bytes = retained_bytes.saturating_add(item_bytes);
            outcome.added = outcome.added.saturating_add(1);
        }

        if attachments.is_empty() {
            self.by_thread.remove(thread_id);
        }
        outcome
    }

    pub fn remove(&mut self, thread_id: &str, index: usize) -> bool {
        let Some(attachments) = self.by_thread.get_mut(thread_id) else {
            return false;
        };
        if index >= attachments.len() {
            return false;
        }
        attachments.remove(index);
        if attachments.is_empty() {
            self.by_thread.remove(thread_id);
        }
        true
    }

    pub fn items(&self, thread_id: &str) -> Vec<AiAttachmentItem> {
        self.by_thread.get(thread_id).cloned().unwrap_or_default()
    }

    pub fn paths(&self, thread_id: &str) -> Vec<PathBuf> {
        self.by_thread
            .get(thread_id)
            .into_iter()
            .flatten()
            .map(|attachment| PathBuf::from(attachment.path.as_str()))
            .collect()
    }

    pub fn clear_thread(&mut self, thread_id: &str) -> bool {
        self.by_thread.remove(thread_id).is_some()
    }

    pub fn clear(&mut self) {
        self.by_thread.clear();
    }

    pub fn retain_threads<'a, I>(&mut self, thread_ids: I)
    where
        I: IntoIterator<Item = &'a str>,
    {
        let thread_ids = thread_ids
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        self.by_thread
            .retain(|thread_id, _| thread_ids.contains(thread_id.as_str()));
    }

    fn retained_bytes(&self) -> usize {
        self.by_thread
            .values()
            .flatten()
            .map(attachment_retained_bytes)
            .fold(0usize, usize::saturating_add)
    }
}

pub fn attachment_paths_from_qml_json(value: &str) -> Result<Vec<PathBuf>, String> {
    if value.len() > AI_ATTACHMENT_SELECTION_MAX_BYTES {
        return Err("The attachment selection is too large.".to_owned());
    }
    let values = serde_json::from_str::<Vec<String>>(value)
        .map_err(|_| "The attachment selection is invalid.".to_owned())?;
    if values.len() > AI_ATTACHMENT_MAX_CANDIDATES {
        return Err(format!(
            "Select at most {AI_ATTACHMENT_MAX_CANDIDATES} images at once."
        ));
    }
    values
        .into_iter()
        .map(|value| {
            if value.len() > AI_ATTACHMENT_PATH_MAX_BYTES {
                return Err("The attachment selection contains a path that is too long.".to_owned());
            }
            local_path_from_qml_file_url(value.as_str()).map_err(|_| {
                "The attachment selection contains an invalid local file URL.".to_owned()
            })
        })
        .collect()
}

pub(super) fn queue_ai_attachments(backend: &mut Backend, paths_json: String) -> bool {
    if !ai_attachments_editable(backend) {
        return reject_attachment_change(backend, "Attachments cannot be changed right now.");
    }
    if !backend
        .ai_session_catalog
        .model_supports_image_inputs(Some(backend.ai_selected_model.as_str()))
    {
        return reject_attachment_change(
            backend,
            "Selected model does not support image attachments.",
        );
    }
    if backend.ai_attachment_pending_threads.len() >= AI_ATTACHMENT_MAX_PENDING_VALIDATIONS {
        return reject_attachment_change(
            backend,
            "Wait for an image attachment validation to finish.",
        );
    }
    let paths = match attachment_paths_from_qml_json(paths_json.as_str()) {
        Ok(paths) => paths,
        Err(error) => return reject_attachment_change(backend, error.as_str()),
    };
    if paths.is_empty() {
        return reject_attachment_change(backend, "No images were selected.");
    }

    backend.ai_attachment_epoch = backend.ai_attachment_epoch.wrapping_add(1).max(1);
    let epoch = backend.ai_attachment_epoch;
    let thread_id = backend.ai_active_thread_id.clone();
    backend
        .ai_attachment_pending_threads
        .insert(epoch, thread_id.clone());
    backend
        .ai_attachment_validation_epochs
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(epoch);
    backend.ai_status_message = "Validating image attachments…".to_owned();

    let validation_epochs = Arc::clone(&backend.ai_attachment_validation_epochs);
    let results = Arc::clone(&backend.ai_attachment_results);
    let invoker = backend.get_qml_method_invoker();
    let spawn_result = std::thread::Builder::new()
        .name("hunk-desktop-ai-attachments".to_owned())
        .spawn(move || {
            let validation = validate_attachment_paths_until(paths, || {
                validation_epochs
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .contains(&epoch)
            });
            if !validation_epochs
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .contains(&epoch)
            {
                return;
            }
            results
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .insert(
                    epoch,
                    AiAttachmentValidationResult {
                        thread_id,
                        items: validation.items,
                        skipped: validation.skipped,
                    },
                );
            invoke_method!(invoker, "complete_ai_attachment_add", epoch);
        });
    match spawn_result {
        Ok(task) => backend.ai_attachment_tasks.push(task),
        Err(error) => {
            backend.ai_attachment_pending_threads.remove(&epoch);
            backend
                .ai_attachment_validation_epochs
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&epoch);
            backend.ai_status_message =
                format!("Failed to start image attachment validation: {error}");
            return false;
        }
    }
    true
}

pub(super) fn complete_ai_attachment_add(backend: &mut Backend, epoch: i32) {
    backend.ai_attachment_tasks.discard_finished();
    let was_pending = backend
        .ai_attachment_validation_epochs
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&epoch);
    let pending_thread_id = backend.ai_attachment_pending_threads.remove(&epoch);
    let result = backend
        .ai_attachment_results
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&epoch);
    if !was_pending {
        return;
    }
    let Some(result) = result else {
        backend.ai_status_message =
            "Image attachment validation completed without a queued result.".to_owned();
        return;
    };
    debug_assert_eq!(
        pending_thread_id.as_deref(),
        Some(result.thread_id.as_str())
    );
    if !backend
        .ai_threads
        .borrow()
        .contains_thread_id(result.thread_id.as_str())
    {
        backend.ai_status_message =
            "The attachment target thread is no longer available.".to_owned();
        return;
    }
    let outcome = backend.ai_attachment_drafts.add_validated(
        result.thread_id.as_str(),
        result.items,
        result.skipped,
    );
    if backend.ai_active_thread_id == result.thread_id {
        sync_ai_attachments(backend);
        backend.ai_status_message = attachment_status_message(outcome);
    }
}

pub(super) fn remove_ai_attachment(backend: &mut Backend, index: i32) -> bool {
    if !ai_attachments_editable(backend) {
        return false;
    }
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    let removed = backend
        .ai_attachment_drafts
        .remove(backend.ai_active_thread_id.as_str(), index);
    if removed {
        sync_ai_attachments(backend);
        backend.ai_status_message.clear();
    }
    removed
}

pub(super) fn current_ai_attachment_paths(backend: &Backend) -> Vec<PathBuf> {
    backend
        .ai_attachment_drafts
        .paths(backend.ai_active_thread_id.as_str())
}

pub(super) fn restore_ai_attachment_paths(
    backend: &mut Backend,
    thread_id: &str,
    paths: Vec<PathBuf>,
) {
    let mut skipped = 0usize;
    let items = paths
        .into_iter()
        .filter_map(|path| {
            let item = AiAttachmentItem::from_path(path);
            skipped = skipped.saturating_add(usize::from(item.is_none()));
            item
        })
        .collect();
    backend
        .ai_attachment_drafts
        .add_validated(thread_id, items, skipped);
    if backend.ai_active_thread_id == thread_id {
        sync_ai_attachments(backend);
    }
}

pub(super) fn clear_ai_attachments_for_thread(backend: &mut Backend, thread_id: &str) {
    if backend.ai_attachment_drafts.clear_thread(thread_id)
        && backend.ai_active_thread_id == thread_id
    {
        sync_ai_attachments(backend);
    }
}

pub(super) fn clear_ai_attachment_drafts(backend: &mut Backend) {
    backend
        .ai_attachment_validation_epochs
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clear();
    backend.ai_attachment_pending_threads.clear();
    backend
        .ai_attachment_results
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clear();
    backend.ai_attachment_drafts.clear();
    backend
        .ai_attachments
        .borrow_mut()
        .defer_replace(Vec::new());
}

pub(super) fn sync_ai_attachments(backend: &mut Backend) {
    let items = backend
        .ai_attachment_drafts
        .items(backend.ai_active_thread_id.as_str());
    backend.ai_attachments.borrow_mut().defer_replace(items);
}

pub(super) fn ai_attachment_pending(backend: &Backend) -> bool {
    backend
        .ai_attachment_pending_threads
        .values()
        .any(|thread_id| thread_id == &backend.ai_active_thread_id)
}

fn ai_attachments_editable(backend: &Backend) -> bool {
    backend.ai_ready
        && !backend.ai_loading
        && !backend.ai_requires_authentication
        && !backend.ai_active_thread_id.is_empty()
        && backend.ai_thread_action.is_none()
        && backend.ai_prompt_receipt.is_none()
        && backend.ai_interrupt_turn_id.is_empty()
        && backend.ai_requests.current.is_none()
        && backend.ai_request_resolving_id.is_empty()
        && !ai_attachment_pending(backend)
}

struct AiAttachmentValidation {
    items: Vec<AiAttachmentItem>,
    skipped: usize,
}

fn validate_attachment_paths<I>(paths: I) -> AiAttachmentValidation
where
    I: IntoIterator<Item = PathBuf>,
{
    validate_attachment_paths_until(paths, || true)
}

fn validate_attachment_paths_until<I, F>(paths: I, mut should_continue: F) -> AiAttachmentValidation
where
    I: IntoIterator<Item = PathBuf>,
    F: FnMut() -> bool,
{
    let mut items = Vec::new();
    let mut canonical_paths = BTreeSet::new();
    let mut skipped = 0usize;
    for path in paths {
        if !should_continue() {
            break;
        }
        let canonical = match std::fs::canonicalize(path.as_path()) {
            Ok(path) => path,
            Err(_) => {
                skipped = skipped.saturating_add(1);
                continue;
            }
        };
        if !canonical.is_file()
            || !hunk_app::ai::is_supported_ai_image_path(&canonical)
            || !canonical_paths.insert(canonical.clone())
        {
            skipped = skipped.saturating_add(1);
            continue;
        }
        let Some(item) = AiAttachmentItem::from_path(canonical) else {
            skipped = skipped.saturating_add(1);
            continue;
        };
        items.push(item);
    }
    AiAttachmentValidation { items, skipped }
}

fn attachment_status_message(outcome: AiAttachmentAddOutcome) -> String {
    match (outcome.added, outcome.skipped) {
        (0, 0) => "No images were selected.".to_owned(),
        (0, _) => "No supported new images were attached.".to_owned(),
        (1, 0) => "Attached 1 image.".to_owned(),
        (added, 0) => format!("Attached {added} images."),
        (1, skipped) => format!("Attached 1 image and skipped {skipped}."),
        (added, skipped) => format!("Attached {added} images and skipped {skipped}."),
    }
}

fn reject_attachment_change(backend: &mut Backend, message: &str) -> bool {
    backend.ai_status_message = message.to_owned();
    false
}

fn attachment_display_name(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    bounded_text(name.as_str(), AI_ATTACHMENT_NAME_MAX_BYTES)
}

fn attachment_retained_bytes(attachment: &AiAttachmentItem) -> usize {
    attachment
        .path
        .len()
        .saturating_add(attachment.display_name.len())
        .saturating_add(2)
}

fn bounded_text(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes.saturating_sub(3).min(value.len());
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", value[..end].trim_end())
}
