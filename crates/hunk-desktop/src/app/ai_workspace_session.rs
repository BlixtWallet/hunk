use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(crate) const AI_WORKSPACE_SURFACE_BLOCK_GAP_PX: usize = 12;
pub(crate) const AI_WORKSPACE_SURFACE_BLOCK_SIDE_PADDING_PX: usize = 16;
pub(crate) const AI_WORKSPACE_SURFACE_BLOCK_TOP_PADDING_PX: usize = 16;
pub(crate) const AI_WORKSPACE_SURFACE_BLOCK_BOTTOM_PADDING_PX: usize = 16;
pub(crate) const AI_WORKSPACE_BLOCK_CONTENT_TOP_PADDING_PX: usize = 10;
pub(crate) const AI_WORKSPACE_BLOCK_CONTENT_BOTTOM_PADDING_PX: usize = 10;
pub(crate) const AI_WORKSPACE_BLOCK_TEXT_SIDE_PADDING_PX: usize = 14;
pub(crate) const AI_WORKSPACE_BLOCK_SECTION_GAP_PX: usize = 8;
pub(crate) const AI_WORKSPACE_BLOCK_MIN_WIDTH_PX: usize = 200;
pub(crate) const AI_WORKSPACE_BLOCK_TITLE_LINE_HEIGHT_PX: usize = 16;
pub(crate) const AI_WORKSPACE_BLOCK_PREVIEW_LINE_HEIGHT_PX: usize = 18;

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
    pub(crate) expandable: bool,
    pub(crate) expanded: bool,
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
    pub(crate) fn bottom_px(&self) -> usize {
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
    pub(crate) text_layout: AiWorkspaceBlockTextLayout,
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AiWorkspaceBlockTextLayout {
    pub(crate) block_width_px: usize,
    pub(crate) title_lines: Vec<String>,
    pub(crate) preview_lines: Vec<String>,
    pub(crate) height_px: usize,
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

    pub(crate) fn block(&self, block_id: &str) -> Option<&AiWorkspaceBlock> {
        self.blocks.iter().find(|block| block.id == block_id)
    }

    pub(crate) fn block_at(&self, index: usize) -> Option<&AiWorkspaceBlock> {
        self.blocks.get(index)
    }

    pub(crate) fn block_index(&self, block_id: &str) -> Option<usize> {
        self.blocks.iter().position(|block| block.id == block_id)
    }

    pub(crate) fn block_geometry(
        &mut self,
        block_id: &str,
        width_px: usize,
    ) -> Option<AiWorkspaceBlockGeometry> {
        let width_bucket = ai_workspace_width_bucket(width_px);
        let geometry = self
            .geometry_by_width_bucket
            .entry(width_bucket)
            .or_insert_with(|| build_ai_workspace_geometry(self.blocks.as_slice(), width_bucket));
        geometry
            .blocks
            .iter()
            .find(|entry| entry.block_id == block_id)
            .cloned()
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
            .or_insert_with(|| build_ai_workspace_geometry(self.blocks.as_slice(), width_bucket))
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
                    .map(|block| {
                        let text_layout = ai_workspace_text_layout_for_block(&block, width_bucket);
                        debug_assert_eq!(text_layout.height_px, entry.height_px);
                        AiWorkspaceViewportBlock {
                            block,
                            top_px: entry.top_px,
                            height_px: entry.height_px,
                            text_layout,
                        }
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

pub(crate) fn ai_workspace_text_layout_for_block(
    block: &AiWorkspaceBlock,
    surface_width_px: usize,
) -> AiWorkspaceBlockTextLayout {
    let block_width_px = ai_workspace_block_width_px(surface_width_px, block.role);
    let text_width_px = ai_workspace_block_text_width_px(block_width_px);
    let title_lines = ai_workspace_wrap_text(
        block.title.as_str(),
        ai_workspace_chars_per_line(text_width_px, true, false),
        2,
    );
    let preview_lines = ai_workspace_wrap_text(
        block.preview.as_str(),
        ai_workspace_chars_per_line(
            text_width_px,
            false,
            block.kind == AiWorkspaceBlockKind::DiffSummary,
        ),
        ai_workspace_preview_line_limit(block),
    );
    let title_height_px = title_lines.len() * AI_WORKSPACE_BLOCK_TITLE_LINE_HEIGHT_PX;
    let preview_height_px = preview_lines.len() * AI_WORKSPACE_BLOCK_PREVIEW_LINE_HEIGHT_PX;
    let preview_gap_px = if preview_lines.is_empty() {
        0
    } else {
        AI_WORKSPACE_BLOCK_SECTION_GAP_PX
    };

    AiWorkspaceBlockTextLayout {
        block_width_px,
        title_lines,
        preview_lines,
        height_px: AI_WORKSPACE_BLOCK_CONTENT_TOP_PADDING_PX
            + title_height_px
            + preview_gap_px
            + preview_height_px
            + AI_WORKSPACE_BLOCK_CONTENT_BOTTOM_PADDING_PX,
    }
}

fn build_ai_workspace_geometry(
    blocks: &[AiWorkspaceBlock],
    surface_width_px: usize,
) -> AiWorkspaceDisplayGeometry {
    let mut top_px = AI_WORKSPACE_SURFACE_BLOCK_TOP_PADDING_PX;
    let mut geometry_blocks = Vec::with_capacity(blocks.len());

    for block in blocks {
        let height_px = ai_workspace_text_layout_for_block(block, surface_width_px).height_px;
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

fn ai_workspace_block_width_px(surface_width_px: usize, role: AiWorkspaceBlockRole) -> usize {
    let available_width_px = surface_width_px
        .saturating_sub(AI_WORKSPACE_SURFACE_BLOCK_SIDE_PADDING_PX * 2)
        .max(180);
    let desired_width_px = match role {
        AiWorkspaceBlockRole::User => ((available_width_px as f32) * 0.72).round() as usize,
        AiWorkspaceBlockRole::Assistant => available_width_px.min(620),
        AiWorkspaceBlockRole::Tool => available_width_px.min(700),
        AiWorkspaceBlockRole::System => available_width_px.min(640),
    };
    desired_width_px
        .clamp(AI_WORKSPACE_BLOCK_MIN_WIDTH_PX, available_width_px)
        .max(AI_WORKSPACE_BLOCK_MIN_WIDTH_PX.min(available_width_px))
}

pub(crate) fn ai_workspace_block_text_width_px(block_width_px: usize) -> usize {
    block_width_px
        .saturating_sub(AI_WORKSPACE_BLOCK_TEXT_SIDE_PADDING_PX * 2)
        .max(120)
}

pub(crate) fn ai_workspace_chars_per_line(
    text_width_px: usize,
    title: bool,
    monospace: bool,
) -> usize {
    let char_width_px = if monospace {
        7.2
    } else if title {
        7.0
    } else {
        6.6
    };
    ((text_width_px as f32) / char_width_px).floor() as usize
}

fn ai_workspace_preview_line_limit(block: &AiWorkspaceBlock) -> usize {
    match block.kind {
        AiWorkspaceBlockKind::Message => 96,
        AiWorkspaceBlockKind::Plan => 32,
        AiWorkspaceBlockKind::DiffSummary => 5,
        AiWorkspaceBlockKind::Group => 4,
        AiWorkspaceBlockKind::Tool | AiWorkspaceBlockKind::Status => {
            if block.expanded {
                48
            } else {
                4
            }
        }
    }
}

fn ai_workspace_width_bucket(width_px: usize) -> usize {
    const AI_WORKSPACE_WIDTH_BUCKET_SIZE_PX: usize = 40;

    let clamped = width_px.max(AI_WORKSPACE_WIDTH_BUCKET_SIZE_PX);
    (clamped / AI_WORKSPACE_WIDTH_BUCKET_SIZE_PX) * AI_WORKSPACE_WIDTH_BUCKET_SIZE_PX
}

fn ai_workspace_wrap_text(text: &str, max_chars_per_line: usize, max_lines: usize) -> Vec<String> {
    if max_lines == 0 {
        return Vec::new();
    }

    let max_chars_per_line = max_chars_per_line.max(8);
    let mut lines = Vec::new();

    let mut raw_lines = text.lines().peekable();
    while let Some(raw_line) = raw_lines.next() {
        let has_more_input = raw_lines.peek().is_some();
        if raw_line.is_empty() {
            lines.push(String::new());
            if lines.len() == max_lines {
                if has_more_input {
                    ai_workspace_append_ellipsis(lines.last_mut());
                }
                return lines;
            }
            continue;
        }

        let mut remaining = raw_line.trim_end_matches(['\r', ' ']);
        loop {
            if remaining.is_empty() {
                break;
            }

            let remaining_chars = remaining.chars().count();
            if remaining_chars <= max_chars_per_line {
                lines.push(remaining.to_string());
                if lines.len() == max_lines {
                    if has_more_input {
                        ai_workspace_append_ellipsis(lines.last_mut());
                    }
                    return lines;
                }
                break;
            }

            let split_index = ai_workspace_wrap_split_index(remaining, max_chars_per_line)
                .unwrap_or(remaining.len());
            let (chunk, rest) = remaining.split_at(split_index);
            let chunk = chunk.trim_end_matches([' ', '\t']);
            lines.push(if chunk.is_empty() {
                remaining[..split_index].to_string()
            } else {
                chunk.to_string()
            });
            if lines.len() == max_lines {
                ai_workspace_append_ellipsis(lines.last_mut());
                return lines;
            }
            remaining = rest.trim_start_matches([' ', '\t']);
        }
    }

    lines
}

fn ai_workspace_wrap_split_index(text: &str, max_chars_per_line: usize) -> Option<usize> {
    let mut char_count = 0usize;
    let mut last_whitespace_break = None;

    for (byte_index, ch) in text.char_indices() {
        char_count = char_count.saturating_add(1);
        if ch.is_whitespace() {
            last_whitespace_break = Some(byte_index + ch.len_utf8());
        }
        if char_count >= max_chars_per_line {
            return last_whitespace_break.or(Some(byte_index + ch.len_utf8()));
        }
    }

    None
}

fn ai_workspace_append_ellipsis(line: Option<&mut String>) {
    let Some(line) = line else {
        return;
    };
    if !line.ends_with("...") {
        line.push_str("...");
    }
}
