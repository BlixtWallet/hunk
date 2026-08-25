use hunk_app::ai::{AiTurnSessionOverrides, AiWorkerCommand, AiWorkerEventPayload};

use crate::AiPromptReceipt;
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
    backend.ai_active_turn_id.clear();
    backend.ai_turn_running = false;
    backend.ai_prompt_receipt = None;
    backend.ai_interrupt_thread_id.clear();
    backend.ai_interrupt_turn_id.clear();
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
                    AiWorkerEventPayload::SteerAccepted(pending) => {
                        if backend
                            .ai_prompt_receipt
                            .as_ref()
                            .is_some_and(|receipt| receipt.thread_id() == pending.thread_id)
                        {
                            accept_pending_prompt(backend);
                        }
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
                        clear_pending_ai_commands(backend);
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
    if backend.ai_prompt_receipt.as_ref().is_some_and(|receipt| {
        receipt.is_accepted_by(projection.active_thread_id.as_str(), &timeline)
    }) {
        accept_pending_prompt(backend);
    }
    if !backend.ai_interrupt_turn_id.is_empty()
        && projection.active_thread_id == backend.ai_interrupt_thread_id
        && timeline.active_turn_id != backend.ai_interrupt_turn_id
    {
        backend.ai_interrupt_thread_id.clear();
        backend.ai_interrupt_turn_id.clear();
    }
    backend
        .ai_threads
        .borrow_mut()
        .replace_if_changed(projection.items);
    backend.ai_timeline.borrow_mut().sync(timeline.items);
    backend.ai_active_thread_id = projection.active_thread_id;
    backend.ai_active_thread_title = projection.active_thread_title;
    backend.ai_active_thread_cwd = projection.active_thread_cwd;
    backend.ai_active_turn_id = timeline.active_turn_id;
    backend.ai_turn_running = timeline.turn_running;
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
    clear_pending_ai_commands(backend);
}

pub(super) fn queue_ai_prompt(backend: &mut Backend, prompt: String) -> bool {
    let prompt = prompt.trim();
    if prompt.is_empty()
        || !backend.ai_ready
        || backend.ai_loading
        || backend.ai_requires_authentication
        || backend.ai_active_thread_id.is_empty()
        || backend.ai_prompt_receipt.is_some()
        || !backend.ai_interrupt_turn_id.is_empty()
    {
        return false;
    }
    let receipt = AiPromptReceipt::new(
        backend.ai_active_thread_id.clone(),
        backend.ai_active_turn_id.clone(),
        backend.ai_timeline_total_turn_count,
    );
    let command = AiWorkerCommand::SendPrompt {
        thread_id: backend.ai_active_thread_id.clone(),
        prompt: Some(prompt.to_owned()),
        local_image_paths: Vec::new(),
        selected_skills: Vec::new(),
        skill_bindings: Vec::new(),
        session_overrides: AiTurnSessionOverrides::default(),
    };
    if !send_ai_command(backend, command, "Sending message to Codex…") {
        return false;
    }
    backend.ai_prompt_receipt = Some(receipt);
    true
}

pub(super) fn queue_ai_interrupt(backend: &mut Backend) -> bool {
    if !backend.ai_ready
        || backend.ai_loading
        || backend.ai_active_thread_id.is_empty()
        || backend.ai_active_turn_id.is_empty()
        || backend.ai_prompt_receipt.is_some()
        || !backend.ai_interrupt_turn_id.is_empty()
    {
        return false;
    }
    let thread_id = backend.ai_active_thread_id.clone();
    let turn_id = backend.ai_active_turn_id.clone();
    if !send_ai_command(
        backend,
        AiWorkerCommand::InterruptTurn {
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
        },
        "Stopping the active Codex turn…",
    ) {
        return false;
    }
    backend.ai_interrupt_thread_id = thread_id;
    backend.ai_interrupt_turn_id = turn_id;
    true
}

fn send_ai_command(backend: &mut Backend, command: AiWorkerCommand, status: &str) -> bool {
    let result = backend
        .ai_runtime
        .session
        .as_ref()
        .ok_or_else(|| "Codex worker is not connected.".to_owned())
        .and_then(|runtime| runtime.send(command));
    match result {
        Ok(()) => {
            backend.ai_error.clear();
            backend.ai_status_message = status.to_owned();
            true
        }
        Err(error) => {
            stop_ai_runtime(backend);
            fail_ai_runtime(backend, error);
            false
        }
    }
}

fn accept_pending_prompt(backend: &mut Backend) {
    backend.ai_prompt_receipt = None;
    backend.ai_prompt_accepted_revision =
        backend.ai_prompt_accepted_revision.wrapping_add(1).max(1);
}

fn clear_pending_ai_commands(backend: &mut Backend) {
    backend.ai_prompt_receipt = None;
    backend.ai_interrupt_thread_id.clear();
    backend.ai_interrupt_turn_id.clear();
}
