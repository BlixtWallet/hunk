use std::collections::BTreeSet;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use hunk_app::ai::{
    AiWorkerCommand, AiWorkerEvent, AiWorkerEventPayload, AiWorkerStartConfig,
    browser_unavailable_response, resolve_codex_executable_path, resolve_codex_home_path,
    spawn_ai_worker, validate_codex_executable_path,
};

use crate::ai_account::AiAccountProjection;
use crate::ai_markdown::AiMarkdownProjectionCache;
use crate::ai_models::AiThreadCatalogProjection;
use crate::ai_queue::AiQueueProjection;
use crate::ai_requests::AiPendingRequestProjection;
use crate::ai_session::AiSessionCatalogProjection;
use crate::ai_timeline_models::AiTimelineProjection;

pub struct AiProjectedSnapshot {
    pub workspace_key: String,
    pub authentication_required: bool,
    pub account: AiAccountProjection,
    pub threads: AiThreadCatalogProjection,
    pub timeline: AiTimelineProjection,
    pub queue: AiQueueProjection,
    pub requests: AiPendingRequestProjection,
    pub session: AiSessionCatalogProjection,
}

pub enum AiRuntimeEvent {
    Worker(Box<AiWorkerEvent>),
    Snapshot(Box<AiProjectedSnapshot>),
    Disconnected,
}

#[derive(Default)]
struct AiEventMailboxState {
    epoch: i32,
    callback_scheduled: bool,
    events: Vec<AiRuntimeEvent>,
}

#[derive(Default)]
pub struct AiEventMailbox {
    state: Mutex<AiEventMailboxState>,
    bookmarked_thread_ids: Mutex<BTreeSet<String>>,
    timeline_markdown_cache: Mutex<AiMarkdownProjectionCache>,
}

pub struct AiRuntimeSession {
    workspace_key: String,
    command_tx: Sender<AiWorkerCommand>,
    worker_thread: Option<JoinHandle<()>>,
    listener_thread: Option<JoinHandle<()>>,
}

#[derive(Default)]
pub struct AiRuntimeSlot {
    pub session: Option<AiRuntimeSession>,
    pub mailbox: Arc<AiEventMailbox>,
}

impl Drop for AiRuntimeSlot {
    fn drop(&mut self) {
        self.mailbox.reset(i32::MIN);
        self.session.take();
    }
}

impl AiRuntimeSession {
    pub fn workspace_key(&self) -> &str {
        self.workspace_key.as_str()
    }

    pub fn send(&self, command: AiWorkerCommand) -> Result<(), String> {
        self.command_tx
            .send(command)
            .map_err(|_| "Codex worker is no longer available.".to_owned())
    }
}

impl Drop for AiRuntimeSession {
    fn drop(&mut self) {
        let _ = self.command_tx.send(AiWorkerCommand::Shutdown);
        let worker_thread = self.worker_thread.take();
        let listener_thread = self.listener_thread.take();
        if worker_thread.is_none() && listener_thread.is_none() {
            return;
        }

        let spawn_result = std::thread::Builder::new()
            .name("hunk-desktop-ai-reaper".to_owned())
            .spawn(move || {
                if let Some(worker_thread) = worker_thread {
                    let _ = worker_thread.join();
                }
                if let Some(listener_thread) = listener_thread {
                    let _ = listener_thread.join();
                }
            });
        if let Err(error) = spawn_result {
            tracing::warn!(%error, "failed to start the Qt AI worker reaper");
        }
    }
}

pub fn prepare_ai_worker_config(
    root: &Path,
    mad_max_mode: bool,
    include_hidden_models: bool,
) -> Result<AiWorkerStartConfig, String> {
    let codex_home = resolve_codex_home_path()
        .ok_or_else(|| "Unable to resolve the Codex home directory.".to_owned())?;
    let codex_executable = resolve_codex_executable_path();
    validate_codex_executable_path(codex_executable.as_path())?;
    let mut config = AiWorkerStartConfig::new(root.to_path_buf(), codex_executable, codex_home);
    config.mad_max_mode = mad_max_mode;
    config.include_hidden_models = include_hidden_models;
    config.browser_tools_enabled = cfg!(feature = "cef-browser");
    Ok(config)
}

pub fn start_ai_runtime<F>(
    config: AiWorkerStartConfig,
    epoch: i32,
    mailbox: Arc<AiEventMailbox>,
    schedule_qt_drain: F,
) -> Result<AiRuntimeSession, String>
where
    F: Fn(i32) + Send + 'static,
{
    let workspace_key = config.workspace_key.clone();
    let (command_tx, command_rx) = std::sync::mpsc::channel();
    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let listener_thread = std::thread::Builder::new()
        .name("hunk-desktop-ai-events".to_owned())
        .spawn(move || {
            while let Ok(event) = event_rx.recv() {
                if mailbox.enqueue_worker(epoch, event) {
                    schedule_qt_drain(epoch);
                }
            }
            if mailbox.enqueue_disconnected(epoch) {
                schedule_qt_drain(epoch);
            }
        })
        .map_err(|error| format!("Failed to start the Codex event listener: {error}"))?;
    let worker_thread = spawn_ai_worker(config, command_rx, event_tx);

    Ok(AiRuntimeSession {
        workspace_key,
        command_tx,
        worker_thread: Some(worker_thread),
        listener_thread: Some(listener_thread),
    })
}

impl AiEventMailbox {
    pub fn set_bookmarked_thread_ids(&self, thread_ids: BTreeSet<String>) {
        *self
            .bookmarked_thread_ids
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = thread_ids;
    }

    pub fn reset(&self, epoch: i32) {
        self.timeline_markdown_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.epoch = epoch;
        state.callback_scheduled = false;
        let discarded = std::mem::take(&mut state.events);
        drop(state);
        reject_browser_calls(
            discarded,
            "The AI workspace changed before the tool call ran.",
        );
    }

    pub fn enqueue_worker(&self, epoch: i32, event: AiWorkerEvent) -> bool {
        {
            let state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if state.epoch != epoch {
                drop(state);
                reject_browser_call(event, "The AI workspace changed before the tool call ran.");
                return false;
            }
        }

        let bookmarked_thread_ids = self
            .bookmarked_thread_ids
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let event = project_worker_event(
            event,
            &bookmarked_thread_ids,
            &mut self
                .timeline_markdown_cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.epoch != epoch {
            drop(state);
            reject_runtime_event(event, "The AI workspace changed before the tool call ran.");
            return false;
        }

        let event_is_snapshot = matches!(&event, AiRuntimeEvent::Snapshot(_));
        let tail_is_snapshot = state
            .events
            .last()
            .is_some_and(|tail| matches!(tail, AiRuntimeEvent::Snapshot(_)));
        let superseded = if event_is_snapshot && tail_is_snapshot {
            state.events.pop()
        } else {
            None
        };
        state.events.push(event);
        let should_schedule = if state.callback_scheduled {
            false
        } else {
            state.callback_scheduled = true;
            true
        };
        drop(state);
        drop(superseded);
        should_schedule
    }

    pub fn enqueue_disconnected(&self, epoch: i32) -> bool {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.epoch != epoch {
            return false;
        }
        state.events.push(AiRuntimeEvent::Disconnected);
        if state.callback_scheduled {
            false
        } else {
            state.callback_scheduled = true;
            true
        }
    }

    pub fn take(&self, epoch: i32) -> Vec<AiRuntimeEvent> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.epoch != epoch {
            return Vec::new();
        }
        state.callback_scheduled = false;
        std::mem::take(&mut state.events)
    }
}

pub fn reject_browser_call(event: AiWorkerEvent, message: &str) {
    if let AiWorkerEventPayload::BrowserToolCall {
        params,
        response_tx,
    } = event.payload
    {
        let _ = response_tx.send(browser_unavailable_response(&params, message));
    }
}

fn project_worker_event(
    event: AiWorkerEvent,
    bookmarked_thread_ids: &BTreeSet<String>,
    timeline_markdown_cache: &mut AiMarkdownProjectionCache,
) -> AiRuntimeEvent {
    let AiWorkerEvent {
        workspace_key,
        payload,
    } = event;
    match payload {
        AiWorkerEventPayload::Snapshot(snapshot) => {
            let authentication_required =
                snapshot.requires_openai_auth && snapshot.account.is_none();
            let account = AiAccountProjection::from_snapshot(
                snapshot.account.as_ref(),
                snapshot.requires_openai_auth,
                snapshot.pending_chatgpt_login_id.as_deref(),
                snapshot.rate_limits.as_ref(),
            );
            let session = AiSessionCatalogProjection::from_snapshot(
                &snapshot.state,
                snapshot.active_thread_id.as_deref(),
                snapshot.models.as_slice(),
                snapshot.mad_max_mode,
                snapshot.include_hidden_models,
            );
            let mut threads = AiThreadCatalogProjection::from_state_with_bookmarks(
                &snapshot.state,
                snapshot.active_thread_id.as_deref(),
                bookmarked_thread_ids,
            );
            let timeline = AiTimelineProjection::from_state_with_markdown_cache(
                &snapshot.state,
                (!threads.active_thread_id.is_empty()).then_some(threads.active_thread_id.as_str()),
                timeline_markdown_cache,
            );
            let visible_thread_ids = threads
                .items
                .iter()
                .map(|thread| thread.thread_id.as_str())
                .collect::<Vec<_>>();
            let requests = AiPendingRequestProjection::from_pending(
                (!threads.active_thread_id.is_empty()).then_some(threads.active_thread_id.as_str()),
                snapshot.pending_approvals.as_slice(),
                snapshot.pending_user_inputs.as_slice(),
                visible_thread_ids.as_slice(),
            );
            let queue =
                AiQueueProjection::from_state(&snapshot.state, visible_thread_ids.as_slice());
            threads.mark_attention(requests.attention_thread_ids());
            AiRuntimeEvent::Snapshot(Box::new(AiProjectedSnapshot {
                workspace_key,
                authentication_required,
                account,
                threads,
                timeline,
                queue,
                requests,
                session,
            }))
        }
        payload => AiRuntimeEvent::Worker(Box::new(AiWorkerEvent {
            workspace_key,
            payload,
        })),
    }
}

fn reject_runtime_event(event: AiRuntimeEvent, message: &str) {
    if let AiRuntimeEvent::Worker(event) = event {
        reject_browser_call(*event, message);
    }
}

fn reject_browser_calls(events: Vec<AiRuntimeEvent>, message: &str) {
    for event in events {
        reject_runtime_event(event, message);
    }
}
