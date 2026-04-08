use std::ops::Range;

use hunk_domain::diff::{DiffCellKind, DiffRowKind};

use crate::app::data::{
    CachedStyledSegment, DiffSegmentQuality, DiffStreamRowKind,
    build_diff_row_segment_cache_from_cells, cached_runtime_fallback_segments,
};
use crate::app::review_workspace_session::{
    ReviewWorkspaceSession, ReviewWorkspaceVisibleFileHeader,
};

pub(crate) const AI_INLINE_REVIEW_CODE_LINE_HEIGHT_PX: usize = 26;
pub(crate) const AI_INLINE_REVIEW_FILE_HEADER_HEIGHT_PX: usize = 34;
pub(crate) const AI_INLINE_REVIEW_META_ROW_HEIGHT_PX: usize = 24;
pub(crate) const AI_INLINE_REVIEW_OVERSCAN_ROWS: usize = 8;

const AI_INLINE_REVIEW_SYNC_SEGMENT_CACHE_LIMIT: usize = 48;
const AI_INLINE_REVIEW_DETAILED_MAX_CHANGED_LINES: u64 = 8_000;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AiInlineReviewDisplayGeometry {
    row_boundaries_px: Vec<usize>,
}

impl AiInlineReviewDisplayGeometry {
    pub(crate) fn build(session: &ReviewWorkspaceSession) -> Self {
        let mut row_boundaries_px = Vec::with_capacity(session.row_count().saturating_add(1));
        row_boundaries_px.push(0);
        let mut cursor = 0usize;
        for row_ix in 0..session.row_count() {
            cursor = cursor.saturating_add(ai_inline_review_row_height_px(session, row_ix));
            row_boundaries_px.push(cursor);
        }
        Self { row_boundaries_px }
    }

    pub(crate) fn total_surface_height_px(&self) -> usize {
        self.row_boundaries_px.last().copied().unwrap_or_default()
    }

    pub(crate) fn row_count(&self) -> usize {
        self.row_boundaries_px.len().saturating_sub(1)
    }

    pub(crate) fn row_top_offset_px(&self, row_ix: usize) -> Option<usize> {
        self.row_boundaries_px.get(row_ix).copied()
    }

    pub(crate) fn row_bottom_offset_px(&self, row_ix: usize) -> Option<usize> {
        self.row_boundaries_px
            .get(row_ix.saturating_add(1))
            .copied()
    }

    pub(crate) fn visible_row_range_for_viewport(
        &self,
        scroll_top_px: usize,
        viewport_height_px: usize,
    ) -> Option<Range<usize>> {
        let row_count = self.row_count();
        if row_count == 0 {
            return None;
        }

        let viewport_bottom = scroll_top_px
            .saturating_add(viewport_height_px.max(AI_INLINE_REVIEW_CODE_LINE_HEIGHT_PX));
        let start = self
            .row_boundaries_px
            .partition_point(|boundary| *boundary <= scroll_top_px)
            .saturating_sub(1)
            .min(row_count.saturating_sub(1));
        let end = self
            .row_boundaries_px
            .partition_point(|boundary| *boundary < viewport_bottom)
            .max(start.saturating_add(1))
            .min(row_count);

        Some(start..end)
    }

    pub(crate) fn render_row_range_for_viewport(
        &self,
        scroll_top_px: usize,
        viewport_height_px: usize,
        overscan_rows: usize,
    ) -> Option<Range<usize>> {
        let visible = self.visible_row_range_for_viewport(scroll_top_px, viewport_height_px)?;
        let row_count = self.row_count();
        let start = visible.start.saturating_sub(overscan_rows);
        let end = visible.end.saturating_add(overscan_rows).min(row_count);
        Some(start..end.max(start.saturating_add(1)).min(row_count))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AiInlineReviewCodeLine {
    pub(crate) kind: DiffCellKind,
    pub(crate) old_line: Option<u32>,
    pub(crate) new_line: Option<u32>,
    pub(crate) text: String,
    pub(crate) segments: Vec<CachedStyledSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AiInlineReviewViewportRowKind {
    FileHeader {
        header: ReviewWorkspaceVisibleFileHeader,
    },
    Meta {
        row_kind: DiffRowKind,
        text: String,
    },
    Code {
        lines: Vec<AiInlineReviewCodeLine>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AiInlineReviewViewportRow {
    pub(crate) row_index: usize,
    pub(crate) stable_id: u64,
    pub(crate) file_path: Option<String>,
    pub(crate) surface_top_px: usize,
    pub(crate) height_px: usize,
    pub(crate) kind: AiInlineReviewViewportRowKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AiInlineReviewViewportSnapshot {
    pub(crate) total_surface_height_px: usize,
    pub(crate) visible_pixel_range: Option<Range<usize>>,
    pub(crate) rows: Vec<AiInlineReviewViewportRow>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AiInlineReviewSurfaceSnapshot {
    pub(crate) scroll_top_px: usize,
    pub(crate) viewport_height_px: usize,
    pub(crate) visible_row_range: Option<Range<usize>>,
    pub(crate) viewport: AiInlineReviewViewportSnapshot,
    pub(crate) sticky_file_header: Option<ReviewWorkspaceVisibleFileHeader>,
}

pub(crate) fn ensure_ai_inline_review_visible_row_caches(
    session: &mut ReviewWorkspaceSession,
    row_range: Range<usize>,
) -> bool {
    let mut inserted = false;
    let mut seeded = 0usize;

    for row_ix in row_range {
        if seeded >= AI_INLINE_REVIEW_SYNC_SEGMENT_CACHE_LIMIT {
            break;
        }
        if session.row_segment_cache(row_ix).is_some() {
            continue;
        }
        let Some(row) = session.row(row_ix) else {
            continue;
        };
        if row.kind != DiffRowKind::Code {
            continue;
        }

        let file_path = session.row_file_path(row_ix);
        let line_stats = session
            .visible_file_header_at_surface_row(row_ix)
            .map(|header| header.line_stats)
            .unwrap_or_default();
        let quality = if line_stats.changed() <= AI_INLINE_REVIEW_DETAILED_MAX_CHANGED_LINES {
            DiffSegmentQuality::Detailed
        } else {
            DiffSegmentQuality::SyntaxOnly
        };
        let cache = build_diff_row_segment_cache_from_cells(
            file_path,
            row.left.text.as_str(),
            row.left.kind,
            row.right.text.as_str(),
            row.right.kind,
            quality,
        );
        if session.set_row_segment_cache_if_better(row_ix, cache) {
            inserted = true;
        }
        seeded = seeded.saturating_add(1);
    }

    inserted
}

pub(crate) fn build_ai_inline_review_surface_snapshot(
    geometry: &AiInlineReviewDisplayGeometry,
    session: &ReviewWorkspaceSession,
    scroll_top_px: usize,
    viewport_height_px: usize,
    overscan_rows: usize,
) -> AiInlineReviewSurfaceSnapshot {
    let visible_row_range =
        geometry.visible_row_range_for_viewport(scroll_top_px, viewport_height_px);
    let render_row_range =
        geometry.render_row_range_for_viewport(scroll_top_px, viewport_height_px, overscan_rows);

    let rows = render_row_range
        .clone()
        .map(|row_range| {
            row_range
                .filter_map(|row_ix| build_ai_inline_review_viewport_row(geometry, session, row_ix))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let visible_pixel_range = render_row_range.as_ref().and_then(|row_range| {
        let start = geometry.row_top_offset_px(row_range.start)?;
        let end = geometry
            .row_bottom_offset_px(row_range.end.saturating_sub(1))
            .unwrap_or(start);
        Some(start..end)
    });
    let sticky_file_header = visible_row_range.as_ref().and_then(|row_range| {
        let header = session.visible_file_header_at_surface_row(row_range.start)?;
        (header.row_index != row_range.start).then_some(header)
    });

    AiInlineReviewSurfaceSnapshot {
        scroll_top_px,
        viewport_height_px,
        visible_row_range,
        viewport: AiInlineReviewViewportSnapshot {
            total_surface_height_px: geometry.total_surface_height_px(),
            visible_pixel_range,
            rows,
        },
        sticky_file_header,
    }
}

fn build_ai_inline_review_viewport_row(
    geometry: &AiInlineReviewDisplayGeometry,
    session: &ReviewWorkspaceSession,
    row_ix: usize,
) -> Option<AiInlineReviewViewportRow> {
    let row = session.row(row_ix)?;
    let row_metadata = session.row_metadata(row_ix);
    let file_path = session.row_file_path(row_ix).map(ToString::to_string);
    let surface_top_px = geometry.row_top_offset_px(row_ix)?;
    let height_px = ai_inline_review_row_height_px(session, row_ix);
    let stable_id = row_metadata
        .map(|meta| meta.stable_id)
        .unwrap_or(row_ix as u64);

    let kind = if row_metadata.is_some_and(|meta| meta.kind == DiffStreamRowKind::FileHeader) {
        AiInlineReviewViewportRowKind::FileHeader {
            header: session.visible_file_header_at_surface_row(row_ix)?,
        }
    } else {
        match row.kind {
            DiffRowKind::Code => AiInlineReviewViewportRowKind::Code {
                lines: ai_inline_review_code_lines(session, row_ix),
            },
            DiffRowKind::HunkHeader | DiffRowKind::Meta | DiffRowKind::Empty => {
                AiInlineReviewViewportRowKind::Meta {
                    row_kind: row.kind,
                    text: row.text.clone(),
                }
            }
        }
    };

    Some(AiInlineReviewViewportRow {
        row_index: row_ix,
        stable_id,
        file_path,
        surface_top_px,
        height_px,
        kind,
    })
}

fn ai_inline_review_row_height_px(session: &ReviewWorkspaceSession, row_ix: usize) -> usize {
    let Some(row) = session.row(row_ix) else {
        return AI_INLINE_REVIEW_CODE_LINE_HEIGHT_PX;
    };
    if session
        .row_metadata(row_ix)
        .is_some_and(|meta| meta.kind == DiffStreamRowKind::FileHeader)
    {
        return AI_INLINE_REVIEW_FILE_HEADER_HEIGHT_PX;
    }

    match row.kind {
        DiffRowKind::Code => ai_inline_review_code_lines(session, row_ix)
            .len()
            .max(1)
            .saturating_mul(AI_INLINE_REVIEW_CODE_LINE_HEIGHT_PX),
        DiffRowKind::HunkHeader | DiffRowKind::Meta | DiffRowKind::Empty => {
            AI_INLINE_REVIEW_META_ROW_HEIGHT_PX
        }
    }
}

fn ai_inline_review_code_lines(
    session: &ReviewWorkspaceSession,
    row_ix: usize,
) -> Vec<AiInlineReviewCodeLine> {
    let Some(row) = session.row(row_ix) else {
        return Vec::new();
    };
    let row_cache = session.row_segment_cache(row_ix);

    let left_line = ai_inline_review_code_line(
        row.left.kind,
        row.left.line,
        row.right.line,
        row.left.text.as_str(),
        row_cache.map(|cache| cache.left.as_slice()),
        false,
    );
    let right_line = ai_inline_review_code_line(
        row.right.kind,
        row.left.line,
        row.right.line,
        row.right.text.as_str(),
        row_cache.map(|cache| cache.right.as_slice()),
        true,
    );

    match (left_line, right_line) {
        (Some(left), Some(right))
            if left.kind == DiffCellKind::Removed && right.kind == DiffCellKind::Added =>
        {
            vec![left, right]
        }
        (Some(left), Some(_right)) if left.kind == DiffCellKind::Removed => vec![left],
        (Some(left), Some(right)) if left.kind == DiffCellKind::Context => {
            vec![AiInlineReviewCodeLine {
                kind: DiffCellKind::Context,
                old_line: left.old_line,
                new_line: right.new_line,
                text: if !right.text.is_empty() {
                    right.text
                } else {
                    left.text
                },
                segments: if !right.segments.is_empty() {
                    right.segments
                } else {
                    left.segments
                },
            }]
        }
        (Some(left), None) => vec![left],
        (None, Some(right)) => vec![right],
        (Some(left), Some(_right)) => vec![left],
        (None, None) => vec![AiInlineReviewCodeLine {
            kind: DiffCellKind::Context,
            old_line: row.left.line,
            new_line: row.right.line,
            text: String::new(),
            segments: Vec::new(),
        }],
    }
}

fn ai_inline_review_code_line(
    cell_kind: DiffCellKind,
    old_line: Option<u32>,
    new_line: Option<u32>,
    text: &str,
    cached_segments: Option<&[CachedStyledSegment]>,
    right_side: bool,
) -> Option<AiInlineReviewCodeLine> {
    match cell_kind {
        DiffCellKind::Added => Some(AiInlineReviewCodeLine {
            kind: DiffCellKind::Added,
            old_line: None,
            new_line,
            text: text.to_string(),
            segments: ai_inline_review_segments(cached_segments, text),
        }),
        DiffCellKind::Removed => Some(AiInlineReviewCodeLine {
            kind: DiffCellKind::Removed,
            old_line,
            new_line: None,
            text: text.to_string(),
            segments: ai_inline_review_segments(cached_segments, text),
        }),
        DiffCellKind::Context => Some(AiInlineReviewCodeLine {
            kind: DiffCellKind::Context,
            old_line,
            new_line,
            text: text.to_string(),
            segments: ai_inline_review_segments(cached_segments, text),
        }),
        DiffCellKind::None => (!text.is_empty() || right_side).then(|| AiInlineReviewCodeLine {
            kind: DiffCellKind::Context,
            old_line,
            new_line,
            text: text.to_string(),
            segments: ai_inline_review_segments(cached_segments, text),
        }),
    }
}

fn ai_inline_review_segments(
    cached_segments: Option<&[CachedStyledSegment]>,
    text: &str,
) -> Vec<CachedStyledSegment> {
    cached_segments
        .map(|segments| segments.to_vec())
        .unwrap_or_else(|| cached_runtime_fallback_segments(text))
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use hunk_git::compare::CompareSnapshot;
    use hunk_git::git::{ChangedFile, FileStatus, LineStats};

    use crate::app::data::build_diff_stream_from_patch_map;

    use super::*;

    fn test_session(patch: &str) -> ReviewWorkspaceSession {
        let files = vec![ChangedFile {
            path: "src/main.rs".to_string(),
            status: FileStatus::Modified,
            staged: false,
            unstaged: true,
            untracked: false,
        }];
        let file_line_stats = [(
            "src/main.rs".to_string(),
            LineStats {
                added: 1,
                removed: 1,
            },
        )]
        .into_iter()
        .collect::<BTreeMap<_, _>>();
        let snapshot = CompareSnapshot {
            files: files.clone(),
            file_line_stats: file_line_stats.clone(),
            overall_line_stats: LineStats {
                added: 1,
                removed: 1,
            },
            patches_by_path: [("src/main.rs".to_string(), patch.to_string())]
                .into_iter()
                .collect::<BTreeMap<_, _>>(),
        };
        let stream = build_diff_stream_from_patch_map(
            &files,
            &BTreeSet::new(),
            &file_line_stats,
            &snapshot.patches_by_path,
            &BTreeSet::new(),
        );
        ReviewWorkspaceSession::from_compare_snapshot(&snapshot, &BTreeSet::new())
            .expect("session")
            .with_render_stream(&stream)
    }

    #[test]
    fn modified_code_rows_expand_to_removed_then_added_lines() {
        let mut session = test_session(
            "diff --git a/src/main.rs b/src/main.rs\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1 +1 @@\n-old\n+new\n",
        );
        let geometry = AiInlineReviewDisplayGeometry::build(&session);
        let visible = geometry
            .visible_row_range_for_viewport(0, 240)
            .expect("visible range");
        assert!(ensure_ai_inline_review_visible_row_caches(
            &mut session,
            visible.clone()
        ));

        let snapshot = build_ai_inline_review_surface_snapshot(
            &geometry,
            &session,
            0,
            240,
            AI_INLINE_REVIEW_OVERSCAN_ROWS,
        );
        let code_row = snapshot
            .viewport
            .rows
            .iter()
            .find_map(|row| match &row.kind {
                AiInlineReviewViewportRowKind::Code { lines } if lines.len() == 2 => Some(lines),
                _ => None,
            })
            .expect("modified code row");

        assert_eq!(code_row[0].kind, DiffCellKind::Removed);
        assert_eq!(code_row[0].text, "old");
        assert_eq!(code_row[1].kind, DiffCellKind::Added);
        assert_eq!(code_row[1].text, "new");
    }

    #[test]
    fn context_code_rows_render_once_with_both_line_numbers() {
        let mut session = test_session(
            "diff --git a/src/main.rs b/src/main.rs\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,2 +1,2 @@\n unchanged\n-old\n+new\n",
        );
        let geometry = AiInlineReviewDisplayGeometry::build(&session);
        let visible = geometry
            .visible_row_range_for_viewport(0, 320)
            .expect("visible range");
        ensure_ai_inline_review_visible_row_caches(&mut session, visible.clone());

        let snapshot = build_ai_inline_review_surface_snapshot(
            &geometry,
            &session,
            0,
            320,
            AI_INLINE_REVIEW_OVERSCAN_ROWS,
        );
        let context_line = snapshot
            .viewport
            .rows
            .iter()
            .find_map(|row| match &row.kind {
                AiInlineReviewViewportRowKind::Code { lines }
                    if lines.len() == 1 && lines[0].kind == DiffCellKind::Context =>
                {
                    Some(&lines[0])
                }
                _ => None,
            })
            .expect("context line");

        assert_eq!(context_line.text.trim(), "unchanged");
        assert!(context_line.old_line.is_some());
        assert!(context_line.new_line.is_some());
    }
}
