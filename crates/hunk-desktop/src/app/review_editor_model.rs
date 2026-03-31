use std::collections::{BTreeMap, BTreeSet, HashMap};

use hunk_domain::diff::{DiffCellKind, DiffRowKind, SideBySideRow};
use hunk_editor::{FoldRegion, OverlayDescriptor, OverlayKind, SpacerDescriptor};

const MAX_LINE_LCS_MATRIX_CELLS: usize = 200_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewEditorPresentation {
    pub left_overlays: Vec<OverlayDescriptor>,
    pub right_overlays: Vec<OverlayDescriptor>,
    pub left_folds: Vec<FoldRegion>,
    pub right_folds: Vec<FoldRegion>,
    pub left_spacers: Vec<SpacerDescriptor>,
    pub right_spacers: Vec<SpacerDescriptor>,
    pub right_hunk_lines: Vec<usize>,
    pub right_to_left_line_map: Vec<Option<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewEditorRightLineAnchor {
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    pub line_text: String,
    pub context_before: String,
    pub context_after: String,
}

pub(crate) fn should_preserve_dirty_review_editor_right(
    previous_path: Option<&str>,
    previous_left_source_id: Option<&str>,
    previous_right_source_id: Option<&str>,
    next_path: &str,
    next_left_source_id: Option<&str>,
    next_right_source_id: Option<&str>,
    right_is_dirty: bool,
) -> bool {
    right_is_dirty
        && previous_path == Some(next_path)
        && previous_left_source_id == next_left_source_id
        && previous_right_source_id == next_right_source_id
}

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

#[allow(dead_code)]
pub(crate) fn build_review_editor_overlays_from_texts(
    left_text: &str,
    right_text: &str,
) -> (Vec<OverlayDescriptor>, Vec<OverlayDescriptor>) {
    let presentation = build_review_editor_presentation_from_texts(left_text, right_text, 0, None);
    (presentation.left_overlays, presentation.right_overlays)
}

pub(crate) fn build_review_editor_presentation_from_texts(
    left_text: &str,
    right_text: &str,
    context_radius: usize,
    pinned_right_line: Option<usize>,
) -> ReviewEditorPresentation {
    let left_lines = text_lines(left_text);
    let right_lines = text_lines(right_text);
    let mut left = BTreeMap::new();
    let mut right = BTreeMap::new();
    let mut left_spacers = BTreeMap::new();
    let mut right_spacers = BTreeMap::new();
    let mut left_changed_lines = BTreeSet::new();
    let mut right_changed_lines = BTreeSet::new();
    let mut right_to_left_line_map = vec![None; right_lines.len()];

    let ops = build_line_diff_ops(&left_lines, &right_lines);

    let mut left_line = 0usize;
    let mut right_line = 0usize;
    let mut ix = 0usize;
    while ix < ops.len() {
        match ops[ix] {
            LineDiffOp::Equal => {
                if right_line < right_to_left_line_map.len() {
                    right_to_left_line_map[right_line] = Some(left_line);
                }
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
                    left_changed_lines.insert(left_line + offset);
                    right_changed_lines.insert(right_line + offset);
                    let right_ix = right_line + offset;
                    if right_ix < right_to_left_line_map.len() {
                        right_to_left_line_map[right_ix] = Some(left_line + offset);
                    }
                }
                for offset in paired_count..deleted_count {
                    left.insert(left_line + offset, OverlayKind::DiffDeletion);
                    left_changed_lines.insert(left_line + offset);
                }
                for offset in paired_count..inserted_count {
                    right.insert(right_line + offset, OverlayKind::DiffAddition);
                    right_changed_lines.insert(right_line + offset);
                    let right_ix = right_line + offset;
                    if right_ix < right_to_left_line_map.len() {
                        right_to_left_line_map[right_ix] = None;
                    }
                }
                if deleted_count > inserted_count {
                    *right_spacers
                        .entry(right_line + inserted_count)
                        .or_insert(0) += deleted_count - inserted_count;
                } else if inserted_count > deleted_count {
                    *left_spacers.entry(left_line + deleted_count).or_insert(0) +=
                        inserted_count - deleted_count;
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
                    right_changed_lines.insert(right_line + offset);
                    let right_ix = right_line + offset;
                    if right_ix < right_to_left_line_map.len() {
                        right_to_left_line_map[right_ix] = None;
                    }
                }
                *left_spacers.entry(left_line).or_insert(0) += inserted_count;
                right_line = right_line.saturating_add(inserted_count);
            }
        }
    }

    let pinned_left_line = pinned_right_line.and_then(|line| {
        build_right_line_entries(left_text, right_text)
            .get(line)
            .and_then(|entry| entry.old_line)
            .map(|line| line.saturating_sub(1) as usize)
    });

    ReviewEditorPresentation {
        left_overlays: overlays_from_entries(left),
        right_overlays: overlays_from_entries(right),
        left_folds: build_fold_regions(
            left_lines.len(),
            &left_changed_lines,
            context_radius,
            pinned_left_line,
        ),
        right_folds: build_fold_regions(
            right_lines.len(),
            &right_changed_lines,
            context_radius,
            pinned_right_line,
        ),
        left_spacers: spacers_from_entries(left_spacers),
        right_spacers: spacers_from_entries(right_spacers),
        right_hunk_lines: right_hunk_lines(&right_changed_lines),
        right_to_left_line_map,
    }
}

pub(crate) fn build_review_editor_right_line_anchor_from_texts(
    left_text: &str,
    right_text: &str,
    right_line_index: usize,
    context_radius: usize,
) -> Option<ReviewEditorRightLineAnchor> {
    let entries = build_right_line_entries(left_text, right_text);
    let entry = entries.get(right_line_index)?.clone();
    let context_before = entries
        .iter()
        .take(right_line_index)
        .rev()
        .take(context_radius)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|entry| entry.line_text.clone())
        .collect::<Vec<_>>()
        .join("\n");
    let context_after = entries
        .iter()
        .skip(right_line_index.saturating_add(1))
        .take(context_radius)
        .map(|entry| entry.line_text.clone())
        .collect::<Vec<_>>()
        .join("\n");

    Some(ReviewEditorRightLineAnchor {
        old_line: entry.old_line,
        new_line: entry.new_line,
        line_text: entry.line_text,
        context_before,
        context_after,
    })
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

fn spacers_from_entries(entries: BTreeMap<usize, usize>) -> Vec<SpacerDescriptor> {
    entries
        .into_iter()
        .filter(|(_, row_count)| *row_count > 0)
        .map(|(before_line, row_count)| SpacerDescriptor {
            before_line,
            row_count,
        })
        .collect()
}

pub(crate) fn find_wrapped_review_editor_hunk_line(
    right_hunk_lines: &[usize],
    current_line: usize,
    direction: isize,
) -> Option<usize> {
    if right_hunk_lines.is_empty() {
        return None;
    }

    if direction >= 0 {
        right_hunk_lines
            .iter()
            .copied()
            .find(|line| *line > current_line)
            .or_else(|| right_hunk_lines.first().copied())
    } else {
        right_hunk_lines
            .iter()
            .rev()
            .copied()
            .find(|line| *line < current_line)
            .or_else(|| right_hunk_lines.last().copied())
    }
}

pub(crate) fn nearest_mapped_review_editor_left_line(
    right_to_left_line_map: &[Option<usize>],
    right_line: usize,
) -> Option<usize> {
    if right_to_left_line_map.is_empty() {
        return None;
    }

    let clamped = right_line.min(right_to_left_line_map.len().saturating_sub(1));
    if let Some(line) = right_to_left_line_map[clamped] {
        return Some(line);
    }

    let mut backward = clamped;
    while backward > 0 {
        backward -= 1;
        if let Some(line) = right_to_left_line_map[backward] {
            return Some(line);
        }
    }

    right_to_left_line_map
        .iter()
        .skip(clamped.saturating_add(1))
        .flatten()
        .next()
        .copied()
}

fn build_fold_regions(
    total_lines: usize,
    changed_lines: &BTreeSet<usize>,
    context_radius: usize,
    pinned_line: Option<usize>,
) -> Vec<FoldRegion> {
    if total_lines == 0 {
        return Vec::new();
    }

    let mut visible_ranges = changed_lines
        .iter()
        .copied()
        .map(|line| {
            (
                line.saturating_sub(context_radius),
                line.saturating_add(context_radius)
                    .min(total_lines.saturating_sub(1)),
            )
        })
        .collect::<Vec<_>>();

    if let Some(line) = pinned_line.filter(|line| *line < total_lines) {
        visible_ranges.push((line, line));
    }

    if visible_ranges.is_empty() {
        return Vec::new();
    }

    visible_ranges.sort_unstable_by_key(|(start, _)| *start);
    let mut merged: Vec<(usize, usize)> = Vec::with_capacity(visible_ranges.len());
    for (start, end) in visible_ranges {
        if let Some((_, previous_end)) = merged.last_mut()
            && start <= previous_end.saturating_add(1)
        {
            *previous_end = (*previous_end).max(end);
            continue;
        }
        merged.push((start, end));
    }

    let mut folds = Vec::new();
    let mut cursor = 0usize;
    for (start, end) in merged {
        if start > cursor + 1
            && let Some(region) = FoldRegion::new(cursor, start - 1)
        {
            folds.push(region);
        }
        cursor = end.saturating_add(1);
    }

    if total_lines > cursor + 1
        && let Some(region) = FoldRegion::new(cursor, total_lines - 1)
    {
        folds.push(region);
    }

    folds
}

fn right_hunk_lines(changed_lines: &BTreeSet<usize>) -> Vec<usize> {
    let mut hunks = Vec::new();
    let mut previous_line = None::<usize>;
    for line in changed_lines.iter().copied() {
        if previous_line.is_none_or(|previous| line > previous.saturating_add(1)) {
            hunks.push(line);
        }
        previous_line = Some(line);
    }
    hunks
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineDiffOp {
    Equal,
    Delete,
    Insert,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RightLineEntry {
    old_line: Option<u32>,
    new_line: Option<u32>,
    line_text: String,
}

fn text_lines(text: &str) -> Vec<&str> {
    text.split('\n').collect()
}

fn build_line_diff_ops(left_lines: &[&str], right_lines: &[&str]) -> Vec<LineDiffOp> {
    if left_lines.is_empty() {
        return std::iter::repeat_n(LineDiffOp::Insert, right_lines.len()).collect();
    }
    if right_lines.is_empty() {
        return std::iter::repeat_n(LineDiffOp::Delete, left_lines.len()).collect();
    }

    let anchors = patience_anchor_pairs(left_lines, right_lines);
    if anchors.is_empty() {
        return build_fallback_line_diff_ops(left_lines, right_lines);
    }

    let mut ops = Vec::new();
    let mut left_start = 0usize;
    let mut right_start = 0usize;
    for (left_anchor, right_anchor) in anchors {
        ops.extend(build_line_diff_ops(
            &left_lines[left_start..left_anchor],
            &right_lines[right_start..right_anchor],
        ));
        ops.push(LineDiffOp::Equal);
        left_start = left_anchor.saturating_add(1);
        right_start = right_anchor.saturating_add(1);
    }
    ops.extend(build_line_diff_ops(
        &left_lines[left_start..],
        &right_lines[right_start..],
    ));
    ops
}

fn build_fallback_line_diff_ops(left_lines: &[&str], right_lines: &[&str]) -> Vec<LineDiffOp> {
    let matrix_cells = left_lines.len().saturating_mul(right_lines.len());
    if matrix_cells <= MAX_LINE_LCS_MATRIX_CELLS {
        build_lcs_line_diff_ops(left_lines, right_lines)
    } else {
        build_coarse_line_diff_ops(left_lines, right_lines)
    }
}

fn build_lcs_line_diff_ops(left_lines: &[&str], right_lines: &[&str]) -> Vec<LineDiffOp> {
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

fn patience_anchor_pairs(left_lines: &[&str], right_lines: &[&str]) -> Vec<(usize, usize)> {
    let left_unique = unique_line_indices(left_lines);
    let right_unique = unique_line_indices(right_lines);
    let pairs = left_lines
        .iter()
        .enumerate()
        .filter_map(|(left_ix, line)| {
            (left_unique.get(line) == Some(&left_ix))
                .then_some(
                    right_unique
                        .get(line)
                        .copied()
                        .map(|right_ix| (left_ix, right_ix)),
                )
                .flatten()
        })
        .collect::<Vec<_>>();
    longest_increasing_right_sequence(&pairs)
}

fn unique_line_indices<'a>(lines: &[&'a str]) -> HashMap<&'a str, usize> {
    let mut counts = HashMap::new();
    for (ix, line) in lines.iter().copied().enumerate() {
        counts
            .entry(line)
            .and_modify(|(count, _)| *count += 1)
            .or_insert((1usize, ix));
    }

    counts
        .into_iter()
        .filter_map(|(line, (count, ix))| (count == 1).then_some((line, ix)))
        .collect()
}

fn longest_increasing_right_sequence(pairs: &[(usize, usize)]) -> Vec<(usize, usize)> {
    if pairs.is_empty() {
        return Vec::new();
    }

    let mut tails = Vec::<usize>::new();
    let mut previous = vec![None; pairs.len()];
    for (pair_ix, &(_, right_ix)) in pairs.iter().enumerate() {
        let tail_ix = tails
            .binary_search_by_key(&right_ix, |tail| pairs[*tail].1)
            .unwrap_or_else(|ix| ix);
        if tail_ix > 0 {
            previous[pair_ix] = tails.get(tail_ix - 1).copied();
        }
        if tail_ix == tails.len() {
            tails.push(pair_ix);
        } else {
            tails[tail_ix] = pair_ix;
        }
    }

    let mut lis = Vec::new();
    let mut cursor = tails.last().copied();
    while let Some(pair_ix) = cursor {
        lis.push(pairs[pair_ix]);
        cursor = previous[pair_ix];
    }
    lis.reverse();
    lis
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

fn build_right_line_entries(left_text: &str, right_text: &str) -> Vec<RightLineEntry> {
    let left_lines = text_lines(left_text);
    let right_lines = text_lines(right_text);
    let ops = build_line_diff_ops(&left_lines, &right_lines);

    let mut entries = Vec::new();
    let mut left_line = 1u32;
    let mut right_line = 1u32;
    let mut ix = 0usize;
    while ix < ops.len() {
        match ops[ix] {
            LineDiffOp::Equal => {
                let right_ix = right_line.saturating_sub(1) as usize;
                entries.push(RightLineEntry {
                    old_line: Some(left_line),
                    new_line: Some(right_line),
                    line_text: format!(" {}", right_lines[right_ix]),
                });
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
                    let old_line = left_line.saturating_add(offset as u32);
                    let new_line = right_line.saturating_add(offset as u32);
                    let right_ix = new_line.saturating_sub(1) as usize;
                    entries.push(RightLineEntry {
                        old_line: Some(old_line),
                        new_line: Some(new_line),
                        line_text: format!("+{}", right_lines[right_ix]),
                    });
                }
                for offset in paired_count..inserted_count {
                    let new_line = right_line.saturating_add(offset as u32);
                    let right_ix = new_line.saturating_sub(1) as usize;
                    entries.push(RightLineEntry {
                        old_line: None,
                        new_line: Some(new_line),
                        line_text: format!("+{}", right_lines[right_ix]),
                    });
                }

                left_line = left_line.saturating_add(deleted_count as u32);
                right_line = right_line.saturating_add(inserted_count as u32);
            }
            LineDiffOp::Insert => {
                let insert_start = ix;
                while ix < ops.len() && ops[ix] == LineDiffOp::Insert {
                    ix += 1;
                }
                let inserted_count = ix.saturating_sub(insert_start);
                for offset in 0..inserted_count {
                    let new_line = right_line.saturating_add(offset as u32);
                    let right_ix = new_line.saturating_sub(1) as usize;
                    entries.push(RightLineEntry {
                        old_line: None,
                        new_line: Some(new_line),
                        line_text: format!("+{}", right_lines[right_ix]),
                    });
                }
                right_line = right_line.saturating_add(inserted_count as u32);
            }
        }
    }

    entries
}
