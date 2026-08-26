use hunk_desktop::{AiPromptReceipt, AiTimelineProjection};

fn timeline(active_turn_id: &str, turn_count: i32) -> AiTimelineProjection {
    AiTimelineProjection {
        active_turn_id: active_turn_id.to_owned(),
        turn_running: !active_turn_id.is_empty(),
        total_turn_count: turn_count,
        ..AiTimelineProjection::default()
    }
}

#[test]
fn prompt_receipt_waits_for_authoritative_progress() {
    let receipt = AiPromptReceipt::new("thread".to_owned(), "turn".to_owned(), 10);

    assert!(!receipt.is_accepted_by("thread", &timeline("turn", 10)));
    assert!(!receipt.is_accepted_by("other", &timeline("next", 11)));
    assert!(!receipt.is_accepted_by("thread", &timeline("turn", 11)));
    assert!(receipt.is_accepted_by("thread", &timeline("next", 10)));
}

#[test]
fn idle_prompt_receipt_accepts_a_new_active_turn() {
    let receipt = AiPromptReceipt::new("thread".to_owned(), String::new(), 10);

    assert!(!receipt.is_accepted_by("thread", &timeline("", 10)));
    assert!(receipt.is_accepted_by("thread", &timeline("new-turn", 10)));
    assert!(receipt.is_accepted_by("thread", &timeline("", 11)));
    assert_eq!(receipt.thread_id(), "thread");
}
