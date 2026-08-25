use std::path::Path;

use hunk_domain::diff::DiffCellKind;

use super::highlight::{
    StyledSegment, SyntaxTokenKind, build_line_segments, build_syntax_only_line_segments,
};

const MAX_RENDER_SEGMENTS_PER_CELL_DETAILED: usize = 48;
const MAX_RENDER_SEGMENTS_PER_CELL_LARGE_FILE: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedStyledSegment {
    pub plain_text: String,
    pub syntax: SyntaxTokenKind,
    pub changed: bool,
    pub search_match: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffRowSegmentCache {
    pub quality: DiffSegmentQuality,
    pub left: Vec<CachedStyledSegment>,
    pub right: Vec<CachedStyledSegment>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiffSegmentQuality {
    #[default]
    Plain,
    SyntaxOnly,
    Detailed,
}

pub fn build_diff_row_segment_cache_from_cells(
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

fn cached_segments_from_styled(segments: Vec<StyledSegment>) -> Vec<CachedStyledSegment> {
    segments
        .into_iter()
        .map(|segment| CachedStyledSegment {
            plain_text: segment.text,
            syntax: segment.syntax,
            changed: segment.changed,
            search_match: false,
        })
        .collect()
}

pub fn compact_cached_segments_for_render(
    segments: Vec<CachedStyledSegment>,
    max_segments: usize,
) -> Vec<CachedStyledSegment> {
    if max_segments == 0 || segments.len() <= max_segments {
        return segments;
    }

    let chunk_size = segments.len().div_ceil(max_segments);
    let mut compacted = Vec::with_capacity(max_segments);
    for chunk in segments.chunks(chunk_size) {
        if chunk.is_empty() {
            continue;
        }

        let plain_capacity = chunk
            .iter()
            .map(|segment| segment.plain_text.len())
            .sum::<usize>();
        let mut plain_text = String::with_capacity(plain_capacity);
        let first_syntax = chunk[0].syntax;
        let mut mixed_syntax = false;
        let mut changed = false;
        let mut search_match = false;
        for segment in chunk {
            plain_text.push_str(segment.plain_text.as_str());
            changed |= segment.changed;
            search_match |= segment.search_match;
            if segment.syntax != first_syntax {
                mixed_syntax = true;
            }
        }

        compacted.push(CachedStyledSegment {
            plain_text,
            syntax: if mixed_syntax {
                SyntaxTokenKind::Plain
            } else {
                first_syntax
            },
            changed,
            search_match,
        });
    }

    compacted
}

pub fn cached_runtime_fallback_segments(text: &str) -> Vec<CachedStyledSegment> {
    if text.is_empty() {
        return Vec::new();
    }

    vec![CachedStyledSegment {
        plain_text: text.to_string(),
        syntax: SyntaxTokenKind::Plain,
        changed: false,
        search_match: false,
    }]
}

pub fn is_probably_binary_extension(path: &str) -> bool {
    let Some(extension) = Path::new(path).extension().and_then(|ext| ext.to_str()) else {
        return false;
    };

    let extension = extension.to_ascii_lowercase();
    matches!(
        extension.as_str(),
        "7z" | "a"
            | "apk"
            | "bin"
            | "bmp"
            | "class"
            | "dll"
            | "dmg"
            | "doc"
            | "docx"
            | "ear"
            | "eot"
            | "exe"
            | "gif"
            | "gz"
            | "ico"
            | "jar"
            | "jpeg"
            | "jpg"
            | "lib"
            | "lockb"
            | "mov"
            | "mp3"
            | "mp4"
            | "o"
            | "obj"
            | "otf"
            | "pdf"
            | "png"
            | "pyc"
            | "so"
            | "tar"
            | "tif"
            | "tiff"
            | "ttf"
            | "war"
            | "wasm"
            | "webm"
            | "webp"
            | "woff"
            | "woff2"
            | "xls"
            | "xlsx"
            | "zip"
    )
}

pub fn is_binary_patch(patch: &str) -> bool {
    patch.contains('\0')
        || patch.contains("\nGIT binary patch\n")
        || patch
            .lines()
            .any(|line| line.starts_with("Binary files ") && line.ends_with(" differ"))
}
