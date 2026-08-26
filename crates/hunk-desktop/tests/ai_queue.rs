use std::collections::BTreeSet;

use hunk_codex::state::{
    AiState, ItemStatus, ItemSummary, ThreadLifecycleStatus, ThreadSummary, TurnStatus, TurnSummary,
};
use hunk_desktop::{
    AI_MESSAGE_QUEUE_MAX_ITEMS, AI_MESSAGE_QUEUE_MAX_PROMPT_BYTES,
    AI_MESSAGE_QUEUE_MAX_RETAINED_BYTES, AiMessageQueue, AiQueueProjection,
};

fn thread(id: &str, status: ThreadLifecycleStatus, sequence: u64) -> ThreadSummary {
    ThreadSummary {
        id: id.to_owned(),
        cwd: "/repo".to_owned(),
        title: Some(id.to_owned()),
        status,
        created_at: 1,
        updated_at: 1,
        last_sequence: sequence,
    }
}

fn turn(thread_id: &str, status: TurnStatus, sequence: u64) -> TurnSummary {
    TurnSummary {
        id: format!("turn-{thread_id}"),
        thread_id: thread_id.to_owned(),
        collaboration_mode: None,
        status,
        last_sequence: sequence,
    }
}

fn user_message(thread_id: &str, content: &str, sequence: u64) -> ItemSummary {
    ItemSummary {
        id: format!("user-{sequence}"),
        thread_id: thread_id.to_owned(),
        turn_id: format!("turn-{thread_id}"),
        kind: "userMessage".to_owned(),
        status: ItemStatus::Completed,
        content: content.to_owned(),
        display_metadata: None,
        last_sequence: sequence,
    }
}

fn projection(state: &AiState, thread_ids: &[&str]) -> AiQueueProjection {
    AiQueueProjection::from_state(state, thread_ids)
}

#[test]
fn queued_follow_up_waits_for_idle_then_exact_authoritative_confirmation() {
    let mut state = AiState::default();
    state.threads.insert(
        "thread".to_owned(),
        thread("thread", ThreadLifecycleStatus::Active, 10),
    );
    state.turns.insert(
        "turn-thread".to_owned(),
        turn("thread", TurnStatus::InProgress, 11),
    );
    let mut queue = AiMessageQueue::default();
    queue
        .enqueue("thread".to_owned(), "Follow up\r\nafter this".to_owned())
        .unwrap();

    let running = projection(&state, &["thread"]);
    assert!(
        queue
            .ready_thread_ids(&running, &BTreeSet::new())
            .is_empty()
    );

    state.turns.get_mut("turn-thread").unwrap().status = TurnStatus::Completed;
    let idle = projection(&state, &["thread"]);
    assert_eq!(queue.ready_thread_ids(&idle, &BTreeSet::new()), ["thread"]);
    let command = queue.mark_next_pending("thread", 11).unwrap();
    assert_eq!(command.prompt, "Follow up\r\nafter this");

    state.items.insert(
        "mismatch".to_owned(),
        user_message("thread", "A different message", 12),
    );
    assert!(!queue.reconcile(&projection(&state, &["thread"])));
    assert_eq!(queue.total_count(), 1);

    state.items.insert(
        "accepted".to_owned(),
        user_message("thread", "Follow up\nafter this", 13),
    );
    assert!(queue.reconcile(&projection(&state, &["thread"])));
    assert_eq!(queue.total_count(), 0);
}

#[test]
fn queue_schedules_one_fifo_message_per_idle_thread() {
    let mut state = AiState::default();
    for thread_id in ["a", "b"] {
        state.threads.insert(
            thread_id.to_owned(),
            thread(thread_id, ThreadLifecycleStatus::Idle, 5),
        );
    }
    let projection = projection(&state, &["a", "b"]);
    let mut queue = AiMessageQueue::default();
    queue.enqueue("a".to_owned(), "a-1".to_owned()).unwrap();
    queue.enqueue("a".to_owned(), "a-2".to_owned()).unwrap();
    queue.enqueue("b".to_owned(), "b-1".to_owned()).unwrap();

    assert_eq!(
        queue.ready_thread_ids(&projection, &BTreeSet::new()),
        ["a", "b"]
    );
    assert_eq!(queue.mark_next_pending("a", 5).unwrap().prompt, "a-1");
    assert_eq!(queue.ready_thread_ids(&projection, &BTreeSet::new()), ["b"]);
    assert_eq!(
        queue.edit_latest_with_attachments("a").unwrap().prompt,
        "a-2"
    );
    assert_eq!(queue.thread_count("a"), 1);
}

#[test]
fn runtime_failure_requeues_an_unconfirmed_message_without_losing_it() {
    let mut state = AiState::default();
    state.threads.insert(
        "thread".to_owned(),
        thread("thread", ThreadLifecycleStatus::Idle, 4),
    );
    let projection = projection(&state, &["thread"]);
    let mut queue = AiMessageQueue::default();
    queue
        .enqueue("thread".to_owned(), "Retry me".to_owned())
        .unwrap();
    queue.mark_next_pending("thread", 4).unwrap();

    assert!(queue.thread_is_sending("thread"));
    assert!(queue.reset_pending_after_runtime_failure());
    assert!(!queue.thread_is_sending("thread"));
    assert_eq!(
        queue.ready_thread_ids(&projection, &BTreeSet::new()),
        ["thread"]
    );
}

#[test]
fn interrupt_and_unavailable_threads_recover_messages_into_their_drafts() {
    let mut state = AiState::default();
    state.threads.insert(
        "interrupted".to_owned(),
        thread("interrupted", ThreadLifecycleStatus::Active, 1),
    );
    state.threads.insert(
        "closed".to_owned(),
        thread("closed", ThreadLifecycleStatus::Closed, 1),
    );
    let mut queue = AiMessageQueue::default();
    queue
        .enqueue("interrupted".to_owned(), "first".to_owned())
        .unwrap();
    queue
        .enqueue("interrupted".to_owned(), "second".to_owned())
        .unwrap();
    queue
        .enqueue("closed".to_owned(), "closed draft".to_owned())
        .unwrap();
    queue.mark_interrupt_restore("interrupted".to_owned());

    assert!(queue.reconcile(&projection(&state, &["interrupted", "closed"])));
    assert_eq!(queue.total_count(), 0);
    assert_eq!(
        queue.take_recovered_draft("interrupted").prompt,
        "first\n\nsecond"
    );
    assert_eq!(queue.take_recovered_draft("closed").prompt, "closed draft");
    assert!(queue.take_recovered_draft("closed").prompt.is_empty());
}

#[test]
fn queue_preserves_a_thread_that_is_only_outside_the_visible_catalog() {
    let mut queue = AiMessageQueue::default();
    queue
        .enqueue("older-thread".to_owned(), "Keep this queued".to_owned())
        .unwrap();

    assert!(!queue.reconcile(&AiQueueProjection::default()));
    assert_eq!(queue.total_count(), 1);
    assert!(queue.take_recovered_draft("older-thread").prompt.is_empty());
}

#[test]
fn recovered_drafts_still_count_toward_the_queue_bound() {
    let mut state = AiState::default();
    state.threads.insert(
        "closed".to_owned(),
        thread("closed", ThreadLifecycleStatus::Closed, 1),
    );
    let mut queue = AiMessageQueue::default();
    for index in 0..AI_MESSAGE_QUEUE_MAX_ITEMS {
        queue
            .enqueue("closed".to_owned(), format!("message {index}"))
            .unwrap();
    }
    queue.reconcile(&projection(&state, &["closed"]));

    assert_eq!(queue.total_count(), 0);
    assert!(
        queue
            .enqueue("other".to_owned(), "overflow".to_owned())
            .is_err()
    );
    assert!(!queue.take_recovered_draft("closed").prompt.is_empty());
    assert!(
        queue
            .enqueue("other".to_owned(), "available again".to_owned())
            .is_ok()
    );
}

#[test]
fn queue_and_visible_projection_are_bounded() {
    let mut queue = AiMessageQueue::default();
    for index in 0..AI_MESSAGE_QUEUE_MAX_ITEMS {
        queue
            .enqueue("thread".to_owned(), format!("message {index}"))
            .unwrap();
    }
    assert!(
        queue
            .enqueue("thread".to_owned(), "overflow".to_owned())
            .is_err()
    );

    let long_prompt = "界".repeat(8_000);
    queue.edit_latest_with_attachments("thread");
    queue.enqueue("other".to_owned(), long_prompt).unwrap();
    let items = queue.timeline_items("other");
    assert_eq!(items.len(), 1);
    assert!(items[0].truncated);
    assert!(items[0].text.len() <= 16 * 1024);
    assert!(items[0].text.is_char_boundary(items[0].text.len()));
}

#[test]
fn queue_bounds_each_prompt_and_total_retained_bytes() {
    let mut queue = AiMessageQueue::default();
    assert!(
        queue
            .enqueue(
                "thread".to_owned(),
                "x".repeat(AI_MESSAGE_QUEUE_MAX_PROMPT_BYTES + 1),
            )
            .is_err()
    );

    let prompt = "x".repeat(AI_MESSAGE_QUEUE_MAX_PROMPT_BYTES - 2);
    let retained_prompt_count =
        AI_MESSAGE_QUEUE_MAX_RETAINED_BYTES / AI_MESSAGE_QUEUE_MAX_PROMPT_BYTES;
    for index in 0..retained_prompt_count {
        queue
            .enqueue(format!("thread-{index}"), prompt.clone())
            .unwrap();
    }
    assert!(
        queue
            .enqueue("overflow".to_owned(), "one more byte".to_owned())
            .is_err()
    );
}

#[test]
fn queued_image_follow_up_preserves_paths_and_matches_authoritative_content() {
    let mut state = AiState::default();
    state.threads.insert(
        "thread".to_owned(),
        thread("thread", ThreadLifecycleStatus::Idle, 5),
    );
    let image = std::path::PathBuf::from("/repo/screenshots/capture.png");
    let mut queue = AiMessageQueue::default();
    queue
        .enqueue_with_attachments(
            "thread".to_owned(),
            "Review this state".to_owned(),
            vec![image.clone()],
        )
        .unwrap();

    assert_eq!(
        queue.timeline_items("thread")[0].text,
        "Review this state\n[image] capture.png"
    );
    let command = queue.mark_next_pending("thread", 5).unwrap();
    assert_eq!(command.local_image_paths, [image]);

    state.items.insert(
        "accepted".to_owned(),
        user_message("thread", "Review this state\n[image] capture.png", 6),
    );
    assert!(queue.reconcile(&projection(&state, &["thread"])));
    assert_eq!(queue.total_count(), 0);
}

#[test]
fn recovered_image_only_follow_up_returns_its_attachments() {
    let mut state = AiState::default();
    state.threads.insert(
        "closed".to_owned(),
        thread("closed", ThreadLifecycleStatus::Closed, 1),
    );
    let image = std::path::PathBuf::from("/repo/screenshot.webp");
    let mut queue = AiMessageQueue::default();
    queue
        .enqueue_with_attachments("closed".to_owned(), String::new(), vec![image.clone()])
        .unwrap();

    assert!(queue.reconcile(&projection(&state, &["closed"])));
    let draft = queue.take_recovered_draft("closed");
    assert!(draft.prompt.is_empty());
    assert_eq!(draft.local_image_paths, [image]);
}

#[test]
fn queued_image_capability_check_tracks_only_the_next_sendable_message() {
    let image = std::path::PathBuf::from("/repo/screenshot.webp");
    let mut queue = AiMessageQueue::default();
    queue
        .enqueue_with_attachments("thread".to_owned(), String::new(), vec![image])
        .unwrap();

    assert!(queue.next_queued_has_attachments("thread"));
    queue.mark_next_pending("thread", 0).unwrap();
    assert!(!queue.next_queued_has_attachments("thread"));
}

#[test]
fn queued_image_capability_check_does_not_skip_a_text_only_head_message() {
    let image = std::path::PathBuf::from("/repo/screenshot.webp");
    let mut queue = AiMessageQueue::default();
    queue
        .enqueue("thread".to_owned(), "First".to_owned())
        .unwrap();
    queue
        .enqueue_with_attachments("thread".to_owned(), String::new(), vec![image])
        .unwrap();

    assert!(!queue.next_queued_has_attachments("thread"));
}
