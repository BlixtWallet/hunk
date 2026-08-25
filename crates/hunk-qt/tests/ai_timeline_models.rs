use hunk_codex::state::{
    AiState, ItemDisplayMetadata, ItemStatus, ItemSummary, TurnPlanStepStatus, TurnPlanStepSummary,
    TurnPlanSummary, TurnStatus, TurnSummary,
};
use hunk_qt::{AiTimelineItem, AiTimelineListModel, AiTimelineProjection};
use qtbridge::{QListModel, QObjectHolder};

fn turn(id: &str, sequence: u64) -> TurnSummary {
    TurnSummary {
        id: id.to_owned(),
        thread_id: "thread".to_owned(),
        collaboration_mode: None,
        status: TurnStatus::Completed,
        last_sequence: sequence,
    }
}

fn item(id: &str, turn_id: &str, kind: &str, content: &str, sequence: u64) -> ItemSummary {
    ItemSummary {
        id: id.to_owned(),
        thread_id: "thread".to_owned(),
        turn_id: turn_id.to_owned(),
        kind: kind.to_owned(),
        status: ItemStatus::Completed,
        content: content.to_owned(),
        display_metadata: None,
        last_sequence: sequence,
    }
}

fn projected_item(row_id: &str, kind: &str, text: &str) -> AiTimelineItem {
    AiTimelineItem {
        row_id: row_id.to_owned(),
        kind: kind.to_owned(),
        text: text.to_owned(),
        ..AiTimelineItem::default()
    }
}

#[test]
fn projection_orders_renderable_items_and_turn_plans() {
    let mut state = AiState::default();
    state.turns.insert("turn".to_owned(), turn("turn", 1));
    state.items.insert(
        "user".to_owned(),
        item("user", "turn", "userMessage", "  Explain this diff.  ", 2),
    );
    state.items.insert(
        "empty-reasoning".to_owned(),
        item("empty-reasoning", "turn", "reasoning", "  ", 3),
    );
    let mut reasoning = item("reasoning", "turn", "reasoning", "", 3);
    reasoning.display_metadata = Some(ItemDisplayMetadata {
        summary: Some("Considering the reducer boundary".to_owned()),
        details_json: Some("{\"phase\":\"analysis\"}".to_owned()),
    });
    state.items.insert("reasoning".to_owned(), reasoning);
    state.items.insert(
        "assistant".to_owned(),
        item(
            "assistant",
            "turn",
            "agentMessage",
            "It changes the parser.",
            4,
        ),
    );
    let mut command = item("command", "turn", "commandExecution", "cargo test", 5);
    command.status = ItemStatus::Streaming;
    command.display_metadata = Some(ItemDisplayMetadata {
        summary: Some("  Running focused tests  ".to_owned()),
        details_json: Some(
            r#"{"kind":"commandExecution","command":"cargo test","cwd":"/repo"}"#.to_owned(),
        ),
    });
    state.items.insert("command".to_owned(), command);
    state.turn_plans.insert(
        "turn".to_owned(),
        TurnPlanSummary {
            thread_id: "thread".to_owned(),
            turn_id: "turn".to_owned(),
            explanation: Some("Verify the change".to_owned()),
            steps: vec![
                TurnPlanStepSummary {
                    step: "Read the parser".to_owned(),
                    status: TurnPlanStepStatus::Completed,
                },
                TurnPlanStepSummary {
                    step: "Run the tests".to_owned(),
                    status: TurnPlanStepStatus::InProgress,
                },
            ],
            created_sequence: 6,
            last_sequence: 6,
        },
    );

    let projection = AiTimelineProjection::from_state(&state, Some("thread"));

    assert_eq!(projection.total_turn_count, 1);
    assert!(projection.active_turn_id.is_empty());
    assert!(!projection.turn_running);
    assert_eq!(projection.visible_turn_count, 1);
    assert_eq!(projection.hidden_turn_count, 0);
    assert_eq!(projection.total_row_count, 5);
    assert_eq!(projection.hidden_row_count, 0);
    assert_eq!(projection.items[0].row_id, "item:user");
    assert_eq!(projection.items[0].role, "user");
    assert_eq!(projection.items[0].title, "You");
    assert_eq!(projection.items[0].text, "Explain this diff.");
    assert_eq!(
        projection.items[1].title,
        "Considering the reducer boundary"
    );
    assert_eq!(projection.items[1].text, "{\"phase\":\"analysis\"}");
    assert_eq!(projection.items[2].role, "assistant");
    assert_eq!(projection.items[3].title, "Running focused tests");
    assert_eq!(projection.items[3].status, "streaming");
    assert!(projection.items[3].streaming);
    assert!(projection.items[3].mono);
    assert_eq!(projection.items[3].command, "cargo test");
    assert_eq!(projection.items[3].cwd, "/repo");
    assert_eq!(projection.items[4].kind, "turnPlan");
    assert_eq!(projection.items[4].status, "in progress");
    assert_eq!(
        projection.items[4].text,
        "Verify the change\n[x] Read the parser\n[~] Run the tests"
    );
}

#[test]
fn projection_does_not_offer_truncated_commands_for_execution() {
    let mut state = AiState::default();
    state.turns.insert("turn".to_owned(), turn("turn", 1));
    let mut command = item("command", "turn", "commandExecution", "output", 2);
    command.display_metadata = Some(ItemDisplayMetadata {
        summary: Some("Long command".to_owned()),
        details_json: Some(
            serde_json::json!({
                "kind": "commandExecution",
                "command": "x".repeat(2 * 1024 + 3),
                "cwd": "/repo",
            })
            .to_string(),
        ),
    });
    state.items.insert("command".to_owned(), command);

    let projection = AiTimelineProjection::from_state(&state, Some("thread"));

    assert!(projection.items[0].command.is_empty());
    assert!(projection.items[0].cwd.is_empty());
}

#[test]
fn projection_bounds_turns_rows_and_utf8_content() {
    let mut state = AiState::default();
    for index in 0..81 {
        let turn_id = format!("turn-{index:03}");
        state
            .turns
            .insert(turn_id.clone(), turn(turn_id.as_str(), index));
        state.items.insert(
            format!("message-{index:03}"),
            item(
                format!("message-{index:03}").as_str(),
                turn_id.as_str(),
                "agentMessage",
                "visible turn",
                index,
            ),
        );
    }
    for index in 0..1_001 {
        let id = format!("extra-{index:04}");
        let content = if index == 1_000 {
            "é".repeat(9_000)
        } else {
            "bounded row".to_owned()
        };
        state.items.insert(
            id.clone(),
            item(
                id.as_str(),
                "turn-080",
                "agentMessage",
                content.as_str(),
                1_000 + index,
            ),
        );
    }

    let projection = AiTimelineProjection::from_state(&state, Some("thread"));

    assert_eq!(projection.total_turn_count, 81);
    assert_eq!(projection.visible_turn_count, 80);
    assert_eq!(projection.hidden_turn_count, 1);
    assert_eq!(projection.total_row_count, 1_081);
    assert_eq!(projection.hidden_row_count, 81);
    assert_eq!(projection.items.len(), 1_000);
    let last = projection.items.last().expect("last bounded row");
    assert!(last.truncated);
    assert!(last.text.ends_with('…'));
    assert!(last.text.len() <= 16 * 1024);
    assert!(last.text.is_char_boundary(last.text.len()));
}

#[test]
fn projection_is_empty_without_an_active_thread() {
    let mut state = AiState::default();
    state.turns.insert("turn".to_owned(), turn("turn", 1));
    state.items.insert(
        "message".to_owned(),
        item("message", "turn", "agentMessage", "Not selected", 2),
    );

    assert_eq!(
        AiTimelineProjection::from_state(&state, None),
        AiTimelineProjection::default()
    );
}

#[test]
fn projection_identifies_the_latest_running_turn() {
    let mut state = AiState::default();
    state.turns.insert("older".to_owned(), turn("older", 4));
    let mut running = turn("running", 9);
    running.status = TurnStatus::InProgress;
    state.turns.insert("running".to_owned(), running);

    let projection = AiTimelineProjection::from_state(&state, Some("thread"));

    assert_eq!(projection.active_turn_id, "running");
    assert!(projection.turn_running);
}

#[test]
fn timeline_model_updates_stable_rows_without_a_reset() {
    let model = AiTimelineListModel::default_with_attached_qobject();
    let mut model = model.borrow_mut();
    let first = AiTimelineItem {
        row_id: "item:message".to_owned(),
        text: "Hel".to_owned(),
        streaming: true,
        ..AiTimelineItem::default()
    };

    assert!(model.sync(vec![first.clone()]));
    assert_eq!(model.len(), 1);
    assert!(!model.sync(vec![first]));

    let completed = AiTimelineItem {
        row_id: "item:message".to_owned(),
        text: "Hello".to_owned(),
        streaming: false,
        ..AiTimelineItem::default()
    };
    assert!(model.sync(vec![completed]));
    assert_eq!(model.get(0).expect("updated row").text, "Hello");

    let replacement = AiTimelineItem {
        row_id: "item:next".to_owned(),
        text: "Next".to_owned(),
        ..AiTimelineItem::default()
    };
    assert!(model.sync(vec![replacement]));
    assert_eq!(model.get(0).expect("replacement row").row_id, "item:next");
}

#[test]
fn timeline_model_reconciles_the_queue_suffix_without_replacing_authoritative_rows() {
    let model = AiTimelineListModel::default_with_attached_qobject();
    let mut model = model.borrow_mut();
    let authoritative = projected_item("item:message", "agentMessage", "Done");
    let first = projected_item("queued-message:1", "queuedMessage", "First");

    assert!(model.sync(vec![authoritative.clone(), first.clone()]));
    assert_eq!(model.sync_queue_items(vec![first.clone()]), (false, 0));

    let mut sending = first;
    sending.status = "sending".to_owned();
    sending.streaming = true;
    let second = projected_item("queued-message:2", "queuedMessage", "Second");
    assert_eq!(
        model.sync_queue_items(vec![sending, second.clone()]),
        (true, 0)
    );
    assert_eq!(model.len(), 3);
    assert_eq!(model.get(0), Some(&authoritative));
    assert_eq!(model.get(1).expect("sending row").status, "sending");

    assert_eq!(model.sync_queue_items(vec![second.clone()]), (true, 0));
    assert_eq!(model.len(), 2);
    assert_eq!(model.get(0), Some(&authoritative));
    assert_eq!(model.get(1), Some(&second));
}

#[test]
fn timeline_model_keeps_the_combined_authoritative_and_queue_rows_bounded() {
    let model = AiTimelineListModel::default_with_attached_qobject();
    let mut model = model.borrow_mut();
    let authoritative = (0..1_000)
        .map(|index| projected_item(format!("item:{index}").as_str(), "agentMessage", "row"))
        .collect::<Vec<_>>();
    assert!(model.sync(authoritative));

    let queue = vec![
        projected_item("queued-message:1", "queuedMessage", "First"),
        projected_item("queued-message:2", "queuedMessage", "Second"),
    ];
    assert_eq!(model.sync_queue_items(queue), (true, 2));
    assert_eq!(model.len(), 1_000);
    assert_eq!(model.get(0).expect("first visible row").row_id, "item:2");
    assert_eq!(
        model.get(999).expect("last visible row").row_id,
        "queued-message:2"
    );
}
