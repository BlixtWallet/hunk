use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use hunk_domain::diff::{
    DiffCell, DiffCellKind, DiffRowKind, SideBySideRow, parse_patch_side_by_side,
};
use hunk_git::git::{ChangedFile, FileStatus, LineStats};

use super::segments::{DiffRowSegmentCache, is_binary_patch, is_probably_binary_extension};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffStreamRowKind {
    FileHeader,
    CoreCode,
    CoreHunkHeader,
    CoreMeta,
    CoreEmpty,
    FileLoading,
    FileCollapsed,
    FileError,
    EmptyState,
}

#[derive(Debug, Clone)]
pub struct DiffStreamRowMeta {
    pub stable_id: u64,
    pub file_path: Option<String>,
    pub file_status: Option<FileStatus>,
    pub kind: DiffStreamRowKind,
}

#[derive(Debug, Clone)]
pub struct DiffStream {
    pub rows: Vec<SideBySideRow>,
    pub row_metadata: Vec<DiffStreamRowMeta>,
    pub row_segments: Vec<Option<DiffRowSegmentCache>>,
}

struct LoadedFileDiffRows {
    core_rows: Vec<SideBySideRow>,
    load_error: Option<String>,
}

pub fn build_diff_stream_from_patch_map(
    files: &[ChangedFile],
    collapsed_files: &BTreeSet<String>,
    previous_file_line_stats: &BTreeMap<String, LineStats>,
    patches_by_path: &BTreeMap<String, String>,
    loading_paths: &BTreeSet<String>,
) -> DiffStream {
    let mut rows = Vec::new();
    let mut row_metadata = Vec::new();
    let mut row_segments = Vec::new();

    for file in files {
        let mut file_row_ordinal = 0_usize;
        push_stream_row(
            &mut rows,
            &mut row_metadata,
            message_row(DiffRowKind::Meta, file.path.clone()),
            DiffStreamRowKind::FileHeader,
            Some(file.path.as_str()),
            Some(file.status),
            file_row_ordinal,
        );
        row_segments.push(None);
        file_row_ordinal = file_row_ordinal.saturating_add(1);

        if collapsed_files.contains(file.path.as_str()) {
            let collapsed_stats = previous_file_line_stats
                .get(file.path.as_str())
                .copied()
                .unwrap_or_default();
            let collapsed_message = if collapsed_stats.changed() > 0 {
                format!(
                    "File collapsed ({} changed lines hidden, counts may be stale). Expand to refresh.",
                    collapsed_stats.changed()
                )
            } else {
                "File collapsed. Expand to load and refresh its diff.".to_string()
            };
            push_stream_row(
                &mut rows,
                &mut row_metadata,
                message_row(DiffRowKind::Empty, collapsed_message),
                DiffStreamRowKind::FileCollapsed,
                Some(file.path.as_str()),
                Some(file.status),
                file_row_ordinal,
            );
            row_segments.push(None);
        } else if loading_paths.contains(file.path.as_str()) {
            push_stream_row(
                &mut rows,
                &mut row_metadata,
                message_row(DiffRowKind::Meta, "Loading file diff..."),
                DiffStreamRowKind::FileLoading,
                Some(file.path.as_str()),
                Some(file.status),
                file_row_ordinal,
            );
            row_segments.push(None);
        } else {
            let patch = patches_by_path
                .get(file.path.as_str())
                .map(String::as_str)
                .unwrap_or_default();
            let loaded_file = load_file_diff_rows(file, patch);
            if let Some(load_error) = loaded_file.load_error {
                push_stream_row(
                    &mut rows,
                    &mut row_metadata,
                    message_row(DiffRowKind::Meta, load_error),
                    DiffStreamRowKind::FileError,
                    Some(file.path.as_str()),
                    Some(file.status),
                    file_row_ordinal,
                );
                row_segments.push(None);
            } else {
                for row in loaded_file.core_rows.into_iter().filter(|row| {
                    matches!(
                        row.kind,
                        DiffRowKind::Code | DiffRowKind::HunkHeader | DiffRowKind::Empty
                    )
                }) {
                    let row_kind = stream_kind_for_core_row(&row);
                    push_stream_row(
                        &mut rows,
                        &mut row_metadata,
                        row,
                        row_kind,
                        Some(file.path.as_str()),
                        Some(file.status),
                        file_row_ordinal,
                    );
                    row_segments.push(None);
                    file_row_ordinal = file_row_ordinal.saturating_add(1);
                }
            }
        }
    }

    if rows.is_empty() {
        push_stream_row(
            &mut rows,
            &mut row_metadata,
            message_row(DiffRowKind::Empty, "No changed files."),
            DiffStreamRowKind::EmptyState,
            None,
            None,
            0,
        );
        row_segments.push(None);
    }

    debug_assert_eq!(row_segments.len(), rows.len());
    DiffStream {
        rows,
        row_metadata,
        row_segments,
    }
}

fn message_row(kind: DiffRowKind, text: impl Into<String>) -> SideBySideRow {
    SideBySideRow {
        kind,
        left: DiffCell {
            line: None,
            text: String::new(),
            kind: DiffCellKind::None,
        },
        right: DiffCell {
            line: None,
            text: String::new(),
            kind: DiffCellKind::None,
        },
        text: text.into(),
    }
}

fn load_file_diff_rows(file: &ChangedFile, patch: &str) -> LoadedFileDiffRows {
    if is_probably_binary_extension(file.path.as_str()) {
        return LoadedFileDiffRows {
            core_rows: Vec::new(),
            load_error: Some(format!(
                "Preview unavailable for {}: binary file type.",
                file.path
            )),
        };
    }
    if is_binary_patch(patch) {
        return LoadedFileDiffRows {
            core_rows: Vec::new(),
            load_error: Some(format!(
                "Preview unavailable for {}: binary diff.",
                file.path
            )),
        };
    }

    LoadedFileDiffRows {
        core_rows: parse_patch_side_by_side(patch),
        load_error: None,
    }
}

fn stream_kind_for_core_row(row: &SideBySideRow) -> DiffStreamRowKind {
    match row.kind {
        DiffRowKind::Code => DiffStreamRowKind::CoreCode,
        DiffRowKind::HunkHeader => DiffStreamRowKind::CoreHunkHeader,
        DiffRowKind::Meta => DiffStreamRowKind::CoreMeta,
        DiffRowKind::Empty => DiffStreamRowKind::CoreEmpty,
    }
}

fn push_stream_row(
    rows: &mut Vec<SideBySideRow>,
    row_metadata: &mut Vec<DiffStreamRowMeta>,
    row: SideBySideRow,
    kind: DiffStreamRowKind,
    file_path: Option<&str>,
    file_status: Option<FileStatus>,
    ordinal: usize,
) -> u64 {
    let stable_id = compute_stable_row_id(file_path, kind, ordinal);
    rows.push(row);
    row_metadata.push(DiffStreamRowMeta {
        stable_id,
        file_path: file_path.map(ToString::to_string),
        file_status,
        kind,
    });
    stable_id
}

fn compute_stable_row_id(file_path: Option<&str>, kind: DiffStreamRowKind, ordinal: usize) -> u64 {
    let mut hasher = DefaultHasher::new();
    file_path.unwrap_or("__stream__").hash(&mut hasher);
    stable_kind_tag(kind).hash(&mut hasher);
    ordinal.hash(&mut hasher);
    hasher.finish()
}

fn stable_kind_tag(kind: DiffStreamRowKind) -> &'static str {
    match kind {
        DiffStreamRowKind::FileHeader => "file-header",
        DiffStreamRowKind::CoreCode => "core-code",
        DiffStreamRowKind::CoreHunkHeader => "core-hunk-header",
        DiffStreamRowKind::CoreMeta => "core-meta",
        DiffStreamRowKind::CoreEmpty => "core-empty",
        DiffStreamRowKind::FileLoading => "file-loading",
        DiffStreamRowKind::FileCollapsed => "file-collapsed",
        DiffStreamRowKind::FileError => "file-error",
        DiffStreamRowKind::EmptyState => "empty-state",
    }
}
