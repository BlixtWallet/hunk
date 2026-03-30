use std::collections::BTreeMap;

use hunk_domain::diff::{DiffCellKind, DiffRowKind, SideBySideRow};
use hunk_editor::{OverlayDescriptor, OverlayKind};

const MAX_LINE_LCS_MATRIX_CELLS: usize = 200_000;

#[allow(dead_code)]
pub(crate) fn build_review_editor_overlays(
    rows: &[SideBySideRow],
) -> (Vec<OverlayDescriptor>, Vec<OverlayDescriptor>) {
    let mut left = BTreeMap::new();
    let mut right = BTreeMap::new();

    for row in rows {
        if row.kind != DiffRowKind::Code {
            continue;
        }

        match (row.left.line, row.left.kind, row.right.line, row.right.kind) {
            (Some(left_line), DiffCellKind::Removed, Some(right_line), DiffCellKind::Added) => {
                left.insert(
                    left_line.saturating_sub(1) as usize,
                    OverlayKind::DiffModification,
                );
                right.insert(
                    right_line.saturating_sub(1) as usize,
                    OverlayKind::DiffModification,
                );
            }
            (Some(left_line), DiffCellKind::Removed, _, _) => {
                left.insert(
                    left_line.saturating_sub(1) as usize,
                    OverlayKind::DiffDeletion,
                );
            }
            (_, _, Some(right_line), DiffCellKind::Added) => {
                right.insert(
                    right_line.saturating_sub(1) as usize,
                    OverlayKind::DiffAddition,
                );
            }
            _ => {}
        }
    }

    (overlays_from_entries(left), overlays_from_entries(right))
}

pub(crate) fn build_review_editor_overlays_from_texts(
    left_text: &str,
    right_text: &str,
) -> (Vec<OverlayDescriptor>, Vec<OverlayDescriptor>) {
    let left_lines = text_lines(left_text);
    let right_lines = text_lines(right_text);
    let mut left = BTreeMap::new();
    let mut right = BTreeMap::new();

    let matrix_cells = left_lines.len().saturating_mul(right_lines.len());
    let ops = if matrix_cells <= MAX_LINE_LCS_MATRIX_CELLS {
        build_line_diff_ops(&left_lines, &right_lines)
    } else {
        build_coarse_line_diff_ops(&left_lines, &right_lines)
    };

    let mut left_line = 0usize;
    let mut right_line = 0usize;
    let mut ix = 0usize;
    while ix < ops.len() {
        match ops[ix] {
            LineDiffOp::Equal => {
                left_line = left_line.saturating_add(1);
                right_line = right_line.saturating_add(1);
                ix += 1;
            }
            LineDiffOp::Delete => {
                let delete_start = ix;
                while ix < ops.len() && ops[ix] == LineDiffOp::Delete {
                    ix += 1;
                }
                let insert_start = ix;
                while ix < ops.len() && ops[ix] == LineDiffOp::Insert {
                    ix += 1;
                }

                let deleted_count = insert_start.saturating_sub(delete_start);
                let inserted_count = ix.saturating_sub(insert_start);
                let paired_count = deleted_count.min(inserted_count);

                for offset in 0..paired_count {
                    left.insert(left_line + offset, OverlayKind::DiffModification);
                    right.insert(right_line + offset, OverlayKind::DiffModification);
                }
                for offset in paired_count..deleted_count {
                    left.insert(left_line + offset, OverlayKind::DiffDeletion);
                }
                for offset in paired_count..inserted_count {
                    right.insert(right_line + offset, OverlayKind::DiffAddition);
                }

                left_line = left_line.saturating_add(deleted_count);
                right_line = right_line.saturating_add(inserted_count);
            }
            LineDiffOp::Insert => {
                let insert_start = ix;
                while ix < ops.len() && ops[ix] == LineDiffOp::Insert {
                    ix += 1;
                }
                let inserted_count = ix.saturating_sub(insert_start);
                for offset in 0..inserted_count {
                    right.insert(right_line + offset, OverlayKind::DiffAddition);
                }
                right_line = right_line.saturating_add(inserted_count);
            }
        }
    }

    (overlays_from_entries(left), overlays_from_entries(right))
}

fn overlays_from_entries(entries: BTreeMap<usize, OverlayKind>) -> Vec<OverlayDescriptor> {
    entries
        .into_iter()
        .map(|(line, kind)| OverlayDescriptor {
            line,
            kind,
            message: None,
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineDiffOp {
    Equal,
    Delete,
    Insert,
}

fn text_lines(text: &str) -> Vec<&str> {
    text.split('\n').collect()
}

fn build_line_diff_ops(left_lines: &[&str], right_lines: &[&str]) -> Vec<LineDiffOp> {
    let left_len = left_lines.len();
    let right_len = right_lines.len();
    let mut lcs = vec![0usize; (left_len + 1).saturating_mul(right_len + 1)];

    for left_ix in (0..left_len).rev() {
        for right_ix in (0..right_len).rev() {
            let ix = left_ix * (right_len + 1) + right_ix;
            let down = (left_ix + 1) * (right_len + 1) + right_ix;
            let right = left_ix * (right_len + 1) + (right_ix + 1);
            let diagonal = (left_ix + 1) * (right_len + 1) + (right_ix + 1);
            lcs[ix] = if left_lines[left_ix] == right_lines[right_ix] {
                lcs[diagonal].saturating_add(1)
            } else {
                lcs[down].max(lcs[right])
            };
        }
    }

    let mut ops = Vec::new();
    let mut left_ix = 0usize;
    let mut right_ix = 0usize;
    while left_ix < left_len && right_ix < right_len {
        if left_lines[left_ix] == right_lines[right_ix] {
            ops.push(LineDiffOp::Equal);
            left_ix += 1;
            right_ix += 1;
            continue;
        }

        let down = lcs[(left_ix + 1) * (right_len + 1) + right_ix];
        let across = lcs[left_ix * (right_len + 1) + (right_ix + 1)];
        if down >= across {
            ops.push(LineDiffOp::Delete);
            left_ix += 1;
        } else {
            ops.push(LineDiffOp::Insert);
            right_ix += 1;
        }
    }

    while left_ix < left_len {
        ops.push(LineDiffOp::Delete);
        left_ix += 1;
    }
    while right_ix < right_len {
        ops.push(LineDiffOp::Insert);
        right_ix += 1;
    }
    ops
}

fn build_coarse_line_diff_ops(left_lines: &[&str], right_lines: &[&str]) -> Vec<LineDiffOp> {
    let mut prefix_len = 0usize;
    while prefix_len < left_lines.len()
        && prefix_len < right_lines.len()
        && left_lines[prefix_len] == right_lines[prefix_len]
    {
        prefix_len += 1;
    }

    let mut left_suffix_len = left_lines.len();
    let mut right_suffix_len = right_lines.len();
    while left_suffix_len > prefix_len
        && right_suffix_len > prefix_len
        && left_lines[left_suffix_len - 1] == right_lines[right_suffix_len - 1]
    {
        left_suffix_len -= 1;
        right_suffix_len -= 1;
    }

    let mut ops = vec![LineDiffOp::Equal; prefix_len];
    let deleted_count = left_suffix_len.saturating_sub(prefix_len);
    let inserted_count = right_suffix_len.saturating_sub(prefix_len);
    ops.extend(std::iter::repeat_n(LineDiffOp::Delete, deleted_count));
    ops.extend(std::iter::repeat_n(LineDiffOp::Insert, inserted_count));
    let suffix_count = left_lines.len().saturating_sub(left_suffix_len);
    ops.extend(std::iter::repeat_n(LineDiffOp::Equal, suffix_count));
    ops
}
