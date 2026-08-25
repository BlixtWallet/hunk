use std::path::Path;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use hunk_app::ai::{
    AiWorkerCommand, AiWorkerEvent, AiWorkerEventPayload, AiWorkerStartConfig,
    browser_unavailable_response, resolve_codex_executable_path, resolve_codex_home_path,
    spawn_ai_worker, validate_codex_executable_path,
};

use crate::ai_models::AiThreadCatalogProjection;
use crate::ai_timeline_models::AiTimelineProjection;

pub struct AiProjectedSnapshot {
    pub workspace_key: String,
    pub requires_openai_auth: bool,
    pub threads: AiThreadCatalogProjection,
    pub timeline: AiTimelineProjection,
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
            .name("hunk-qt-ai-reaper".to_owned())
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

pub fn prepare_ai_worker_config(root: &Path) -> Result<AiWorkerStartConfig, String> {
    let codex_home = resolve_codex_home_path()
        .ok_or_else(|| "Unable to resolve the Codex home directory.".to_owned())?;
    let codex_executable = resolve_codex_executable_path();
    validate_codex_executable_path(codex_executable.as_path())?;
    Ok(AiWorkerStartConfig::new(
        root.to_path_buf(),
        codex_executable,
        codex_home,
    ))
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
        .name("hunk-qt-ai-events".to_owned())
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
    pub fn reset(&self, epoch: i32) {
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

        let event = project_worker_event(event);
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

fn project_worker_event(event: AiWorkerEvent) -> AiRuntimeEvent {
    let AiWorkerEvent {
        workspace_key,
        payload,
    } = event;
    match payload {
        AiWorkerEventPayload::Snapshot(snapshot) => {
            let requires_openai_auth = snapshot.requires_openai_auth;
            let threads = AiThreadCatalogProjection::from_state(
                &snapshot.state,
                snapshot.active_thread_id.as_deref(),
            );
            let timeline = AiTimelineProjection::from_state(
                &snapshot.state,
                (!threads.active_thread_id.is_empty()).then_some(threads.active_thread_id.as_str()),
            );
            AiRuntimeEvent::Snapshot(Box::new(AiProjectedSnapshot {
                workspace_key,
                requires_openai_auth,
                threads,
                timeline,
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
