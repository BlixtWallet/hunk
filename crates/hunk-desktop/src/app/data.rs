use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

pub(super) use super::data_segments::{
    cached_runtime_fallback_segments, compact_cached_segments_for_render, is_binary_patch,
    is_probably_binary_extension,
};
use super::highlight::{
    StyledSegment, SyntaxTokenKind, build_line_segments, build_syntax_only_line_segments,
};
pub(super) use super::workspace_view::{WorkspaceSwitchAction, WorkspaceViewMode};
use super::*;
use hunk_domain::diff::parse_patch_side_by_side;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RepoTreeRow {
    pub(super) path: String,
    pub(super) name: String,
    pub(super) file_status: Option<FileStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CachedStyledSegment {
    pub(super) plain_text: SharedString,
    pub(super) syntax: SyntaxTokenKind,
    pub(super) changed: bool,
    pub(super) search_match: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct DiffRowSegmentCache {
    pub(super) quality: DiffSegmentQuality,
    pub(super) left: Vec<CachedStyledSegment>,
    pub(super) right: Vec<CachedStyledSegment>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum DiffSegmentQuality {
    #[default]
    Plain,
    SyntaxOnly,
    Detailed,
}

#[derive(Debug, Clone)]
pub(super) struct FileRowRange {
    pub(super) path: String,
    pub(super) status: FileStatus,
    pub(super) start_row: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DiffStreamRowKind {
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
pub(super) struct DiffStreamRowMeta {
    pub(super) stable_id: u64,
    pub(super) file_path: Option<String>,
    pub(super) file_status: Option<FileStatus>,
    pub(super) kind: DiffStreamRowKind,
}

pub(super) struct DiffStream {
    pub(super) rows: Vec<SideBySideRow>,
    pub(super) row_metadata: Vec<DiffStreamRowMeta>,
    pub(super) row_segments: Vec<Option<DiffRowSegmentCache>>,
}

struct LoadedFileDiffRows {
    core_rows: Vec<SideBySideRow>,
    load_error: Option<String>,
}

const MAX_RENDER_SEGMENTS_PER_CELL_DETAILED: usize = 48;
const MAX_RENDER_SEGMENTS_PER_CELL_LARGE_FILE: usize = 24;

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

pub(super) fn message_row(kind: DiffRowKind, text: impl Into<String>) -> SideBySideRow {
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

pub(super) fn build_diff_stream_from_patch_map(
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

    let core_rows = parse_patch_side_by_side(patch);
    LoadedFileDiffRows {
        core_rows,
        load_error: None,
    }
}

pub(super) fn cached_segments_from_styled(
    segments: Vec<StyledSegment>,
) -> Vec<CachedStyledSegment> {
    segments
        .into_iter()
        .map(|segment| CachedStyledSegment {
            plain_text: SharedString::from(segment.text),
            syntax: segment.syntax,
            changed: segment.changed,
            search_match: false,
        })
        .collect::<Vec<_>>()
}

pub(super) fn build_diff_row_segment_cache_from_cells(
    file_path: Option<&str>,
    left_text: &str,
    left_kind: DiffCellKind,
    right_text: &str,
    right_kind: DiffCellKind,
    quality: DiffSegmentQuality,
) -> DiffRowSegmentCache {
    match quality {
        DiffSegmentQuality::Detailed => {
            let left = compact_cached_segments_for_render(
                cached_segments_from_styled(build_line_segments(
                    file_path, left_text, left_kind, right_text, right_kind,
                )),
                MAX_RENDER_SEGMENTS_PER_CELL_DETAILED,
            );
            let right = compact_cached_segments_for_render(
                cached_segments_from_styled(build_line_segments(
                    file_path, right_text, right_kind, left_text, left_kind,
                )),
                MAX_RENDER_SEGMENTS_PER_CELL_DETAILED,
            );

            DiffRowSegmentCache {
                quality,
                left,
                right,
            }
        }
        DiffSegmentQuality::SyntaxOnly => {
            let left = compact_cached_segments_for_render(
                cached_segments_from_styled(build_syntax_only_line_segments(file_path, left_text)),
                MAX_RENDER_SEGMENTS_PER_CELL_LARGE_FILE,
            );
            let right = compact_cached_segments_for_render(
                cached_segments_from_styled(build_syntax_only_line_segments(file_path, right_text)),
                MAX_RENDER_SEGMENTS_PER_CELL_LARGE_FILE,
            );

            DiffRowSegmentCache {
                quality,
                left,
                right,
            }
        }
        DiffSegmentQuality::Plain => DiffRowSegmentCache {
            quality,
            left: cached_runtime_fallback_segments(left_text),
            right: cached_runtime_fallback_segments(right_text),
        },
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

pub(super) fn line_number_column_width(digits: u32) -> f32 {
    digits as f32 * DIFF_MONO_CHAR_WIDTH + DIFF_LINE_NUMBER_EXTRA_PADDING
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_row_id_is_deterministic_for_same_row() {
        let first = compute_stable_row_id(Some("src/lib.rs"), DiffStreamRowKind::CoreCode, 2);
        let second = compute_stable_row_id(Some("src/lib.rs"), DiffStreamRowKind::CoreCode, 2);

        assert_eq!(first, second);
    }

    #[test]
    fn stable_row_id_changes_when_ordinal_changes() {
        let first = compute_stable_row_id(Some("src/lib.rs"), DiffStreamRowKind::CoreMeta, 0);
        let second = compute_stable_row_id(Some("src/lib.rs"), DiffStreamRowKind::CoreMeta, 1);

        assert_ne!(first, second);
    }

    #[test]
    fn compact_cached_segments_caps_count_and_preserves_text() {
        let mut expected = String::new();
        let styled = (0..20)
            .map(|ix| {
                let text = format!("part-{ix}|");
                expected.push_str(&text);
                StyledSegment {
                    text,
                    syntax: if ix % 2 == 0 {
                        SyntaxTokenKind::Keyword
                    } else {
                        SyntaxTokenKind::String
                    },
                    changed: ix % 3 == 0,
                }
            })
            .collect::<Vec<_>>();
        let cached = cached_segments_from_styled(styled);

        let compacted = compact_cached_segments_for_render(cached, 6);
        assert!(compacted.len() <= 6);

        let reconstructed = compacted.iter().fold(String::new(), |mut acc, segment| {
            acc.push_str(segment.plain_text.as_ref());
            acc
        });
        assert_eq!(reconstructed, expected);
    }

    #[test]
    fn large_file_segment_mode_keeps_syntax_without_changed_pair_lcs() {
        let row = SideBySideRow {
            kind: DiffRowKind::Code,
            left: DiffCell {
                line: Some(1),
                text: "name = \"hunk\" # app".to_string(),
                kind: DiffCellKind::Added,
            },
            right: DiffCell {
                line: Some(1),
                text: "name = \"hunk\" # app".to_string(),
                kind: DiffCellKind::Added,
            },
            text: String::new(),
        };

        let cache = build_diff_row_segment_cache_from_cells(
            Some("Cargo.toml"),
            &row.left.text,
            row.left.kind,
            &row.right.text,
            row.right.kind,
            DiffSegmentQuality::SyntaxOnly,
        );
        assert!(
            cache
                .left
                .iter()
                .any(|segment| segment.syntax != SyntaxTokenKind::Plain),
            "expected syntax-only large-file mode to keep non-plain tokens"
        );
        assert_eq!(cache.quality, DiffSegmentQuality::SyntaxOnly);
        assert!(cache.left.iter().all(|segment| !segment.changed));
        assert!(cache.right.iter().all(|segment| !segment.changed));
    }

    #[test]
    fn changed_files_tree_is_flat_and_uses_full_paths() {
        let files = vec![
            ChangedFile {
                path: "src/main.rs".to_string(),
                status: FileStatus::Modified,
                staged: false,
                unstaged: true,
                untracked: false,
            },
            ChangedFile {
                path: "README.md".to_string(),
                status: FileStatus::Untracked,
                staged: false,
                unstaged: true,
                untracked: true,
            },
        ];

        let rows = build_changed_file_rows(&files);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "src/main.rs");
        assert_eq!(rows[0].file_status, Some(FileStatus::Modified));
        assert_eq!(rows[1].name, "README.md");
        assert_eq!(rows[1].file_status, Some(FileStatus::Untracked));
    }
}
