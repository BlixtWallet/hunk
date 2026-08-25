use hunk_domain::comments::{CommentLineSide, compute_comment_anchor_hash};
use hunk_domain::diff::{DiffCellKind, DiffRowKind, SideBySideRow};

use super::{DiffStream, DiffStreamRowKind};

pub const DIFF_COMMENT_CONTEXT_RADIUS_ROWS: usize = 2;

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
