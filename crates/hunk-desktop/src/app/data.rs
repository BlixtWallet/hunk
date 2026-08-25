pub(super) use super::workspace_view::{WorkspaceSwitchAction, WorkspaceViewMode};
use super::*;

pub(super) use hunk_app::diff::{
    CachedStyledSegment, DiffRowSegmentCache, DiffSegmentQuality, DiffStream, DiffStreamRowKind,
    DiffStreamRowMeta, build_diff_row_segment_cache_from_cells, cached_runtime_fallback_segments,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RepoTreeRow {
    pub(super) path: String,
    pub(super) name: String,
    pub(super) file_status: Option<FileStatus>,
}

#[derive(Debug, Clone)]
pub(super) struct FileRowRange {
    pub(super) path: String,
    pub(super) status: FileStatus,
    pub(super) start_row: usize,
}

pub(super) fn build_changed_file_rows(files: &[ChangedFile]) -> Vec<RepoTreeRow> {
    files
        .iter()
        .map(|file| RepoTreeRow {
            path: file.path.clone(),
            name: file.path.clone(),
            file_status: Some(file.status),
        })
        .collect()
}

pub(super) fn line_number_column_width(digits: u32) -> f32 {
    digits as f32 * DIFF_MONO_CHAR_WIDTH + DIFF_LINE_NUMBER_EXTRA_PADDING
}
