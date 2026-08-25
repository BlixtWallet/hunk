use hunk_app::ai::AiWorkerEventPayload;

use crate::ai_runtime::{AiProjectedSnapshot, AiRuntimeEvent, reject_browser_call};
use crate::backend_state::Backend;

pub(super) fn reset_ai_runtime_state(backend: &mut Backend) {
    stop_ai_runtime(backend);
    backend.ai_threads.borrow_mut().replace(Vec::new());
    backend.ai_timeline.borrow_mut().replace(Vec::new());
    backend.ai_ready = false;
    backend.ai_loading = false;
    backend.ai_requires_authentication = false;
    backend.ai_connection_state = "disconnected".to_owned();
    backend.ai_workspace_root.clear();
    backend.ai_active_thread_id.clear();
    backend.ai_active_thread_title.clear();
    backend.ai_active_thread_cwd.clear();
    backend.ai_thread_count = 0;
    backend.ai_running_thread_count = 0;
    backend.ai_timeline_total_turn_count = 0;
    backend.ai_timeline_visible_turn_count = 0;
    backend.ai_timeline_hidden_turn_count = 0;
    backend.ai_timeline_total_row_count = 0;
    backend.ai_timeline_hidden_row_count = 0;
    backend.ai_error.clear();
    backend.ai_status_message.clear();
}

pub(super) fn stop_ai_runtime(backend: &mut Backend) {
    backend.ai_epoch = backend.ai_epoch.wrapping_add(1).max(1);
    backend.ai_runtime.mailbox.reset(backend.ai_epoch);
    backend.ai_runtime.session.take();
}

pub(super) fn apply_ai_runtime_events(backend: &mut Backend, events: Vec<AiRuntimeEvent>) -> bool {
    let workspace_key = backend
        .ai_runtime
        .session
        .as_ref()
        .map(|runtime| runtime.workspace_key().to_owned());
    let mut stop_runtime = false;

    for event in events {
        match event {
            AiRuntimeEvent::Snapshot(snapshot) => {
                if workspace_key.as_deref() != Some(snapshot.workspace_key.as_str()) {
                    continue;
                }
                apply_ai_snapshot(backend, *snapshot);
            }
            AiRuntimeEvent::Worker(event) => {
                let event = *event;
                if workspace_key.as_deref() != Some(event.workspace_key.as_str()) {
                    reject_browser_call(
                        event,
                        "The AI workspace changed before the tool call ran.",
                    );
                    continue;
                }
                match event.payload {
                    AiWorkerEventPayload::Snapshot(_) => {
                        unreachable!("AI snapshots are projected before reaching the Qt thread")
                    }
                    AiWorkerEventPayload::BootstrapCompleted => {
                        backend.ai_loading = false;
                    }
                    AiWorkerEventPayload::ThreadStarted { .. } => {
                        backend.ai_status_message = "Created a new Codex thread.".to_owned();
                    }
                    AiWorkerEventPayload::SteerAccepted(_) => {
                        backend.ai_status_message =
                            "Added the message to the active turn.".to_owned();
                    }
                    AiWorkerEventPayload::BrowserToolCall {
                        params,
                        response_tx,
                    } => {
                        let response = hunk_app::ai::browser_unavailable_response(
                            &params,
                            "The embedded browser is not connected to the Qt frontend yet.",
                        );
                        let _ = response_tx.send(response);
                    }
                    AiWorkerEventPayload::Reconnecting(message) => {
                        backend.ai_connection_state = "reconnecting".to_owned();
                        backend.ai_loading = false;
                        backend.ai_error.clear();
                        backend.ai_status_message = message;
                    }
                    AiWorkerEventPayload::Status(message) => {
                        backend.ai_status_message = message;
                    }
                    AiWorkerEventPayload::Error(message) => {
                        backend.ai_loading = false;
                        backend.ai_error = message.clone();
                        backend.ai_status_message = message;
                    }
                    AiWorkerEventPayload::Fatal(message) => {
                        fail_ai_runtime(backend, message);
                        stop_runtime = true;
                    }
                }
            }
            AiRuntimeEvent::Disconnected => {
                if !stop_runtime && backend.ai_connection_state != "failed" {
                    fail_ai_runtime(backend, "Codex worker disconnected.".to_owned());
                }
                stop_runtime = true;
            }
        }
    }

    stop_runtime
}

fn apply_ai_snapshot(backend: &mut Backend, projected: AiProjectedSnapshot) {
    let AiProjectedSnapshot {
        requires_openai_auth,
        threads: projection,
        timeline,
        ..
    } = projected;
    backend
        .ai_threads
        .borrow_mut()
        .replace_if_changed(projection.items);
    backend.ai_timeline.borrow_mut().sync(timeline.items);
    backend.ai_active_thread_id = projection.active_thread_id;
    backend.ai_active_thread_title = projection.active_thread_title;
    backend.ai_active_thread_cwd = projection.active_thread_cwd;
    backend.ai_thread_count = projection.thread_count;
    backend.ai_running_thread_count = projection.running_thread_count;
    backend.ai_timeline_total_turn_count = timeline.total_turn_count;
    backend.ai_timeline_visible_turn_count = timeline.visible_turn_count;
    backend.ai_timeline_hidden_turn_count = timeline.hidden_turn_count;
    backend.ai_timeline_total_row_count = timeline.total_row_count;
    backend.ai_timeline_hidden_row_count = timeline.hidden_row_count;
    backend.ai_requires_authentication = requires_openai_auth;
    backend.ai_ready = true;
    backend.ai_loading = false;
    backend.ai_connection_state = "ready".to_owned();
    backend.ai_error.clear();
    backend.ai_status_message.clear();
}

fn fail_ai_runtime(backend: &mut Backend, message: String) {
    backend.ai_ready = false;
    backend.ai_loading = false;
    backend.ai_connection_state = "failed".to_owned();
    backend.ai_error = message.clone();
    backend.ai_status_message = message;
}
