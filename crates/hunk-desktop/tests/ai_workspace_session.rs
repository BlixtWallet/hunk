#[path = "../src/app/ai_workspace_session.rs"]
mod ai_workspace_session;

use std::sync::Arc;

use ai_workspace_session::{
    AI_WORKSPACE_SURFACE_BLOCK_BOTTOM_PADDING_PX, AI_WORKSPACE_SURFACE_BLOCK_GAP_PX,
    AI_WORKSPACE_SURFACE_BLOCK_SIDE_PADDING_PX, AI_WORKSPACE_SURFACE_BLOCK_TOP_PADDING_PX,
    AiWorkspaceBlock, AiWorkspaceBlockKind, AiWorkspaceBlockRole, AiWorkspaceSelection,
    AiWorkspaceSelectionRegion, AiWorkspaceSession, AiWorkspaceSourceRow,
};

fn block(id: &str, kind: AiWorkspaceBlockKind, preview: &str) -> AiWorkspaceBlock {
    AiWorkspaceBlock {
        id: id.to_string(),
        source_row_id: id.to_string(),
        role: match kind {
            AiWorkspaceBlockKind::Message | AiWorkspaceBlockKind::Plan => {
                AiWorkspaceBlockRole::Assistant
            }
            AiWorkspaceBlockKind::Group
            | AiWorkspaceBlockKind::DiffSummary
            | AiWorkspaceBlockKind::Tool => AiWorkspaceBlockRole::Tool,
            AiWorkspaceBlockKind::Status => AiWorkspaceBlockRole::System,
        },
        kind,
        title: id.to_string(),
        preview: preview.to_string(),
        last_sequence: 1,
    }
}

fn source_rows(entries: &[(&str, u64)]) -> Arc<[AiWorkspaceSourceRow]> {
    Arc::<[AiWorkspaceSourceRow]>::from(
        entries
            .iter()
            .map(|(row_id, last_sequence)| AiWorkspaceSourceRow {
                row_id: (*row_id).to_string(),
                last_sequence: *last_sequence,
            })
            .collect::<Vec<_>>(),
    )
}

#[test]
fn session_matches_source_thread_and_row_ids() {
    let session = AiWorkspaceSession::new(
        "thread-1",
        source_rows(&[("row-1", 1), ("row-2", 2)]),
        vec![block("row-1", AiWorkspaceBlockKind::Message, "preview")],
    );

    assert!(session.matches_source("thread-1", &source_rows(&[("row-1", 1), ("row-2", 2)])));
    assert!(!session.matches_source("thread-2", &source_rows(&[("row-1", 1), ("row-2", 2)])));
    assert!(!session.matches_source("thread-1", &source_rows(&[("row-1", 1)])));
    assert!(!session.matches_source("thread-1", &source_rows(&[("row-1", 1), ("row-2", 3)])));
}

#[test]
fn surface_snapshot_projects_visible_blocks_and_total_height() {
    let mut session = AiWorkspaceSession::new(
        "thread-1",
        source_rows(&[("row-1", 1), ("row-2", 2), ("row-3", 3)]),
        vec![
            block("row-1", AiWorkspaceBlockKind::Message, "first preview"),
            block("row-2", AiWorkspaceBlockKind::DiffSummary, "diff preview"),
            block("row-3", AiWorkspaceBlockKind::Status, ""),
        ],
    );

    let snapshot = session.surface_snapshot(0, 220, 640);

    assert_eq!(snapshot.viewport.visible_blocks.len(), 3);
    assert_eq!(
        snapshot
            .viewport
            .visible_blocks
            .first()
            .expect("first visible block")
            .top_px,
        AI_WORKSPACE_SURFACE_BLOCK_TOP_PADDING_PX
    );
    assert_eq!(
        snapshot
            .viewport
            .visible_blocks
            .get(1)
            .expect("second visible block")
            .top_px,
        AI_WORKSPACE_SURFACE_BLOCK_TOP_PADDING_PX + 72 + AI_WORKSPACE_SURFACE_BLOCK_GAP_PX
    );
    assert_eq!(
        snapshot.viewport.total_surface_height_px,
        AI_WORKSPACE_SURFACE_BLOCK_TOP_PADDING_PX
            + 72
            + AI_WORKSPACE_SURFACE_BLOCK_GAP_PX
            + 60
            + AI_WORKSPACE_SURFACE_BLOCK_GAP_PX
            + 56
            + AI_WORKSPACE_SURFACE_BLOCK_BOTTOM_PADDING_PX
    );
}

#[test]
fn surface_snapshot_limits_visible_blocks_to_requested_range() {
    let mut session = AiWorkspaceSession::new(
        "thread-1",
        source_rows(&[("row-1", 1), ("row-2", 2), ("row-3", 3)]),
        vec![
            block("row-1", AiWorkspaceBlockKind::Message, "first preview"),
            block("row-2", AiWorkspaceBlockKind::Message, "second preview"),
            block("row-3", AiWorkspaceBlockKind::Message, "third preview"),
        ],
    );

    let snapshot = session.surface_snapshot(96, 90, 640);
    let visible_ids = snapshot
        .viewport
        .visible_blocks
        .iter()
        .map(|entry| entry.block.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(visible_ids, vec!["row-2", "row-3"]);
}

#[test]
fn surface_snapshot_supports_all_block_kinds_and_roles() {
    let mut session = AiWorkspaceSession::new(
        "thread-1",
        source_rows(&[
            ("row-user", 1),
            ("row-group", 2),
            ("row-plan", 3),
            ("row-tool", 4),
            ("row-status", 5),
        ]),
        vec![
            AiWorkspaceBlock {
                id: "row-user".to_string(),
                source_row_id: "row-user".to_string(),
                role: AiWorkspaceBlockRole::User,
                kind: AiWorkspaceBlockKind::Message,
                title: "You".to_string(),
                preview: "prompt".to_string(),
                last_sequence: 1,
            },
            block("row-group", AiWorkspaceBlockKind::Group, "group"),
            block("row-plan", AiWorkspaceBlockKind::Plan, "plan"),
            block("row-tool", AiWorkspaceBlockKind::Tool, "tool"),
            block("row-status", AiWorkspaceBlockKind::Status, "status"),
        ],
    );

    let snapshot = session.surface_snapshot(0, 640, 800);
    let ids = snapshot
        .viewport
        .visible_blocks
        .iter()
        .map(|entry| entry.block.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        ids,
        vec![
            "row-user",
            "row-group",
            "row-plan",
            "row-tool",
            "row-status"
        ]
    );
}

#[test]
fn selection_matches_block_and_helpers_remain_addressable() {
    let selection = AiWorkspaceSelection {
        block_id: "row-2".to_string(),
        block_kind: AiWorkspaceBlockKind::DiffSummary,
        line_index: Some(1),
        region: AiWorkspaceSelectionRegion::Preview,
    };
    let session = AiWorkspaceSession::new(
        "thread-1",
        source_rows(&[("row-1", 1), ("row-2", 2)]),
        vec![
            block("row-1", AiWorkspaceBlockKind::Message, "first preview"),
            block("row-2", AiWorkspaceBlockKind::DiffSummary, "diff preview"),
        ],
    );

    assert!(selection.matches_block("row-2"));
    assert!(!selection.matches_block("row-1"));
    assert_eq!(
        AiWorkspaceSelectionRegion::Block,
        AiWorkspaceSelectionRegion::Block
    );
    assert_eq!(
        AiWorkspaceSelectionRegion::Title,
        AiWorkspaceSelectionRegion::Title
    );
    assert_eq!(session.block_count(), 2);
    assert_eq!(AI_WORKSPACE_SURFACE_BLOCK_SIDE_PADDING_PX, 16);
}
