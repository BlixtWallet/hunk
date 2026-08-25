use std::fs;

use hunk_app::ai::{find_rollout_path_for_thread, parse_rollout_fallback};
use tempfile::tempdir;

#[test]
fn parser_reconstructs_turns_and_messages_from_event_stream() {
    let input = r#"{"type":"event_msg","payload":{"type":"task_started","turn_id":"turn-1"}}
{"type":"event_msg","payload":{"type":"user_message","message":"hi"}}
{"type":"event_msg","payload":{"type":"agent_message","message":"hello"}}
{"type":"event_msg","payload":{"type":"task_complete","turn_id":"turn-1","last_agent_message":"hello"}}
{"type":"event_msg","payload":{"type":"task_started","turn_id":"turn-2"}}
{"type":"event_msg","payload":{"type":"user_message","message":"run tests"}}
{"type":"event_msg","payload":{"type":"agent_reasoning","text":"thinking"}}
{"type":"event_msg","payload":{"type":"agent_message","message":"done"}}
{"type":"event_msg","payload":{"type":"task_complete","turn_id":"turn-2","last_agent_message":"done"}}"#;
    let temp = tempdir().expect("temporary directory should be created");
    let path = temp.path().join("rollout.jsonl");
    fs::write(&path, input).expect("rollout should be written");

    let turns = parse_rollout_fallback(&path).expect("rollout should parse");

    assert_eq!(turns.len(), 2);
    assert_eq!(turns[0].turn_id, "turn-1");
    assert!(turns[0].completed);
    assert_eq!(turns[0].items.len(), 2);
    assert_eq!(turns[0].items[0].kind, "userMessage");
    assert_eq!(turns[0].items[0].content, "hi");
    assert_eq!(turns[0].items[1].kind, "agentMessage");
    assert_eq!(turns[0].items[1].content, "hello");

    assert_eq!(turns[1].turn_id, "turn-2");
    assert!(turns[1].completed);
    assert_eq!(turns[1].items.len(), 3);
    assert_eq!(turns[1].items[0].kind, "userMessage");
    assert_eq!(turns[1].items[1].kind, "reasoning");
    assert_eq!(turns[1].items[2].kind, "agentMessage");
}

#[test]
fn rollout_path_lookup_finds_thread_specific_jsonl() {
    let temp = tempdir().expect("temporary directory should be created");
    let sessions = temp.path().join("sessions/2026/03/04");
    fs::create_dir_all(&sessions).expect("sessions directories should be created");
    let target = sessions.join("rollout-2026-03-04T12-00-00-thread-abc.jsonl");
    fs::write(&target, b"").expect("rollout should be written");

    let resolved = find_rollout_path_for_thread(temp.path(), "thread-abc")
        .expect("rollout lookup should succeed");

    assert_eq!(resolved, Some(target));
}

#[test]
fn rollout_path_lookup_prefers_latest_rollout_for_same_thread() {
    let temp = tempdir().expect("temporary directory should be created");
    let sessions = temp.path().join("sessions/2026/03/04");
    fs::create_dir_all(&sessions).expect("sessions directories should be created");
    let older = sessions.join("rollout-2026-03-04T10-00-00-thread-xyz.jsonl");
    let newer = sessions.join("rollout-2026-03-04T12-00-00-thread-xyz.jsonl");
    fs::write(&older, b"").expect("older rollout should be written");
    fs::write(&newer, b"").expect("newer rollout should be written");

    let resolved = find_rollout_path_for_thread(temp.path(), "thread-xyz")
        .expect("rollout lookup should succeed");

    assert_eq!(resolved, Some(newer));
}
