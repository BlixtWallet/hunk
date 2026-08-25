use hunk_app::ai::{AiSnapshot, AiWorkerEvent, AiWorkerEventPayload};
use hunk_codex::protocol::DynamicToolCallParams;
use hunk_codex::state::AiState;
use hunk_qt::{AiEventMailbox, AiRuntimeEvent};

fn event(payload: AiWorkerEventPayload) -> AiWorkerEvent {
    AiWorkerEvent {
        workspace_key: "/repo".to_owned(),
        payload,
    }
}

fn snapshot(active_thread_id: &str) -> AiSnapshot {
    AiSnapshot {
        state: AiState::default(),
        active_thread_id: Some(active_thread_id.to_owned()),
        pending_approvals: Vec::new(),
        pending_user_inputs: Vec::new(),
        account: None,
        requires_openai_auth: false,
        pending_chatgpt_login_id: None,
        pending_chatgpt_auth_url: None,
        rate_limits: None,
        models: Vec::new(),
        experimental_features: Vec::new(),
        collaboration_modes: Vec::new(),
        skills: Vec::new(),
        include_hidden_models: true,
        mad_max_mode: false,
    }
}

fn browser_tool_call() -> (
    AiWorkerEventPayload,
    std::sync::mpsc::Receiver<hunk_codex::protocol::DynamicToolCallResponse>,
) {
    let (response_tx, response_rx) = std::sync::mpsc::channel();
    (
        AiWorkerEventPayload::BrowserToolCall {
            params: DynamicToolCallParams {
                thread_id: "thread".to_owned(),
                turn_id: "turn".to_owned(),
                call_id: "call".to_owned(),
                namespace: Some("browser".to_owned()),
                tool: "open".to_owned(),
                arguments: serde_json::Value::Null,
            },
            response_tx,
        },
        response_rx,
    )
}

#[test]
fn mailbox_schedules_once_until_the_qt_thread_drains_it() {
    let mailbox = AiEventMailbox::default();
    mailbox.reset(4);

    assert!(mailbox.enqueue_worker(4, event(AiWorkerEventPayload::Status("one".to_owned()))));
    assert!(!mailbox.enqueue_worker(4, event(AiWorkerEventPayload::Status("two".to_owned()))));
    assert_eq!(mailbox.take(4).len(), 2);
    assert!(mailbox.enqueue_disconnected(4));
    assert!(matches!(
        mailbox.take(4).as_slice(),
        [AiRuntimeEvent::Disconnected]
    ));
}

#[test]
fn mailbox_coalesces_only_consecutive_snapshots() {
    let mailbox = AiEventMailbox::default();
    mailbox.reset(8);

    assert!(mailbox.enqueue_worker(
        8,
        event(AiWorkerEventPayload::Snapshot(Box::new(snapshot("old"))))
    ));
    assert!(!mailbox.enqueue_worker(
        8,
        event(AiWorkerEventPayload::Snapshot(Box::new(snapshot("new"))))
    ));
    assert!(!mailbox.enqueue_worker(8, event(AiWorkerEventPayload::Status("ready".to_owned()))));
    assert!(!mailbox.enqueue_worker(
        8,
        event(AiWorkerEventPayload::Snapshot(Box::new(snapshot("latest"))))
    ));

    let events = mailbox.take(8);
    assert_eq!(events.len(), 3);
    let AiRuntimeEvent::Worker(event) = &events[0] else {
        panic!("expected the coalesced snapshot first");
    };
    let AiWorkerEventPayload::Snapshot(snapshot) = &event.payload else {
        panic!("expected the coalesced snapshot first");
    };
    assert_eq!(snapshot.active_thread_id.as_deref(), Some("new"));
}

#[test]
fn mailbox_rejects_stale_epochs_without_scheduling_qt_work() {
    let mailbox = AiEventMailbox::default();
    mailbox.reset(12);

    assert!(!mailbox.enqueue_worker(11, event(AiWorkerEventPayload::Status("stale".to_owned()))));
    assert!(mailbox.take(12).is_empty());
}

#[test]
fn resetting_the_mailbox_answers_queued_browser_calls() {
    let mailbox = AiEventMailbox::default();
    mailbox.reset(20);
    let (payload, response_rx) = browser_tool_call();

    assert!(mailbox.enqueue_worker(20, event(payload)));
    mailbox.reset(21);

    let response = response_rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("reset should answer the abandoned browser call");
    assert!(!response.success);
    assert!(mailbox.take(21).is_empty());
}

#[test]
fn stale_browser_calls_are_answered_without_scheduling_qt_work() {
    let mailbox = AiEventMailbox::default();
    mailbox.reset(31);
    let (payload, response_rx) = browser_tool_call();

    assert!(!mailbox.enqueue_worker(30, event(payload)));

    let response = response_rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("stale browser calls should receive a terminal response");
    assert!(!response.success);
    assert!(mailbox.take(31).is_empty());
}
