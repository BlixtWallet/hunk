use hunk_domain::comments::{CommentLineSide, compute_comment_anchor_hash};
use hunk_domain::diff::{DiffCellKind, DiffRowKind, SideBySideRow};

use super::{DiffStream, DiffStreamRowKind};

pub const DIFF_COMMENT_CONTEXT_RADIUS_ROWS: usize = 2;
const COMMENT_FUZZY_MATCH_MIN_SCORE: i32 = 6;
const COMMENT_FUZZY_RENAME_MATCH_MIN_SCORE: i32 = 11;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffCommentAnchor {
    pub stable_id: u64,
    pub file_path: String,
    pub line_side: CommentLineSide,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    pub hunk_header: Option<String>,
    pub line_text: String,
    pub context_before: String,
    pub context_after: String,
    pub anchor_hash: String,
}

#[derive(Debug, Clone, Copy)]
pub struct DiffCommentLookup<'a> {
    pub file_path: &'a str,
    pub line_side: CommentLineSide,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    pub hunk_header: Option<&'a str>,
    pub line_text: &'a str,
    pub context_before: &'a str,
    pub context_after: &'a str,
    pub anchor_hash: &'a str,
}

pub fn find_diff_comment_row(
    anchors: &[Option<DiffCommentAnchor>],
    lookup: DiffCommentLookup<'_>,
) -> Option<usize> {
    let key = FuzzyCommentKey::from_lookup(lookup);
    let mut hash_fallback = None;
    let mut fuzzy_fallback = None::<(usize, i32)>;
    let mut rename_fallback = None::<(usize, i32)>;

    for (row, anchor) in anchors
        .iter()
        .enumerate()
        .filter_map(|(row, anchor)| anchor.as_ref().map(|anchor| (row, anchor)))
    {
        if anchor.file_path == lookup.file_path {
            if exact_anchor_match(anchor, lookup) {
                return Some(row);
            }
            if hash_fallback.is_none() && anchor.anchor_hash == lookup.anchor_hash {
                hash_fallback = Some(row);
            }
            let score = fuzzy_anchor_match_score(&key, anchor);
            if score >= COMMENT_FUZZY_MATCH_MIN_SCORE
                && fuzzy_fallback.is_none_or(|(_, best)| score > best)
            {
                fuzzy_fallback = Some((row, score));
            }
        } else {
            let score = fuzzy_anchor_match_score(&key, anchor);
            if score >= COMMENT_FUZZY_RENAME_MATCH_MIN_SCORE
                && rename_fallback.is_none_or(|(_, best)| score > best)
            {
                rename_fallback = Some((row, score));
            }
        }
    }

    hash_fallback
        .or_else(|| fuzzy_fallback.map(|(row, _)| row))
        .or_else(|| rename_fallback.map(|(row, _)| row))
}

#[derive(Debug)]
struct FuzzyCommentKey {
    line_side: CommentLineSide,
    old_line: Option<u32>,
    new_line: Option<u32>,
    line_text: String,
    line_core: String,
    hunk_header: String,
    context_before_line: String,
    context_after_line: String,
}

impl FuzzyCommentKey {
    fn from_lookup(lookup: DiffCommentLookup<'_>) -> Self {
        Self {
            line_side: lookup.line_side,
            old_line: lookup.old_line,
            new_line: lookup.new_line,
            line_text: normalize_text_for_fuzzy(lookup.line_text),
            line_core: normalize_diff_line_body(lookup.line_text),
            hunk_header: normalize_text_for_fuzzy(lookup.hunk_header.unwrap_or("")),
            context_before_line: normalize_diff_line_body(last_non_empty_line(
                lookup.context_before,
            )),
            context_after_line: normalize_diff_line_body(first_non_empty_line(
                lookup.context_after,
            )),
        }
    }
}

pub fn build_diff_comment_anchors(
    stream: &DiffStream,
    context_radius_rows: usize,
) -> Vec<Option<DiffCommentAnchor>> {
    let mut current_path = None::<&str>;
    let mut current_hunk = None::<&str>;
    (0..stream.rows.len())
        .map(|row_index| {
            let metadata = stream.row_metadata.get(row_index);
            let path = metadata.and_then(|row| row.file_path.as_deref());
            if path != current_path {
                current_path = path;
                current_hunk = None;
            }
            if metadata.is_some_and(|row| row.kind == DiffStreamRowKind::CoreHunkHeader) {
                current_hunk = stream.rows.get(row_index).map(|row| row.text.as_str());
            }
            build_comment_anchor(stream, current_hunk, row_index, context_radius_rows)
        })
        .collect()
}

fn build_comment_anchor(
    stream: &DiffStream,
    hunk_header: Option<&str>,
    row_index: usize,
    context_radius_rows: usize,
) -> Option<DiffCommentAnchor> {
    let row = stream.rows.get(row_index)?;
    let metadata = stream.row_metadata.get(row_index)?;
    if !matches!(
        metadata.kind,
        DiffStreamRowKind::CoreCode | DiffStreamRowKind::CoreMeta | DiffStreamRowKind::CoreEmpty
    ) {
        return None;
    }

    let file_path = metadata.file_path.clone()?;
    let line_text = diff_row_lines(row).join("\n");
    let (line_side, old_line, new_line) = comment_line_location(row);
    let context_before = collect_row_context(
        stream,
        row_index.saturating_sub(context_radius_rows)..row_index,
        file_path.as_str(),
    );
    let context_after = collect_row_context(
        stream,
        row_index.saturating_add(1)
            ..row_index
                .saturating_add(1)
                .saturating_add(context_radius_rows)
                .min(stream.rows.len()),
        file_path.as_str(),
    );
    let hunk_header = hunk_header.map(ToOwned::to_owned);
    let anchor_hash = compute_comment_anchor_hash(
        file_path.as_str(),
        hunk_header.as_deref(),
        line_text.as_str(),
        context_before.as_str(),
        context_after.as_str(),
    );

    Some(DiffCommentAnchor {
        stable_id: metadata.stable_id,
        file_path,
        line_side,
        old_line,
        new_line,
        hunk_header,
        line_text,
        context_before,
        context_after,
        anchor_hash,
    })
}

fn comment_line_location(row: &SideBySideRow) -> (CommentLineSide, Option<u32>, Option<u32>) {
    if row.kind != DiffRowKind::Code {
        return (CommentLineSide::Meta, None, None);
    }
    if row.right.kind != DiffCellKind::None {
        (CommentLineSide::Right, row.left.line, row.right.line)
    } else if row.left.kind != DiffCellKind::None {
        (CommentLineSide::Left, row.left.line, row.right.line)
    } else {
        (CommentLineSide::Meta, None, None)
    }
}

fn collect_row_context(
    stream: &DiffStream,
    range: std::ops::Range<usize>,
    anchor_path: &str,
) -> String {
    let mut lines = Vec::new();
    for row_index in range {
        let same_file = stream
            .row_metadata
            .get(row_index)
            .and_then(|row| row.file_path.as_deref())
            == Some(anchor_path);
        if same_file && let Some(row) = stream.rows.get(row_index) {
            lines.extend(diff_row_lines(row));
        }
    }
    lines.join("\n")
}

fn diff_row_lines(row: &SideBySideRow) -> Vec<String> {
    let mut lines = Vec::with_capacity(2);
    match row.kind {
        DiffRowKind::Code => {
            if row.left.kind == DiffCellKind::Removed {
                lines.push(format!("-{}", row.left.text));
            }
            if row.right.kind == DiffCellKind::Added {
                lines.push(format!("+{}", row.right.text));
            }
            if row.left.kind == DiffCellKind::Context {
                lines.push(format!(" {}", row.left.text));
            }
            if row.left.kind == DiffCellKind::None
                && row.right.kind == DiffCellKind::None
                && !row.text.is_empty()
            {
                lines.push(row.text.clone());
            }
        }
        DiffRowKind::HunkHeader => {}
        DiffRowKind::Meta | DiffRowKind::Empty => lines.push(row.text.clone()),
    }
    lines
}

fn exact_anchor_match(anchor: &DiffCommentAnchor, lookup: DiffCommentLookup<'_>) -> bool {
    if anchor.file_path != lookup.file_path {
        return false;
    }
    match lookup.line_side {
        CommentLineSide::Left => {
            anchor.old_line == lookup.old_line
                && (lookup.new_line.is_none() || anchor.new_line == lookup.new_line)
        }
        CommentLineSide::Right => {
            anchor.new_line == lookup.new_line
                && (lookup.old_line.is_none() || anchor.old_line == lookup.old_line)
        }
        CommentLineSide::Meta => {
            anchor.line_side == CommentLineSide::Meta && anchor.line_text == lookup.line_text
        }
    }
}

fn fuzzy_anchor_match_score(key: &FuzzyCommentKey, anchor: &DiffCommentAnchor) -> i32 {
    let mut score = if key.line_side == anchor.line_side {
        2
    } else {
        -1
    };

    let anchor_line = normalize_text_for_fuzzy(anchor.line_text.as_str());
    if !key.line_text.is_empty() && key.line_text == anchor_line {
        score += 6;
    } else {
        let anchor_core = normalize_diff_line_body(anchor.line_text.as_str());
        if !key.line_core.is_empty() && key.line_core == anchor_core {
            score += 5;
        } else if has_substring_overlap(key.line_core.as_str(), anchor_core.as_str()) {
            score += 3;
        }
    }

    let anchor_hunk = normalize_text_for_fuzzy(anchor.hunk_header.as_deref().unwrap_or(""));
    if !key.hunk_header.is_empty() && key.hunk_header == anchor_hunk {
        score += 2;
    }
    let anchor_before =
        normalize_diff_line_body(last_non_empty_line(anchor.context_before.as_str()));
    let anchor_after =
        normalize_diff_line_body(first_non_empty_line(anchor.context_after.as_str()));
    score += context_line_score(key.context_before_line.as_str(), anchor_before.as_str());
    score += context_line_score(key.context_after_line.as_str(), anchor_after.as_str());
    score += line_distance_score(key.old_line, anchor.old_line);
    score += line_distance_score(key.new_line, anchor.new_line);
    score
}

fn normalize_text_for_fuzzy(text: &str) -> String {
    text.split_whitespace()
        .map(|part| part.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_diff_line_body(text: &str) -> String {
    text.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            trimmed
                .strip_prefix('+')
                .or_else(|| trimmed.strip_prefix('-'))
                .or_else(|| trimmed.strip_prefix(' '))
                .unwrap_or(trimmed)
                .trim()
        })
        .filter(|line| !line.is_empty())
        .map(normalize_text_for_fuzzy)
        .collect::<Vec<_>>()
        .join(" ")
}

fn first_non_empty_line(text: &str) -> &str {
    text.lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
}

fn last_non_empty_line(text: &str) -> &str {
    text.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
}

fn has_substring_overlap(left: &str, right: &str) -> bool {
    left.len().min(right.len()) >= 12 && (left.contains(right) || right.contains(left))
}

fn context_line_score(left: &str, right: &str) -> i32 {
    if left.is_empty() || right.is_empty() {
        0
    } else if left == right {
        2
    } else if has_substring_overlap(left, right) {
        1
    } else {
        0
    }
}

fn line_distance_score(left: Option<u32>, right: Option<u32>) -> i32 {
    match (left, right) {
        (Some(left), Some(right)) => match left.abs_diff(right) {
            0 => 2,
            1..=2 => 1,
            3..=8 => 0,
            _ => -1,
        },
        _ => 0,
    }
}
