use std::path::PathBuf;

use hunk_app::ai::{
    AiApprovalKind, AiPendingApproval, AiPendingUserInputQuestion,
    AiPendingUserInputQuestionOption, AiPendingUserInputRequest,
};
use hunk_qt::AiPendingRequestProjection;

fn approval(request_id: &str, thread_id: &str) -> AiPendingApproval {
    AiPendingApproval {
        request_id: request_id.to_owned(),
        thread_id: thread_id.to_owned(),
        turn_id: "turn".to_owned(),
        item_id: "item".to_owned(),
        kind: AiApprovalKind::CommandExecution,
        reason: Some("Needs network access".to_owned()),
        command: Some("cargo test".to_owned()),
        cwd: Some(PathBuf::from("/repo")),
        grant_root: None,
    }
}

fn user_input(request_id: &str, thread_id: &str) -> AiPendingUserInputRequest {
    AiPendingUserInputRequest {
        request_id: request_id.to_owned(),
        thread_id: thread_id.to_owned(),
        turn_id: "turn".to_owned(),
        item_id: "item".to_owned(),
        questions: vec![AiPendingUserInputQuestion {
            id: "choice".to_owned(),
            header: "Approach".to_owned(),
            question: "Which approach should I use?".to_owned(),
            is_other: false,
            is_secret: false,
            options: vec![
                AiPendingUserInputQuestionOption {
                    label: "Simple".to_owned(),
                    description: "Keep the change narrow.".to_owned(),
                },
                AiPendingUserInputQuestionOption {
                    label: "Broad".to_owned(),
                    description: "Include adjacent cleanup.".to_owned(),
                },
            ],
        }],
    }
}

#[test]
fn projection_prioritizes_selected_thread_approvals_and_marks_attention() {
    let projection = AiPendingRequestProjection::from_pending(
        Some("active"),
        &[
            approval("other-approval", "other"),
            approval("active-approval", "active"),
        ],
        &[user_input("active-input", "active")],
        &["active", "other"],
    );

    assert_eq!(projection.total_count, 3);
    assert_eq!(projection.active_count, 2);
    assert_eq!(
        projection.current.as_ref().unwrap().request_id,
        "active-approval"
    );
    assert_eq!(projection.current.as_ref().unwrap().kind, "approval");
    assert!(projection.attention_thread_ids().contains("active"));
    assert!(projection.attention_thread_ids().contains("other"));
    assert!(projection.request_is_pending("active-approval"));
    assert!(!projection.request_is_pending("missing"));
    assert!(projection.thread_needs_attention("active"));
    assert!(projection.thread_needs_attention("other"));
}

#[test]
fn projection_exposes_bounded_structured_questions_as_json() {
    let projection = AiPendingRequestProjection::from_pending(
        Some("active"),
        &[],
        &[user_input("input", "active")],
        &["active"],
    );
    let questions = serde_json::from_str::<serde_json::Value>(&projection.questions_json())
        .expect("questions JSON");

    assert_eq!(projection.current.as_ref().unwrap().kind, "userInput");
    assert_eq!(questions[0]["id"], "choice");
    assert_eq!(questions[0]["options"][0]["label"], "Simple");
}

#[test]
fn answer_validation_rejects_stale_unknown_and_invalid_options() {
    let projection = AiPendingRequestProjection::from_pending(
        Some("active"),
        &[],
        &[user_input("input", "active")],
        &["active"],
    );

    assert!(
        projection
            .validated_answers("input", r#"{"choice":["Simple"]}"#)
            .is_ok()
    );
    assert!(
        projection
            .validated_answers("stale", r#"{"choice":["Simple"]}"#)
            .is_err()
    );
    assert!(
        projection
            .validated_answers("input", r#"{"unknown":["Simple"]}"#)
            .is_err()
    );
    assert!(
        projection
            .validated_answers("input", r#"{"choice":["Missing"]}"#)
            .is_err()
    );
}

#[test]
fn projection_hides_other_thread_details_until_selected() {
    let projection = AiPendingRequestProjection::from_pending(
        Some("active"),
        &[approval("other-approval", "other")],
        &[],
        &["active", "other"],
    );

    assert_eq!(projection.total_count, 1);
    assert_eq!(projection.active_count, 0);
    assert!(projection.current.is_none());
    assert!(projection.attention_thread_ids().contains("other"));
}

#[test]
fn oversized_or_ambiguous_questions_cannot_be_submitted_partially() {
    let mut request = user_input("input", "active");
    request.questions = (0..9)
        .map(|index| AiPendingUserInputQuestion {
            id: format!("question-{index}"),
            header: "Question".to_owned(),
            question: "Choose safely.".to_owned(),
            is_other: false,
            is_secret: false,
            options: Vec::new(),
        })
        .collect();
    let projection =
        AiPendingRequestProjection::from_pending(Some("active"), &[], &[request], &["active"]);

    assert!(!projection.current.as_ref().unwrap().answerable);
    assert!(projection.validated_answers("input", "{}").is_err());

    let mut request = user_input("long-option", "active");
    request.questions[0].options[0].label = "x".repeat(513);
    let projection =
        AiPendingRequestProjection::from_pending(Some("active"), &[], &[request], &["active"]);

    assert!(!projection.current.as_ref().unwrap().answerable);
}

#[test]
fn attention_projection_is_bounded_and_preserves_the_active_thread() {
    let approvals = (0..240)
        .map(|index| {
            approval(
                format!("request-{index}").as_str(),
                format!("thread-{index}").as_str(),
            )
        })
        .collect::<Vec<_>>();
    let visible_thread_ids = (0..240)
        .map(|index| format!("thread-{index}"))
        .collect::<Vec<_>>();
    let visible_thread_ids = visible_thread_ids
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let projection = AiPendingRequestProjection::from_pending(
        Some("thread-239"),
        approvals.as_slice(),
        &[],
        visible_thread_ids.as_slice(),
    );

    assert_eq!(projection.attention_thread_ids().len(), 200);
    assert!(projection.attention_thread_ids().contains("thread-239"));
    assert!(projection.request_is_pending("request-239"));
    assert!(!projection.request_is_pending("request-200"));
}
