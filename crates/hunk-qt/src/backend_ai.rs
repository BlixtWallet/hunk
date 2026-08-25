use std::path::Path;
use std::sync::Arc;

use hunk_app::ai::{AiApprovalDecision, AiWorkerCommand, AiWorkerEventPayload};
use qtbridge::{QObjectHolder, invoke_method};

use crate::AiPromptReceipt;
use crate::ai_attachments::{
    clear_ai_attachment_drafts, clear_ai_attachments_for_thread, current_ai_attachment_paths,
    restore_ai_attachment_paths, sync_ai_attachments,
};
use crate::ai_requests::AiPendingRequestProjection;
use crate::ai_runtime::{
    AiProjectedSnapshot, AiRuntimeEvent, prepare_ai_worker_config, reject_browser_call,
    start_ai_runtime,
};
use crate::ai_session::{
    ai_turn_session_overrides, apply_ai_session_projection, reset_ai_session_projection,
};
use crate::ai_thread_actions::AiThreadActionReceipt;
use crate::backend_state::Backend;

pub(super) fn reset_ai_runtime_state(backend: &mut Backend) {
    stop_ai_runtime(backend);
    backend.ai_threads.borrow_mut().replace(Vec::new());
    backend.ai_timeline.borrow_mut().replace(Vec::new());
    clear_ai_attachment_drafts(backend);
    backend
        .ai_message_queue
        .reset_pending_after_runtime_failure();
    backend.ai_ready = false;
    backend.ai_loading = false;
    backend.ai_requires_authentication = false;
    backend.ai_connection_state = "disconnected".to_owned();
    backend.ai_workspace_root.clear();
    backend.ai_active_thread_id.clear();
    backend
        .browser
        .borrow_mut()
        .set_active_thread(String::new());
    backend.ai_active_thread_title.clear();
    backend.ai_active_thread_cwd.clear();
    backend.ai_active_turn_id.clear();
    backend.ai_turn_running = false;
    reset_ai_session_projection(backend);
    backend.ai_session_state_changed();
    backend.ai_prompt_receipt = None;
    backend.ai_thread_action = None;
    backend.ai_interrupt_thread_id.clear();
    backend.ai_interrupt_turn_id.clear();
    backend.ai_requests = AiPendingRequestProjection::default();
    backend.ai_request_resolving_id.clear();
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
                    AiWorkerEventPayload::ThreadStarted { thread_id } => {
                        let recorded = backend
                            .ai_thread_action
                            .as_mut()
                            .is_some_and(|receipt| receipt.record_started_thread(thread_id));
                        if !recorded {
                            backend.ai_status_message = "Created a new Codex thread.".to_owned();
                        }
                    }
                    AiWorkerEventPayload::SteerAccepted(pending) => {
                        let direct_prompt_accepted = backend
                            .ai_prompt_receipt
                            .as_ref()
                            .is_some_and(|receipt| receipt.thread_id() == pending.thread_id);
                        if direct_prompt_accepted {
                            accept_pending_prompt(backend);
                        }
                        let queued_prompt_accepted = !direct_prompt_accepted
                            && backend.ai_message_queue.accept_steer(
                                pending.thread_id.as_str(),
                                pending.prompt.as_str(),
                                pending.local_images.as_slice(),
                            );
                        if queued_prompt_accepted {
                            sync_ai_timeline_queue(backend);
                        }
                        backend.ai_status_message = if queued_prompt_accepted {
                            "Sent the next queued message to Codex."
                        } else {
                            "Added the message to the active turn."
                        }
                        .to_owned();
                    }
                    AiWorkerEventPayload::BrowserToolCall {
                        params,
                        response_tx,
                    } => {
                        backend
                            .browser
                            .borrow_mut()
                            .handle_ai_tool_call(params, response_tx);
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
        threads: mut projection,
        timeline,
        queue,
        requests,
        session,
        ..
    } = projected;
    let active_thread_changed = backend.ai_active_thread_id != projection.active_thread_id;
    projection.apply_bookmarks(&backend.ai_bookmarked_thread_ids);
    let completed_thread_action = backend
        .ai_thread_action
        .as_ref()
        .filter(|receipt| receipt.is_complete(&projection))
        .cloned();
    if completed_thread_action.is_some() {
        backend.ai_thread_action = None;
    }
    if backend.ai_prompt_receipt.as_ref().is_some_and(|receipt| {
        receipt.is_accepted_by(projection.active_thread_id.as_str(), &timeline)
    }) {
        accept_pending_prompt(backend);
    }
    backend.ai_message_queue.reconcile(&queue);
    if !backend.ai_interrupt_turn_id.is_empty()
        && projection.active_thread_id == backend.ai_interrupt_thread_id
        && timeline.active_turn_id != backend.ai_interrupt_turn_id
    {
        backend.ai_interrupt_thread_id.clear();
        backend.ai_interrupt_turn_id.clear();
    }
    if !backend.ai_request_resolving_id.is_empty()
        && requests
            .current
            .as_ref()
            .map(|request| request.request_id.as_str())
            != Some(backend.ai_request_resolving_id.as_str())
    {
        backend.ai_request_resolving_id.clear();
    }
    backend.ai_attachment_drafts.retain_threads(
        projection
            .items
            .iter()
            .map(|thread| thread.thread_id.as_str()),
    );
    backend
        .ai_threads
        .borrow_mut()
        .replace_if_changed(projection.items);
    let timeline_items = timeline.items;
    backend.ai_active_thread_id = projection.active_thread_id;
    if active_thread_changed {
        backend
            .browser
            .borrow_mut()
            .set_active_thread(backend.ai_active_thread_id.clone());
    }
    if active_thread_changed {
        sync_ai_attachments(backend);
    }
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
    backend.ai_requests = requests;
    let session_changed = backend.ai_session_catalog != session;
    let previous_session_selection = active_thread_changed.then(|| {
        (
            backend.ai_selected_model.clone(),
            backend.ai_selected_effort.clone(),
            backend.ai_selected_collaboration_mode.clone(),
            backend.ai_selected_service_tier.clone(),
            backend.ai_mad_max_mode,
        )
    });
    apply_ai_session_projection(backend, session);
    if session_changed
        || previous_session_selection.is_some_and(|previous| {
            (
                backend.ai_selected_model.as_str(),
                backend.ai_selected_effort.as_str(),
                backend.ai_selected_collaboration_mode.as_str(),
                backend.ai_selected_service_tier.as_str(),
                backend.ai_mad_max_mode,
            ) != (
                previous.0.as_str(),
                previous.1.as_str(),
                previous.2.as_str(),
                previous.3.as_str(),
                previous.4,
            )
        })
    {
        backend.ai_session_state_changed();
    }
    backend.ai_requires_authentication = requires_openai_auth;
    backend.ai_ready = true;
    backend.ai_loading = false;
    backend.ai_connection_state = "ready".to_owned();
    backend.ai_error.clear();
    if let Some(receipt) = completed_thread_action {
        backend.ai_status_message = receipt.completion_message().to_owned();
    } else if backend.ai_thread_action.is_none()
        && !crate::ai_attachments::ai_attachment_pending(backend)
    {
        backend.ai_status_message.clear();
    }
    maybe_submit_ready_ai_queued_messages(backend, &queue);
    sync_ai_timeline_snapshot(backend, timeline_items);
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
    let local_image_paths = current_ai_attachment_paths(backend);
    if (prompt.is_empty() && local_image_paths.is_empty())
        || !backend.ai_ready
        || backend.ai_loading
        || backend.ai_requires_authentication
        || backend.ai_active_thread_id.is_empty()
        || backend.ai_thread_action.is_some()
        || backend.ai_prompt_receipt.is_some()
        || crate::ai_attachments::ai_attachment_pending(backend)
        || backend
            .ai_message_queue
            .thread_is_sending(backend.ai_active_thread_id.as_str())
        || !backend.ai_interrupt_turn_id.is_empty()
        || backend.ai_requests.current.is_some()
        || !backend.ai_request_resolving_id.is_empty()
    {
        return false;
    }
    if !local_image_paths.is_empty()
        && !backend
            .ai_session_catalog
            .model_supports_image_inputs(Some(backend.ai_selected_model.as_str()))
    {
        return reject_ai_prompt_input(
            backend,
            "Selected model does not support image attachments. Remove attachments or switch models.",
        );
    }
    let receipt = AiPromptReceipt::new(
        backend.ai_active_thread_id.clone(),
        backend.ai_active_turn_id.clone(),
        backend.ai_timeline_total_turn_count,
    );
    let thread_id = backend.ai_active_thread_id.clone();
    let command = AiWorkerCommand::SendPrompt {
        thread_id: thread_id.clone(),
        prompt: (!prompt.is_empty()).then(|| prompt.to_owned()),
        local_image_paths,
        selected_skills: Vec::new(),
        skill_bindings: Vec::new(),
        session_overrides: ai_turn_session_overrides(backend, Some(thread_id.as_str())),
    };
    if !send_ai_command(backend, command, "Sending message to Codex…") {
        return false;
    }
    backend.ai_prompt_receipt = Some(receipt);
    true
}

pub(super) fn queue_ai_follow_up(backend: &mut Backend, prompt: String) -> bool {
    let prompt = prompt.trim();
    let local_image_paths = current_ai_attachment_paths(backend);
    if !backend.ai_ready
        || backend.ai_loading
        || backend.ai_requires_authentication
        || backend.ai_active_thread_id.is_empty()
        || backend.ai_active_turn_id.is_empty()
        || !backend.ai_turn_running
        || backend.ai_thread_action.is_some()
        || backend.ai_prompt_receipt.is_some()
        || crate::ai_attachments::ai_attachment_pending(backend)
        || !backend.ai_interrupt_thread_id.is_empty()
        || backend.ai_requests.current.is_some()
        || !backend.ai_request_resolving_id.is_empty()
        || (prompt.is_empty() && local_image_paths.is_empty())
    {
        return false;
    }
    if !local_image_paths.is_empty()
        && !backend
            .ai_session_catalog
            .model_supports_image_inputs(Some(backend.ai_selected_model.as_str()))
    {
        return reject_ai_prompt_input(
            backend,
            "Selected model does not support image attachments. Remove attachments or switch models.",
        );
    }
    match backend.ai_message_queue.enqueue_with_attachments(
        backend.ai_active_thread_id.clone(),
        prompt.to_owned(),
        local_image_paths,
    ) {
        Ok(()) => {
            let thread_id = backend.ai_active_thread_id.clone();
            clear_ai_attachments_for_thread(backend, thread_id.as_str());
            backend.ai_error.clear();
            backend.ai_status_message = "Queued a follow-up for this thread.".to_owned();
            sync_ai_timeline_queue(backend);
            true
        }
        Err(error) => {
            backend.ai_status_message = error.to_owned();
            false
        }
    }
}

pub(super) fn edit_last_ai_queued_prompt(backend: &mut Backend) -> String {
    let Some(message) = backend
        .ai_message_queue
        .edit_latest_with_attachments(backend.ai_active_thread_id.as_str())
    else {
        return String::new();
    };
    let prompt = message.prompt;
    restore_ai_attachment_paths(
        backend,
        message.thread_id.as_str(),
        message.local_image_paths,
    );
    backend.ai_status_message = "Moved the newest queued message back to the draft.".to_owned();
    sync_ai_timeline_queue(backend);
    prompt
}

pub(super) fn take_ai_recovered_prompt(backend: &mut Backend, thread_id: String) -> String {
    let draft = backend
        .ai_message_queue
        .take_recovered_draft(thread_id.as_str());
    restore_ai_attachment_paths(backend, thread_id.as_str(), draft.local_image_paths);
    draft.prompt
}

pub(super) fn queue_ai_interrupt(backend: &mut Backend) -> bool {
    if !backend.ai_ready
        || backend.ai_loading
        || backend.ai_active_thread_id.is_empty()
        || backend.ai_active_turn_id.is_empty()
        || backend.ai_thread_action.is_some()
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
    backend
        .ai_message_queue
        .mark_interrupt_restore(backend.ai_interrupt_thread_id.clone());
    true
}

pub(super) fn queue_ai_approval(backend: &mut Backend, request_id: String, accept: bool) -> bool {
    let valid =
        backend.ai_request_resolving_id.is_empty()
            && backend.ai_ready
            && !backend.ai_loading
            && !backend.ai_requires_authentication
            && backend.ai_thread_action.is_none()
            && backend.ai_requests.current.as_ref().is_some_and(|request| {
                request.kind == "approval" && request.request_id == request_id
            });
    if !valid {
        return reject_ai_request(backend, "The pending Codex approval changed.");
    }
    let decision = if accept {
        AiApprovalDecision::Accept
    } else {
        AiApprovalDecision::Decline
    };
    if !send_ai_command(
        backend,
        AiWorkerCommand::ResolveApproval {
            request_id: request_id.clone(),
            decision,
        },
        if accept {
            "Accepting Codex approval…"
        } else {
            "Declining Codex approval…"
        },
    ) {
        return false;
    }
    backend.ai_request_resolving_id = request_id;
    true
}

pub(super) fn queue_ai_user_input(
    backend: &mut Backend,
    request_id: String,
    answers_json: String,
) -> bool {
    if !backend.ai_ready
        || backend.ai_loading
        || backend.ai_requires_authentication
        || backend.ai_thread_action.is_some()
        || !backend.ai_request_resolving_id.is_empty()
    {
        return false;
    }
    let answers = match backend
        .ai_requests
        .validated_answers(request_id.as_str(), answers_json.as_str())
    {
        Ok(answers) => answers,
        Err(error) => return reject_ai_request(backend, error.as_str()),
    };
    if !send_ai_command(
        backend,
        AiWorkerCommand::SubmitUserInput {
            request_id: request_id.clone(),
            answers,
        },
        "Submitting input to Codex…",
    ) {
        return false;
    }
    backend.ai_request_resolving_id = request_id;
    true
}

pub(super) fn queue_ai_select_thread(backend: &mut Backend, thread_id: String) -> bool {
    let thread_id = thread_id.trim();
    if thread_action_blocked(backend)
        || thread_id.is_empty()
        || thread_id == backend.ai_active_thread_id
        || !backend.ai_threads.borrow().contains_thread_id(thread_id)
    {
        return false;
    }
    let thread_id = thread_id.to_owned();
    queue_ai_thread_action(
        backend,
        AiThreadActionReceipt::select(thread_id.clone()),
        AiWorkerCommand::SelectThread { thread_id },
        "Opening Codex thread…",
    )
}

pub(super) fn queue_ai_create_thread(backend: &mut Backend) -> bool {
    if thread_action_blocked(backend) {
        return false;
    }
    let session_overrides = ai_turn_session_overrides(backend, None);
    queue_ai_thread_action(
        backend,
        AiThreadActionReceipt::create(),
        AiWorkerCommand::StartThread {
            prompt: None,
            local_image_paths: Vec::new(),
            selected_skills: Vec::new(),
            skill_bindings: Vec::new(),
            session_overrides,
        },
        "Creating a Codex thread…",
    )
}

pub(super) fn queue_ai_fork_thread(backend: &mut Backend) -> bool {
    let thread_id = backend.ai_active_thread_id.clone();
    if thread_action_blocked(backend)
        || backend.ai_turn_running
        || thread_id.is_empty()
        || !backend
            .ai_threads
            .borrow()
            .contains_thread_id(thread_id.as_str())
    {
        return false;
    }
    queue_ai_thread_action(
        backend,
        AiThreadActionReceipt::fork(thread_id.clone()),
        AiWorkerCommand::ForkThread { thread_id },
        "Forking Codex thread…",
    )
}

pub(super) fn queue_ai_archive_thread(backend: &mut Backend, thread_id: String) -> bool {
    let thread_id = thread_id.trim();
    if thread_action_blocked(backend)
        || thread_id.is_empty()
        || backend.ai_requests.thread_needs_attention(thread_id)
        || backend.ai_message_queue.thread_count(thread_id) > 0
        || !backend.ai_threads.borrow().contains_thread_id(thread_id)
    {
        return false;
    }
    let thread_id = thread_id.to_owned();
    queue_ai_thread_action(
        backend,
        AiThreadActionReceipt::archive(thread_id.clone()),
        AiWorkerCommand::ArchiveThread { thread_id },
        "Archiving Codex thread…",
    )
}

fn queue_ai_thread_action(
    backend: &mut Backend,
    receipt: AiThreadActionReceipt,
    command: AiWorkerCommand,
    status: &str,
) -> bool {
    if !send_ai_command(backend, command, status) {
        return false;
    }
    backend.ai_thread_action = Some(receipt);
    true
}

fn thread_action_blocked(backend: &Backend) -> bool {
    !backend.ai_ready
        || backend.ai_loading
        || backend.ai_requires_authentication
        || backend.ai_thread_action.is_some()
        || backend.ai_prompt_receipt.is_some()
        || !backend.ai_interrupt_thread_id.is_empty()
        || backend.ai_requests.current.is_some()
        || !backend.ai_request_resolving_id.is_empty()
}

pub(super) fn ensure_ai_runtime_started(backend: &mut Backend) -> bool {
    let workspace_key = backend.git_root.clone();
    if backend
        .ai_runtime
        .session
        .as_ref()
        .is_some_and(|runtime| runtime.workspace_key() == workspace_key.as_str())
    {
        return false;
    }
    if !backend.git_ready {
        backend.ai_loading = true;
        backend.ai_connection_state = "waiting".to_owned();
        backend.ai_status_message = "Waiting for the repository to load…".to_owned();
        return true;
    }

    reset_ai_runtime_state(backend);
    backend.ai_workspace_root = workspace_key.clone();
    let mad_max_mode = backend
        .ai_session_preferences
        .workspace_mad_max(workspace_key.as_str());
    let include_hidden_models = backend
        .ai_session_preferences
        .workspace_include_hidden_models(workspace_key.as_str());
    let config = match prepare_ai_worker_config(
        Path::new(workspace_key.as_str()),
        mad_max_mode,
        include_hidden_models,
    ) {
        Ok(config) => config,
        Err(error) => {
            backend.ai_connection_state = "failed".to_owned();
            backend.ai_error = error.clone();
            backend.ai_status_message = error;
            return true;
        }
    };
    let starting_status_message = config.starting_status_message();
    let epoch = backend.ai_epoch;
    let mailbox = Arc::clone(&backend.ai_runtime.mailbox);
    let invoker = backend.get_qml_method_invoker();
    let start_result = start_ai_runtime(config, epoch, mailbox, move |event_epoch| {
        invoke_method!(invoker, "apply_ai_events", event_epoch);
    });
    match start_result {
        Ok(runtime) => {
            backend.ai_runtime.session = Some(runtime);
            backend.ai_loading = true;
            backend.ai_connection_state = "connecting".to_owned();
            backend.ai_status_message = starting_status_message;
        }
        Err(error) => {
            backend.ai_connection_state = "failed".to_owned();
            backend.ai_error = error.clone();
            backend.ai_status_message = error;
        }
    }
    true
}

pub(super) fn send_ai_worker_command(
    backend: &mut Backend,
    command: AiWorkerCommand,
    status_message: &str,
) -> bool {
    send_ai_command(backend, command, status_message)
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
    let thread_id = backend
        .ai_prompt_receipt
        .as_ref()
        .map(|receipt| receipt.thread_id().to_owned());
    backend.ai_prompt_receipt = None;
    if let Some(thread_id) = thread_id {
        clear_ai_attachments_for_thread(backend, thread_id.as_str());
    }
    backend.ai_prompt_accepted_revision =
        backend.ai_prompt_accepted_revision.wrapping_add(1).max(1);
}

fn clear_pending_ai_commands(backend: &mut Backend) {
    backend.ai_prompt_receipt = None;
    backend.ai_thread_action = None;
    backend.ai_interrupt_thread_id.clear();
    backend.ai_interrupt_turn_id.clear();
    backend.ai_request_resolving_id.clear();
    backend
        .ai_message_queue
        .reset_pending_after_runtime_failure();
    sync_ai_timeline_queue(backend);
}

pub(super) fn clear_ai_message_queue(backend: &mut Backend) {
    backend.ai_message_queue.clear();
    sync_ai_timeline_queue(backend);
}

fn maybe_submit_ready_ai_queued_messages(
    backend: &mut Backend,
    projection: &crate::AiQueueProjection,
) {
    if backend.ai_thread_action.is_some() || !backend.ai_ready {
        return;
    }
    let mut blocked_thread_ids = backend.ai_requests.attention_thread_ids().clone();
    if let Some(receipt) = &backend.ai_prompt_receipt {
        blocked_thread_ids.insert(receipt.thread_id().to_owned());
    }
    if !backend.ai_interrupt_thread_id.is_empty() {
        blocked_thread_ids.insert(backend.ai_interrupt_thread_id.clone());
    }
    let ready_thread_ids = backend
        .ai_message_queue
        .ready_thread_ids(projection, &blocked_thread_ids);
    for thread_id in ready_thread_ids {
        let Some(thread) = projection.threads.get(thread_id.as_str()) else {
            continue;
        };
        let session_overrides = ai_turn_session_overrides(backend, Some(thread_id.as_str()));
        if backend
            .ai_message_queue
            .next_queued_has_attachments(thread_id.as_str())
            && !backend
                .ai_session_catalog
                .model_supports_image_inputs(session_overrides.model.as_deref())
        {
            if backend.ai_active_thread_id == thread_id {
                backend.ai_status_message =
                    "Queued image attachments need an image-capable model.".to_owned();
            }
            continue;
        }
        let Some(queued) = backend
            .ai_message_queue
            .mark_next_pending(thread_id.as_str(), thread.latest_sequence)
        else {
            continue;
        };
        let sent = send_ai_command(
            backend,
            AiWorkerCommand::SendPrompt {
                session_overrides,
                thread_id: queued.thread_id,
                prompt: (!queued.prompt.trim().is_empty()).then_some(queued.prompt),
                local_image_paths: queued.local_image_paths,
                selected_skills: Vec::new(),
                skill_bindings: Vec::new(),
            },
            "Sending the next queued message to Codex…",
        );
        if !sent {
            break;
        }
    }
}

fn sync_ai_timeline_snapshot(backend: &mut Backend, mut items: Vec<crate::AiTimelineItem>) {
    let queue_items = backend
        .ai_message_queue
        .timeline_items(backend.ai_active_thread_id.as_str());
    let hidden_authoritative_rows = items
        .len()
        .saturating_add(queue_items.len())
        .saturating_sub(crate::AI_TIMELINE_MAX_VISIBLE_ROWS)
        .min(items.len());
    items.drain(..hidden_authoritative_rows);
    items.extend(queue_items);
    backend.ai_timeline.borrow_mut().sync(items);
    backend.ai_timeline_hidden_row_count = backend
        .ai_timeline_hidden_row_count
        .saturating_add(i32::try_from(hidden_authoritative_rows).unwrap_or(i32::MAX));
}

fn sync_ai_timeline_queue(backend: &mut Backend) {
    let items = backend
        .ai_message_queue
        .timeline_items(backend.ai_active_thread_id.as_str());
    let (_, hidden_authoritative_rows) = backend.ai_timeline.borrow_mut().sync_queue_items(items);
    backend.ai_timeline_hidden_row_count = backend
        .ai_timeline_hidden_row_count
        .saturating_add(i32::try_from(hidden_authoritative_rows).unwrap_or(i32::MAX));
}

fn reject_ai_request(backend: &mut Backend, message: &str) -> bool {
    backend.ai_error = message.to_owned();
    backend.ai_status_message = message.to_owned();
    false
}

fn reject_ai_prompt_input(backend: &mut Backend, message: &str) -> bool {
    backend.ai_status_message = message.to_owned();
    false
}

pub(super) fn ai_prompt_pending(backend: &Backend) -> bool {
    backend.ai_prompt_receipt.is_some()
}

pub(super) fn ai_queued_message_count(backend: &Backend) -> i32 {
    i32::try_from(backend.ai_message_queue.total_count()).unwrap_or(i32::MAX)
}

pub(super) fn ai_active_queued_message_count(backend: &Backend) -> i32 {
    i32::try_from(
        backend
            .ai_message_queue
            .thread_count(backend.ai_active_thread_id.as_str()),
    )
    .unwrap_or(i32::MAX)
}

pub(super) fn ai_active_queue_sending(backend: &Backend) -> bool {
    backend
        .ai_message_queue
        .thread_is_sending(backend.ai_active_thread_id.as_str())
}

pub(super) fn ai_interrupt_pending(backend: &Backend) -> bool {
    !backend.ai_interrupt_turn_id.is_empty()
}

pub(super) fn ai_thread_action_pending(backend: &Backend) -> bool {
    backend.ai_thread_action.is_some()
}

pub(super) fn ai_pending_request_count(backend: &Backend) -> i32 {
    backend.ai_requests.total_count
}

pub(super) fn ai_active_request_count(backend: &Backend) -> i32 {
    backend.ai_requests.active_count
}

pub(super) fn ai_request_id(backend: &Backend) -> String {
    backend
        .ai_requests
        .current
        .as_ref()
        .map(|request| request.request_id.clone())
        .unwrap_or_default()
}

pub(super) fn ai_request_kind(backend: &Backend) -> String {
    backend
        .ai_requests
        .current
        .as_ref()
        .map(|request| request.kind.clone())
        .unwrap_or_default()
}

pub(super) fn ai_request_title(backend: &Backend) -> String {
    backend
        .ai_requests
        .current
        .as_ref()
        .map(|request| request.title.clone())
        .unwrap_or_default()
}

pub(super) fn ai_request_description(backend: &Backend) -> String {
    backend
        .ai_requests
        .current
        .as_ref()
        .map(|request| request.description.clone())
        .unwrap_or_default()
}

pub(super) fn ai_request_reason(backend: &Backend) -> String {
    backend
        .ai_requests
        .current
        .as_ref()
        .map(|request| request.reason.clone())
        .unwrap_or_default()
}

pub(super) fn ai_request_questions_json(backend: &Backend) -> String {
    backend.ai_requests.questions_json()
}

pub(super) fn ai_request_answerable(backend: &Backend) -> bool {
    backend
        .ai_requests
        .current
        .as_ref()
        .is_some_and(|request| request.answerable)
}

pub(super) fn ai_request_resolving(backend: &Backend) -> bool {
    !backend.ai_request_resolving_id.is_empty()
}
