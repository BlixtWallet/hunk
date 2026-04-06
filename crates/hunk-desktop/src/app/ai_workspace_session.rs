use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(crate) const AI_WORKSPACE_SURFACE_BLOCK_GAP_PX: usize = 12;
pub(crate) const AI_WORKSPACE_SURFACE_BLOCK_SIDE_PADDING_PX: usize = 16;
pub(crate) const AI_WORKSPACE_SURFACE_BLOCK_TOP_PADDING_PX: usize = 16;
pub(crate) const AI_WORKSPACE_SURFACE_BLOCK_BOTTOM_PADDING_PX: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AiWorkspaceBlockRole {
    User,
    Assistant,
    Tool,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AiWorkspaceBlockKind {
    Message,
    Group,
    DiffSummary,
    Plan,
    Tool,
    Status,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AiWorkspaceBlock {
    pub(crate) id: String,
    pub(crate) source_row_id: String,
    pub(crate) role: AiWorkspaceBlockRole,
    pub(crate) kind: AiWorkspaceBlockKind,
    pub(crate) title: String,
    pub(crate) preview: String,
    pub(crate) last_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AiWorkspaceSourceRow {
    pub(crate) row_id: String,
    pub(crate) last_sequence: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AiWorkspaceSelectionRegion {
    Block,
    Title,
    Preview,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AiWorkspaceSelection {
    pub(crate) block_id: String,
    pub(crate) block_kind: AiWorkspaceBlockKind,
    pub(crate) line_index: Option<usize>,
    pub(crate) region: AiWorkspaceSelectionRegion,
}

impl AiWorkspaceSelection {
    pub(crate) fn matches_block(&self, block_id: &str) -> bool {
        self.block_id == block_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AiWorkspaceBlockGeometry {
    pub(crate) block_id: String,
    pub(crate) top_px: usize,
    pub(crate) height_px: usize,
}

impl AiWorkspaceBlockGeometry {
    fn bottom_px(&self) -> usize {
        self.top_px.saturating_add(self.height_px)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AiWorkspaceDisplayGeometry {
    pub(crate) total_surface_height_px: usize,
    pub(crate) blocks: Vec<AiWorkspaceBlockGeometry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AiWorkspaceViewportBlock {
    pub(crate) block: AiWorkspaceBlock,
    pub(crate) top_px: usize,
    pub(crate) height_px: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AiWorkspaceViewportSnapshot {
    pub(crate) total_surface_height_px: usize,
    pub(crate) visible_pixel_range: Option<Range<usize>>,
    pub(crate) visible_blocks: Vec<AiWorkspaceViewportBlock>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AiWorkspaceSurfaceSnapshot {
    pub(crate) scroll_top_px: usize,
    pub(crate) viewport_height_px: usize,
    pub(crate) viewport: AiWorkspaceViewportSnapshot,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AiWorkspaceSurfaceSnapshotResult {
    pub(crate) snapshot: AiWorkspaceSurfaceSnapshot,
    pub(crate) geometry_rebuild_duration: Option<Duration>,
}

#[derive(Debug, Clone)]
pub(crate) struct AiWorkspaceSession {
    thread_id: String,
    source_rows: Arc<[AiWorkspaceSourceRow]>,
    blocks: Vec<AiWorkspaceBlock>,
    geometry_by_width_bucket: BTreeMap<usize, AiWorkspaceDisplayGeometry>,
}

impl AiWorkspaceSession {
    pub(crate) fn new(
        thread_id: impl Into<String>,
        source_rows: Arc<[AiWorkspaceSourceRow]>,
        blocks: Vec<AiWorkspaceBlock>,
    ) -> Self {
        Self {
            thread_id: thread_id.into(),
            source_rows,
            blocks,
            geometry_by_width_bucket: BTreeMap::new(),
        }
    }

    pub(crate) fn matches_source(
        &self,
        thread_id: &str,
        source_rows: &[AiWorkspaceSourceRow],
    ) -> bool {
        self.thread_id == thread_id && self.source_rows.as_ref() == source_rows
    }

    pub(crate) fn block_count(&self) -> usize {
        self.blocks.len()
    }

    pub(crate) fn surface_snapshot(
        &mut self,
        scroll_top_px: usize,
        viewport_height_px: usize,
        width_px: usize,
    ) -> AiWorkspaceSurfaceSnapshot {
        self.surface_snapshot_with_stats(scroll_top_px, viewport_height_px, width_px)
            .snapshot
    }

    pub(crate) fn surface_snapshot_with_stats(
        &mut self,
        scroll_top_px: usize,
        viewport_height_px: usize,
        width_px: usize,
    ) -> AiWorkspaceSurfaceSnapshotResult {
        let width_bucket = ai_workspace_width_bucket(width_px);
        let geometry_rebuild_started_at =
            (!self.geometry_by_width_bucket.contains_key(&width_bucket)).then(Instant::now);
        let geometry = self
            .geometry_by_width_bucket
            .entry(width_bucket)
            .or_insert_with(|| build_ai_workspace_geometry(self.blocks.as_slice()))
            .clone();
        let geometry_rebuild_duration =
            geometry_rebuild_started_at.map(|started_at| started_at.elapsed());
        let viewport_end_px = scroll_top_px.saturating_add(viewport_height_px);
        let visible_blocks = geometry
            .blocks
            .iter()
            .filter_map(|entry| {
                if entry.bottom_px() <= scroll_top_px || entry.top_px >= viewport_end_px {
                    return None;
                }

                self.blocks
                    .iter()
                    .find(|block| block.id == entry.block_id)
                    .cloned()
                    .map(|block| AiWorkspaceViewportBlock {
                        block,
                        top_px: entry.top_px,
                        height_px: entry.height_px,
                    })
            })
            .collect::<Vec<_>>();

        AiWorkspaceSurfaceSnapshotResult {
            snapshot: AiWorkspaceSurfaceSnapshot {
                scroll_top_px,
                viewport_height_px,
                viewport: AiWorkspaceViewportSnapshot {
                    total_surface_height_px: geometry.total_surface_height_px,
                    visible_pixel_range: (!visible_blocks.is_empty()).then_some(
                        scroll_top_px..viewport_end_px.min(geometry.total_surface_height_px),
                    ),
                    visible_blocks,
                },
            },
            geometry_rebuild_duration,
        }
    }
}

fn build_ai_workspace_geometry(blocks: &[AiWorkspaceBlock]) -> AiWorkspaceDisplayGeometry {
    let mut top_px = AI_WORKSPACE_SURFACE_BLOCK_TOP_PADDING_PX;
    let mut geometry_blocks = Vec::with_capacity(blocks.len());

    for block in blocks {
        let height_px = ai_workspace_block_height_px(block);
        geometry_blocks.push(AiWorkspaceBlockGeometry {
            block_id: block.id.clone(),
            top_px,
            height_px,
        });
        top_px = top_px
            .saturating_add(height_px)
            .saturating_add(AI_WORKSPACE_SURFACE_BLOCK_GAP_PX);
    }

    let total_surface_height_px = if geometry_blocks.is_empty() {
        AI_WORKSPACE_SURFACE_BLOCK_TOP_PADDING_PX + AI_WORKSPACE_SURFACE_BLOCK_BOTTOM_PADDING_PX
    } else {
        top_px
            .saturating_sub(AI_WORKSPACE_SURFACE_BLOCK_GAP_PX)
            .saturating_add(AI_WORKSPACE_SURFACE_BLOCK_BOTTOM_PADDING_PX)
    };

    AiWorkspaceDisplayGeometry {
        total_surface_height_px,
        blocks: geometry_blocks,
    }
}

fn ai_workspace_block_height_px(block: &AiWorkspaceBlock) -> usize {
    let has_preview = !block.preview.trim().is_empty();
    match (block.kind, has_preview) {
        (AiWorkspaceBlockKind::DiffSummary, _) => 60,
        (AiWorkspaceBlockKind::Group, true) => 68,
        (_, true) => 72,
        _ => 56,
    }
}

fn ai_workspace_width_bucket(width_px: usize) -> usize {
    let clamped = width_px.max(1);
    clamped.div_ceil(160) * 160
}
