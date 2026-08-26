use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::thread::JoinHandle;

use hunk_app::ai::{AiTurnSessionOverrides, AiWorkerCommand};
use hunk_codex::protocol::Model;
use hunk_codex::state::{AiState, ThreadTokenUsageSummary};
use hunk_domain::state::{
    AiCollaborationModeSelection, AiServiceTierSelection, AiThreadSessionState, AppState,
    AppStateStore,
};
use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, invoke_method, qobject};

use crate::backend_ai::send_ai_worker_command;
use crate::backend_state::{Backend, app_state_write_lock};

const AI_MODEL_OPTION_MAX_ITEMS: usize = 128;
const AI_EFFORT_OPTION_MAX_ITEMS: usize = 16;
const AI_OPTION_LABEL_MAX_BYTES: usize = 128;
const AI_OPTION_DESCRIPTION_MAX_BYTES: usize = 512;
const AI_CONTEXT_WINDOW_BASELINE_TOKENS: i64 = 12_000;

#[derive(Clone, Debug, Default, Eq, PartialEq, QModelItem)]
pub struct AiSessionChoiceItem {
    pub value: String,
    pub label: String,
    pub description: String,
    pub hidden: bool,
    pub is_default: bool,
}

impl AiSessionChoiceItem {
    fn new(
        value: impl Into<String>,
        label: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            description: description.into(),
            hidden: false,
            is_default: false,
        }
    }
}

#[qobject(Base = QListModel)]
mod choice_model {
    use qtbridge::QObjectHolder;

    use super::{AiSessionChoiceItem, QListModel, QListModelBase};

    #[derive(Default)]
    pub struct AiSessionChoiceListModel {
        items: Vec<AiSessionChoiceItem>,
        replacement: Option<Vec<AiSessionChoiceItem>>,
        deferred_replacement: Option<Vec<AiSessionChoiceItem>>,
        deferred_update_scheduled: bool,
    }

    impl AiSessionChoiceListModel {
        pub fn replace(&mut self, items: Vec<AiSessionChoiceItem>) {
            if self.items == items {
                return;
            }
            self.replacement = Some(items);
            self.reset();
        }

        pub fn defer_replace(&mut self, items: Vec<AiSessionChoiceItem>) {
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

        fn visible_items(&self) -> &[AiSessionChoiceItem] {
            self.deferred_replacement
                .as_deref()
                .unwrap_or(self.items.as_slice())
        }

        pub fn value_at(&self, index: i32) -> Option<&str> {
            usize::try_from(index)
                .ok()
                .and_then(|index| self.visible_items().get(index))
                .map(|item| item.value.as_str())
        }

        pub fn index_of(&self, value: &str) -> i32 {
            self.visible_items()
                .iter()
                .position(|item| item.value == value)
                .and_then(|index| i32::try_from(index).ok())
                .unwrap_or(0)
        }

        pub fn label_for_value(&self, value: &str) -> String {
            self.visible_items()
                .iter()
                .find(|item| item.value == value)
                .map(|item| item.label.clone())
                .or_else(|| self.visible_items().first().map(|item| item.label.clone()))
                .unwrap_or_default()
        }

        pub fn contains_value(&self, value: &str) -> bool {
            self.visible_items().iter().any(|item| item.value == value)
        }

        pub fn item_count(&self) -> i32 {
            i32::try_from(self.visible_items().len()).unwrap_or(i32::MAX)
        }
    }

    impl QListModel for AiSessionChoiceListModel {
        type Item = AiSessionChoiceItem;

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

pub use choice_model::AiSessionChoiceListModel;
pub(super) type AiSessionChoiceModelHandle = Rc<RefCell<AiSessionChoiceListModel>>;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AiContextUsageProjection {
    pub available: bool,
    pub percent_used: i32,
    pub percent_left: i32,
    pub context_tokens: i64,
    pub context_window_tokens: i64,
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_tokens: i64,
    pub billable_tokens: i64,
}

impl AiContextUsageProjection {
    pub fn from_usage(usage: Option<&ThreadTokenUsageSummary>) -> Self {
        let Some(usage) = usage else {
            return Self::default();
        };
        let Some(window) = usage.model_context_window.filter(|window| *window > 0) else {
            return Self::default();
        };
        let percent_left = context_percent_left(usage, window);
        let cached_input_tokens = usage.last.cached_input_tokens.max(0);
        let input_tokens = (usage.last.input_tokens.max(0) - cached_input_tokens).max(0);
        let output_tokens = usage.last.output_tokens.max(0);

        Self {
            available: true,
            percent_used: i32::from(100u16.saturating_sub(percent_left)),
            percent_left: i32::from(percent_left),
            context_tokens: usage.last.total_tokens.max(0),
            context_window_tokens: window,
            input_tokens,
            cached_input_tokens,
            output_tokens,
            reasoning_tokens: usage.last.reasoning_output_tokens.max(0),
            billable_tokens: input_tokens.saturating_add(output_tokens),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AiSessionCatalogProjection {
    pub models: Vec<AiSessionChoiceItem>,
    pub efforts_by_model: BTreeMap<String, Vec<AiSessionChoiceItem>>,
    pub context_usage: AiContextUsageProjection,
    pub mad_max_mode: bool,
    pub include_hidden_models: bool,
    pub image_capable_model_ids: BTreeSet<String>,
    pub default_model_supports_image_inputs: bool,
}

impl AiSessionCatalogProjection {
    pub fn from_snapshot(
        state: &AiState,
        active_thread_id: Option<&str>,
        models: &[Model],
        mad_max_mode: bool,
        include_hidden_models: bool,
    ) -> Self {
        let mut model_choices = vec![default_model_choice()];
        let mut efforts_by_model = BTreeMap::new();
        let mut seen_model_ids = BTreeSet::new();
        let mut image_capable_model_ids = BTreeSet::new();
        let default_model_supports_image_inputs = models
            .iter()
            .find(|model| model.is_default)
            .or_else(|| models.first())
            .is_none_or(|model| {
                model
                    .input_modalities
                    .contains(&hunk_codex::protocol::InputModality::Image)
            });

        for model in models.iter().take(AI_MODEL_OPTION_MAX_ITEMS) {
            if model.id.trim().is_empty() || !seen_model_ids.insert(model.id.clone()) {
                continue;
            }
            let display_name = model.display_name.trim();
            let label = if display_name.is_empty() {
                model.id.as_str()
            } else {
                display_name
            };
            let mut choice = AiSessionChoiceItem::new(
                model.id.clone(),
                bounded_text(label, AI_OPTION_LABEL_MAX_BYTES),
                bounded_text(model.description.trim(), AI_OPTION_DESCRIPTION_MAX_BYTES),
            );
            choice.hidden = model.hidden;
            choice.is_default = model.is_default;
            model_choices.push(choice);
            if model
                .input_modalities
                .contains(&hunk_codex::protocol::InputModality::Image)
            {
                image_capable_model_ids.insert(model.id.clone());
            }

            let mut effort_choices = vec![default_effort_choice()];
            for option in model
                .supported_reasoning_efforts
                .iter()
                .take(AI_EFFORT_OPTION_MAX_ITEMS)
            {
                let value = option.reasoning_effort.as_str();
                if effort_choices.iter().any(|choice| choice.value == value) {
                    continue;
                }
                let mut choice = AiSessionChoiceItem::new(
                    value,
                    effort_label(value),
                    bounded_text(option.description.trim(), AI_OPTION_DESCRIPTION_MAX_BYTES),
                );
                choice.is_default = option.reasoning_effort == model.default_reasoning_effort;
                effort_choices.push(choice);
            }
            efforts_by_model.insert(model.id.clone(), effort_choices);
        }

        let usage = active_thread_id
            .filter(|thread_id| !thread_id.is_empty())
            .and_then(|thread_id| state.thread_token_usage.get(thread_id));
        Self {
            models: model_choices,
            efforts_by_model,
            context_usage: AiContextUsageProjection::from_usage(usage),
            mad_max_mode,
            include_hidden_models,
            image_capable_model_ids,
            default_model_supports_image_inputs,
        }
    }

    pub fn effort_choices(&self, model: Option<&str>) -> Vec<AiSessionChoiceItem> {
        model
            .filter(|model| !model.is_empty())
            .and_then(|model| self.efforts_by_model.get(model).cloned())
            .unwrap_or_else(|| vec![default_effort_choice()])
    }

    pub fn normalized_session(&self, mut session: AiThreadSessionState) -> AiThreadSessionState {
        let model_available = session
            .model
            .as_ref()
            .is_some_and(|selected| self.models.iter().any(|model| model.value == *selected));
        if !model_available {
            session.model = None;
            session.effort = None;
            return session;
        }
        let effort_available = session
            .model
            .as_ref()
            .and_then(|model| self.efforts_by_model.get(model))
            .is_some_and(|efforts| {
                session
                    .effort
                    .as_ref()
                    .is_some_and(|selected| efforts.iter().any(|effort| effort.value == *selected))
            });
        if !effort_available {
            session.effort = None;
        }
        session
    }

    pub fn model_supports_image_inputs(&self, selected_model: Option<&str>) -> bool {
        selected_model
            .filter(|model| !model.is_empty())
            .map(|model| self.image_capable_model_ids.contains(model))
            .unwrap_or(self.default_model_supports_image_inputs)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AiSessionPreferences {
    workspace_mad_max: BTreeMap<String, bool>,
    workspace_include_hidden_models: BTreeMap<String, bool>,
    workspace_sessions: BTreeMap<String, AiThreadSessionState>,
    thread_sessions: BTreeMap<String, AiThreadSessionState>,
}

impl AiSessionPreferences {
    pub fn from_state(state: &AppState) -> Self {
        Self {
            workspace_mad_max: state.ai_workspace_mad_max.clone(),
            workspace_include_hidden_models: state.ai_workspace_include_hidden_models.clone(),
            workspace_sessions: state.ai_workspace_session_overrides.clone(),
            thread_sessions: state.ai_thread_session_overrides.clone(),
        }
    }

    pub fn workspace_mad_max(&self, workspace: &str) -> bool {
        self.workspace_mad_max
            .get(workspace)
            .copied()
            .unwrap_or(true)
    }

    pub fn workspace_include_hidden_models(&self, workspace: &str) -> bool {
        self.workspace_include_hidden_models
            .get(workspace)
            .copied()
            .unwrap_or(true)
    }

    pub fn resolved_session(
        &self,
        thread_id: Option<&str>,
        workspace: Option<&str>,
    ) -> AiThreadSessionState {
        thread_id
            .and_then(|thread_id| self.thread_sessions.get(thread_id).cloned())
            .or_else(|| {
                workspace.and_then(|workspace| self.workspace_sessions.get(workspace).cloned())
            })
            .unwrap_or_else(AiThreadSessionState::preferred_defaults)
    }

    pub fn set_session(
        &mut self,
        thread_id: Option<&str>,
        workspace: Option<&str>,
        session: AiThreadSessionState,
    ) {
        if let Some(thread_id) = thread_id {
            set_session_entry(&mut self.thread_sessions, thread_id, session);
        } else if let Some(workspace) = workspace {
            set_session_entry(&mut self.workspace_sessions, workspace, session);
        }
    }

    pub fn set_workspace_mad_max(&mut self, workspace: &str, enabled: bool) {
        if enabled {
            self.workspace_mad_max.remove(workspace);
        } else {
            self.workspace_mad_max.insert(workspace.to_owned(), false);
        }
    }

    pub fn apply_to_state(&self, state: &mut AppState) {
        state.ai_workspace_mad_max = self.workspace_mad_max.clone();
        state.ai_workspace_include_hidden_models = self.workspace_include_hidden_models.clone();
        state.ai_workspace_session_overrides = self.workspace_sessions.clone();
        state.ai_thread_session_overrides = self.thread_sessions.clone();
    }
}

pub(super) struct AiSessionPersistResult {
    previous_preferences: AiSessionPreferences,
    result: Result<(), String>,
}

#[derive(Default)]
pub(super) struct AiSessionTasks {
    tasks: Vec<JoinHandle<()>>,
}

impl AiSessionTasks {
    fn push(&mut self, task: JoinHandle<()>) {
        self.tasks.retain(|task| !task.is_finished());
        self.tasks.push(task);
    }

    fn discard_finished(&mut self) {
        self.tasks.retain(|task| !task.is_finished());
    }
}

impl Drop for AiSessionTasks {
    fn drop(&mut self) {
        for task in self.tasks.drain(..) {
            let _ = task.join();
        }
    }
}

pub(super) fn apply_ai_session_projection(
    backend: &mut Backend,
    projection: AiSessionCatalogProjection,
) {
    backend
        .ai_models
        .borrow_mut()
        .defer_replace(projection.models.clone());
    backend.ai_session_catalog = projection;
    sync_ai_session_selection(backend);
}

pub(super) fn reset_ai_session_projection(backend: &mut Backend) {
    backend.ai_session_catalog = AiSessionCatalogProjection::default();
    backend
        .ai_models
        .borrow_mut()
        .defer_replace(vec![default_model_choice()]);
    backend
        .ai_efforts
        .borrow_mut()
        .defer_replace(vec![default_effort_choice()]);
    sync_ai_session_selection(backend);
}

pub(super) fn ai_turn_session_overrides(
    backend: &Backend,
    thread_id: Option<&str>,
) -> AiTurnSessionOverrides {
    let workspace = current_workspace(backend);
    let session = backend.ai_session_catalog.normalized_session(
        backend
            .ai_session_preferences
            .resolved_session(thread_id, workspace),
    );
    AiTurnSessionOverrides {
        model: session.model,
        effort: session.effort,
        collaboration_mode: session.collaboration_mode,
        service_tier: session.service_tier.unwrap_or_default(),
    }
}

pub(super) fn queue_ai_select_model(backend: &mut Backend, index: i32) -> bool {
    if ai_session_controls_locked(backend) {
        return false;
    }
    let Some(model) = backend
        .ai_models
        .borrow()
        .value_at(index)
        .map(str::to_owned)
    else {
        return false;
    };
    let previous_preferences = backend.ai_session_preferences.clone();
    backend.ai_selected_model = model;
    normalize_selected_effort(backend);
    record_current_session(backend);
    queue_ai_session_persist(backend, previous_preferences, "Updated the Codex model.")
}

pub(super) fn queue_ai_select_effort(backend: &mut Backend, index: i32) -> bool {
    if ai_session_controls_locked(backend) {
        return false;
    }
    let Some(effort) = backend
        .ai_efforts
        .borrow()
        .value_at(index)
        .map(str::to_owned)
    else {
        return false;
    };
    let previous_preferences = backend.ai_session_preferences.clone();
    backend.ai_selected_effort = effort;
    normalize_selected_effort(backend);
    record_current_session(backend);
    queue_ai_session_persist(backend, previous_preferences, "Updated reasoning effort.")
}

pub(super) fn queue_ai_select_collaboration_mode(backend: &mut Backend, value: String) -> bool {
    if ai_session_controls_locked(backend) {
        return false;
    }
    let Some(selection) = parse_collaboration_mode(value.as_str()) else {
        return false;
    };
    let previous_preferences = backend.ai_session_preferences.clone();
    backend.ai_selected_collaboration_mode = collaboration_mode_value(selection).to_owned();
    record_current_session(backend);
    queue_ai_session_persist(backend, previous_preferences, "Updated the Codex mode.")
}

pub(super) fn queue_ai_select_service_tier(backend: &mut Backend, index: i32) -> bool {
    if ai_session_controls_locked(backend) {
        return false;
    }
    let Some(service_tier) = backend
        .ai_service_tiers
        .borrow()
        .value_at(index)
        .map(str::to_owned)
    else {
        return false;
    };
    if parse_service_tier(service_tier.as_str()).is_none() {
        return false;
    }
    let previous_preferences = backend.ai_session_preferences.clone();
    backend.ai_selected_service_tier = service_tier;
    record_current_session(backend);
    queue_ai_session_persist(backend, previous_preferences, "Updated the service tier.")
}

pub(super) fn queue_ai_set_mad_max_mode(backend: &mut Backend, enabled: bool) -> bool {
    if ai_session_controls_locked(backend) || backend.ai_mad_max_mode == enabled {
        return false;
    }
    let Some(workspace) = current_workspace(backend).map(str::to_owned) else {
        return false;
    };
    if !send_ai_worker_command(
        backend,
        AiWorkerCommand::SetMadMaxMode { enabled },
        "Updating the Codex approval policy…",
    ) {
        return false;
    }
    let previous_preferences = backend.ai_session_preferences.clone();
    backend
        .ai_session_preferences
        .set_workspace_mad_max(workspace.as_str(), enabled);
    backend.ai_mad_max_mode = enabled;
    queue_ai_session_persist(
        backend,
        previous_preferences,
        if enabled {
            "Approval policy set to Full access."
        } else {
            "Approval policy set to Ask for approvals."
        },
    )
}

pub(super) fn complete_ai_session_persist(backend: &mut Backend, epoch: i32) {
    backend.ai_session_tasks.discard_finished();
    let result = backend
        .ai_session_results
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&epoch);
    let Some(result) = result else {
        return;
    };
    if backend.ai_session_current_epoch.load(Ordering::Acquire) != epoch {
        return;
    }
    if let Err(error) = result.result {
        backend.ai_session_preferences = result.previous_preferences;
        sync_ai_session_selection(backend);
        if backend.ai_runtime.session.is_some() {
            let enabled = backend.ai_mad_max_mode;
            let _ = send_ai_worker_command(
                backend,
                AiWorkerCommand::SetMadMaxMode { enabled },
                "Restoring the previous approval policy…",
            );
        }
        backend.ai_status_message = format!("Failed to save Codex session settings: {error}");
    }
}

pub(super) fn sync_ai_session_selection(backend: &mut Backend) {
    let workspace = current_workspace(backend).map(str::to_owned);
    let thread_id =
        (!backend.ai_active_thread_id.is_empty()).then_some(backend.ai_active_thread_id.as_str());
    let session = backend.ai_session_catalog.normalized_session(
        backend
            .ai_session_preferences
            .resolved_session(thread_id, workspace.as_deref()),
    );
    backend.ai_selected_model = session.model.unwrap_or_default();
    backend.ai_selected_effort = session.effort.unwrap_or_default();
    backend.ai_selected_collaboration_mode =
        collaboration_mode_value(session.collaboration_mode).to_owned();
    backend.ai_selected_service_tier =
        service_tier_value(session.service_tier.unwrap_or_default()).to_owned();
    backend.ai_mad_max_mode = workspace
        .as_deref()
        .is_some_and(|workspace| backend.ai_session_preferences.workspace_mad_max(workspace));
    replace_effort_choices(backend);
}

pub(super) fn ai_session_controls_locked(backend: &Backend) -> bool {
    !backend.ai_ready
        || backend.ai_loading
        || backend.ai_requires_authentication
        || backend.ai_turn_running
        || backend.ai_prompt_receipt.is_some()
        || backend.ai_thread_action.is_some()
        || !backend.ai_interrupt_turn_id.is_empty()
}

pub(super) fn service_tier_choices() -> Vec<AiSessionChoiceItem> {
    vec![
        AiSessionChoiceItem::new("standard", "Standard", "Use normal request routing."),
        AiSessionChoiceItem::new("fast", "Fast", "Prioritize lower-latency responses."),
        AiSessionChoiceItem::new("flex", "Flex", "Use flexible-capacity request routing."),
    ]
}

fn queue_ai_session_persist(
    backend: &mut Backend,
    previous_preferences: AiSessionPreferences,
    success_message: &str,
) -> bool {
    backend.ai_session_epoch = backend.ai_session_epoch.wrapping_add(1).max(1);
    let epoch = backend.ai_session_epoch;
    backend
        .ai_session_current_epoch
        .store(epoch, Ordering::Release);
    let current_epoch = backend.ai_session_current_epoch.clone();
    let preferences = backend.ai_session_preferences.clone();
    let rollback_preferences = previous_preferences.clone();
    let results = backend.ai_session_results.clone();
    let invoker = backend.get_qml_method_invoker();
    let spawn_result = std::thread::Builder::new()
        .name("hunk-desktop-ai-session".to_owned())
        .spawn(move || {
            let _writer_guard = app_state_write_lock()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if current_epoch.load(Ordering::Acquire) != epoch {
                return;
            }
            let result = match persist_preferences_locked(&preferences) {
                Ok(()) => Ok(()),
                Err(error) => match persist_preferences_locked(&rollback_preferences) {
                    Ok(()) => Err(error),
                    Err(recovery_error) => Err(format!(
                        "{error}; failed to restore saved session settings: {recovery_error}"
                    )),
                },
            };
            results
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .insert(
                    epoch,
                    AiSessionPersistResult {
                        previous_preferences: rollback_preferences,
                        result,
                    },
                );
            invoke_method!(invoker, "complete_ai_session_persist", epoch);
        });
    match spawn_result {
        Ok(task) => {
            backend.ai_session_tasks.push(task);
            backend.ai_status_message = success_message.to_owned();
            true
        }
        Err(error) => {
            backend.ai_session_preferences = previous_preferences;
            sync_ai_session_selection(backend);
            if backend.ai_runtime.session.is_some() {
                let enabled = backend.ai_mad_max_mode;
                let _ = send_ai_worker_command(
                    backend,
                    AiWorkerCommand::SetMadMaxMode { enabled },
                    "Restoring the previous approval policy…",
                );
            }
            let recovery_error = persist_preferences(&backend.ai_session_preferences).err();
            backend.ai_status_message = recovery_error.map_or_else(
                || format!("Failed to start session settings save: {error}"),
                |recovery_error| {
                    format!(
                        "Failed to start session settings save: {error}; failed to restore saved session settings: {recovery_error}"
                    )
                },
            );
            false
        }
    }
}

fn persist_preferences(preferences: &AiSessionPreferences) -> Result<(), String> {
    let _writer_guard = app_state_write_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    persist_preferences_locked(preferences)
}

fn persist_preferences_locked(preferences: &AiSessionPreferences) -> Result<(), String> {
    let store = AppStateStore::new().map_err(|error| format!("{error:#}"))?;
    let mut state = store
        .load_or_default()
        .map_err(|error| format!("{error:#}"))?;
    preferences.apply_to_state(&mut state);
    store.save(&state).map_err(|error| format!("{error:#}"))
}

fn record_current_session(backend: &mut Backend) {
    let thread_id =
        (!backend.ai_active_thread_id.is_empty()).then_some(backend.ai_active_thread_id.clone());
    let workspace = current_workspace(backend).map(str::to_owned);
    let session = AiThreadSessionState {
        model: (!backend.ai_selected_model.is_empty()).then(|| backend.ai_selected_model.clone()),
        effort: (!backend.ai_selected_effort.is_empty())
            .then(|| backend.ai_selected_effort.clone()),
        collaboration_mode: parse_collaboration_mode(
            backend.ai_selected_collaboration_mode.as_str(),
        )
        .unwrap_or_default(),
        service_tier: parse_service_tier(backend.ai_selected_service_tier.as_str())
            .and_then(normalized_service_tier),
    };
    backend
        .ai_session_preferences
        .set_session(thread_id.as_deref(), workspace.as_deref(), session);
}

fn normalize_selected_effort(backend: &mut Backend) {
    replace_effort_choices(backend);
    if !backend
        .ai_efforts
        .borrow()
        .contains_value(backend.ai_selected_effort.as_str())
    {
        backend.ai_selected_effort.clear();
    }
}

fn replace_effort_choices(backend: &mut Backend) {
    let selected_model =
        (!backend.ai_selected_model.is_empty()).then_some(backend.ai_selected_model.as_str());
    backend
        .ai_efforts
        .borrow_mut()
        .defer_replace(backend.ai_session_catalog.effort_choices(selected_model));
}

fn current_workspace(backend: &Backend) -> Option<&str> {
    if backend.ai_workspace_root.is_empty() {
        (!backend.git_root.is_empty()).then_some(backend.git_root.as_str())
    } else {
        Some(backend.ai_workspace_root.as_str())
    }
}

fn set_session_entry(
    sessions: &mut BTreeMap<String, AiThreadSessionState>,
    key: &str,
    session: AiThreadSessionState,
) {
    if let Some(session) = normalized_session_state(session) {
        sessions.insert(key.to_owned(), session);
    } else {
        sessions.remove(key);
    }
}

fn normalized_session_state(mut session: AiThreadSessionState) -> Option<AiThreadSessionState> {
    session.service_tier = session.service_tier.and_then(normalized_service_tier);
    let empty = session.model.is_none()
        && session.effort.is_none()
        && session.collaboration_mode == AiCollaborationModeSelection::Default
        && session.service_tier.is_none();
    (!empty).then_some(session)
}

fn normalized_service_tier(service_tier: AiServiceTierSelection) -> Option<AiServiceTierSelection> {
    match service_tier {
        AiServiceTierSelection::Standard => None,
        other => Some(other),
    }
}

fn parse_collaboration_mode(value: &str) -> Option<AiCollaborationModeSelection> {
    match value {
        "code" => Some(AiCollaborationModeSelection::Default),
        "plan" => Some(AiCollaborationModeSelection::Plan),
        _ => None,
    }
}

fn collaboration_mode_value(selection: AiCollaborationModeSelection) -> &'static str {
    match selection {
        AiCollaborationModeSelection::Default => "code",
        AiCollaborationModeSelection::Plan => "plan",
    }
}

fn parse_service_tier(value: &str) -> Option<AiServiceTierSelection> {
    match value {
        "standard" => Some(AiServiceTierSelection::Standard),
        "fast" => Some(AiServiceTierSelection::Fast),
        "flex" => Some(AiServiceTierSelection::Flex),
        _ => None,
    }
}

fn service_tier_value(selection: AiServiceTierSelection) -> &'static str {
    match selection {
        AiServiceTierSelection::Standard => "standard",
        AiServiceTierSelection::Fast => "fast",
        AiServiceTierSelection::Flex => "flex",
    }
}

fn default_model_choice() -> AiSessionChoiceItem {
    let mut choice =
        AiSessionChoiceItem::new("", "Server default", "Let Codex choose its default model.");
    choice.is_default = true;
    choice
}

fn default_effort_choice() -> AiSessionChoiceItem {
    let mut choice = AiSessionChoiceItem::new(
        "",
        "Model default",
        "Use the selected model's default reasoning effort.",
    );
    choice.is_default = true;
    choice
}

fn effort_label(value: &str) -> String {
    match value {
        "none" => "None".to_owned(),
        "minimal" => "Minimal".to_owned(),
        "low" => "Low".to_owned(),
        "medium" => "Medium".to_owned(),
        "high" => "High".to_owned(),
        "xhigh" | "extra_high" | "extra-high" => "Extra High".to_owned(),
        "max" => "Max".to_owned(),
        "ultra" => "Ultra".to_owned(),
        other => other
            .split(['_', '-'])
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_uppercase().chain(chars).collect()
                })
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn context_percent_left(usage: &ThreadTokenUsageSummary, window: i64) -> u16 {
    if window <= AI_CONTEXT_WINDOW_BASELINE_TOKENS {
        return 0;
    }
    let effective_window = window - AI_CONTEXT_WINDOW_BASELINE_TOKENS;
    let used = (usage.last.total_tokens.max(0) - AI_CONTEXT_WINDOW_BASELINE_TOKENS).max(0);
    let remaining = (effective_window - used).max(0);
    ((remaining as f64 / effective_window as f64) * 100.0)
        .clamp(0.0, 100.0)
        .round() as u16
}

fn bounded_text(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes.min(value.len());
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

pub(super) fn compact_token_count(token_count: i64) -> String {
    let token_count = token_count.max(0);
    if token_count >= 1_000_000 {
        format!("{}m", (token_count + 500_000) / 1_000_000)
    } else if token_count >= 1_000 {
        format!("{}k", (token_count + 500) / 1_000)
    } else {
        token_count.to_string()
    }
}

pub(super) fn exact_token_count(token_count: i64) -> String {
    let digits = token_count.max(0).to_string();
    let mut reversed = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            reversed.push(',');
        }
        reversed.push(ch);
    }
    reversed.chars().rev().collect()
}

pub(super) fn initial_choice_models() -> (
    AiSessionChoiceModelHandle,
    AiSessionChoiceModelHandle,
    AiSessionChoiceModelHandle,
) {
    let models = AiSessionChoiceListModel::default_with_attached_qobject();
    models.borrow_mut().replace(vec![default_model_choice()]);
    let efforts = AiSessionChoiceListModel::default_with_attached_qobject();
    efforts.borrow_mut().replace(vec![default_effort_choice()]);
    let service_tiers = AiSessionChoiceListModel::default_with_attached_qobject();
    service_tiers.borrow_mut().replace(service_tier_choices());
    (models, efforts, service_tiers)
}
