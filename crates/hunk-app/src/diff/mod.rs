mod highlight;
mod segments;
mod stream;
mod turn_patch;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use anyhow::Result;
use hunk_git::compare::{CompareSnapshot, CompareSource, load_compare_snapshot};
use hunk_git::git::LineStats;

pub use highlight::{
    StyledSegment, SyntaxTokenKind, build_line_segments, build_plain_line_segments,
    build_syntax_only_line_segments,
};
pub use segments::{
    CachedStyledSegment, DiffRowSegmentCache, DiffSegmentQuality,
    build_diff_row_segment_cache_from_cells, cached_runtime_fallback_segments,
    compact_cached_segments_for_render, is_binary_patch, is_probably_binary_extension,
};
pub use stream::{
    DiffStream, DiffStreamRowKind, DiffStreamRowMeta, build_diff_stream_from_patch_map,
};
pub use turn_patch::compare_snapshot_from_turn_diff;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffCommand {
    Compare {
        primary_repo_root: PathBuf,
        left: CompareSource,
        right: CompareSource,
    },
    HistoricalTurn {
        patch: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffProjectionOptions {
    pub collapsed_paths: BTreeSet<String>,
    pub previous_file_line_stats: BTreeMap<String, LineStats>,
    pub loading_paths: BTreeSet<String>,
}

#[derive(Debug, Clone)]
pub struct DiffSnapshot {
    pub comparison: CompareSnapshot,
    pub projection: DiffStream,
}

pub fn load_diff_snapshot(
    command: DiffCommand,
    options: DiffProjectionOptions,
) -> Result<DiffSnapshot> {
    let comparison = match command {
        DiffCommand::Compare {
            primary_repo_root,
            left,
            right,
        } => load_compare_snapshot(primary_repo_root.as_path(), &left, &right)?,
        DiffCommand::HistoricalTurn { patch } => compare_snapshot_from_turn_diff(patch.as_str()),
    };
    let projection = build_diff_stream_from_patch_map(
        &comparison.files,
        &options.collapsed_paths,
        &options.previous_file_line_stats,
        &comparison.patches_by_path,
        &options.loading_paths,
    );

    Ok(DiffSnapshot {
        comparison,
        projection,
    })
}
