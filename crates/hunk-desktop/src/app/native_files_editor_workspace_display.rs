use std::collections::BTreeMap;

#[cfg(test)]
use hunk_editor::WorkspaceProjectedSnapshot;
use hunk_editor::{
    SearchHighlight, Viewport, WorkspaceDisplayRow, WorkspaceDisplaySnapshot, WorkspaceDocumentId,
    WorkspaceExcerptId, WorkspaceLayout, WorkspaceRowLocation,
};
use hunk_text::{TextBuffer, TextSnapshot};

#[allow(clippy::duplicate_mod)]
#[path = "workspace_display_buffers.rs"]
mod workspace_display_buffers;

use workspace_display_buffers::{
    WorkspaceSearchMatch, build_workspace_display_snapshot_from_document_snapshots,
    find_workspace_search_matches,
};

use super::{FilesEditor, RowSyntaxSpan};

#[cfg(test)]
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct WorkspaceProjectedRenderSnapshot {
    pub(crate) projection: WorkspaceProjectedSnapshot,
    pub(crate) visible_display_rows: Vec<WorkspaceDisplayRow>,
    pub(crate) syntax_by_display_row: BTreeMap<usize, Vec<RowSyntaxSpan>>,
    pub(crate) line_number_digits: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct WorkspaceVisibleRenderSnapshot {
    pub(crate) rows_by_display_row: BTreeMap<usize, WorkspaceDisplayRow>,
    pub(crate) syntax_by_display_row: BTreeMap<usize, Vec<RowSyntaxSpan>>,
}

impl FilesEditor {
    fn build_workspace_display_snapshot(
        &self,
        viewport: Viewport,
        tab_width: usize,
    ) -> Option<WorkspaceDisplaySnapshot> {
        let layout = self.workspace_session.layout()?;
        let document_snapshots = layout
            .documents()
            .iter()
            .filter_map(|document| {
                self.workspace_buffer_for_document(document.id)
                    .map(|buffer| (document.id, buffer.snapshot()))
            })
            .collect::<BTreeMap<_, _>>();
        let mut snapshot = build_workspace_display_snapshot_from_document_snapshots(
            layout,
            viewport,
            tab_width,
            false,
            &document_snapshots,
        );
        if let Some(query) = self.search_query.as_deref() {
            apply_workspace_search_highlights(
                layout,
                &mut snapshot.visible_rows,
                query,
                &document_snapshots,
            );
        }
        Some(snapshot)
    }

    pub(crate) fn build_workspace_visible_render_snapshot(
        &mut self,
        viewport: Viewport,
        tab_width: usize,
    ) -> Option<WorkspaceVisibleRenderSnapshot> {
        let snapshot = self.build_workspace_display_snapshot(viewport, tab_width)?;
        let syntax_by_display_row =
            self.workspace_display_segments_by_row(&snapshot.visible_rows)?;
        let rows_by_display_row = snapshot
            .visible_rows
            .into_iter()
            .map(|row| (row.row_index, row))
            .collect();
        Some(WorkspaceVisibleRenderSnapshot {
            rows_by_display_row,
            syntax_by_display_row,
        })
    }

    fn workspace_buffer_for_document(
        &self,
        document_id: WorkspaceDocumentId,
    ) -> Option<&TextBuffer> {
        let layout = self.workspace_session.layout()?;
        let document = layout.document(document_id)?;
        self.workspace_buffers.get(document.path())
    }
}

fn apply_workspace_search_highlights(
    layout: &WorkspaceLayout,
    visible_rows: &mut [WorkspaceDisplayRow],
    query: &str,
    document_snapshots: &BTreeMap<WorkspaceDocumentId, TextSnapshot>,
) {
    if query.trim().is_empty() {
        return;
    }

    let matches = find_workspace_search_matches(layout, query, document_snapshots);
    let mut matches_by_excerpt =
        BTreeMap::<(WorkspaceDocumentId, WorkspaceExcerptId), Vec<WorkspaceSearchMatch>>::new();
    for found in matches {
        matches_by_excerpt
            .entry((found.document_id, found.excerpt_id))
            .or_default()
            .push(found);
    }

    for row in visible_rows {
        let Some(location) = row.location.as_ref() else {
            continue;
        };
        let Some(document_line) = location.document_line else {
            continue;
        };
        let Some(snapshot) = document_snapshots.get(&location.document_id) else {
            continue;
        };
        let Some(matches) = matches_by_excerpt.get(&(location.document_id, location.excerpt_id))
        else {
            continue;
        };
        row.search_highlights =
            workspace_search_highlights_for_row(row, location, document_line, matches, snapshot);
    }
}

fn workspace_search_highlights_for_row(
    row: &WorkspaceDisplayRow,
    location: &WorkspaceRowLocation,
    document_line: usize,
    matches: &[WorkspaceSearchMatch],
    snapshot: &TextSnapshot,
) -> Vec<SearchHighlight> {
    let mut highlights = Vec::new();
    for found in matches {
        if found.excerpt_id != location.excerpt_id {
            continue;
        }
        let (Ok(start), Ok(end)) = (
            snapshot.byte_to_position(found.byte_range.start),
            snapshot.byte_to_position(found.byte_range.end),
        ) else {
            continue;
        };
        if document_line < start.line || document_line > end.line {
            continue;
        }

        let start_raw_column = if document_line == start.line {
            start.column
        } else {
            row.raw_start_column
        };
        let end_raw_column = if document_line == end.line {
            end.column
        } else {
            row.raw_end_column
        };
        let start_column = workspace_display_column_for_raw(row, start_raw_column);
        let end_column = workspace_display_column_for_raw(row, end_raw_column);
        if start_column < end_column {
            highlights.push(SearchHighlight {
                start_column,
                end_column,
            });
        }
    }
    highlights
}

fn workspace_display_column_for_raw(row: &WorkspaceDisplayRow, raw_column: usize) -> usize {
    if row.raw_column_offsets.is_empty() {
        return 0;
    }

    let relative_raw = raw_column
        .saturating_sub(row.raw_start_column)
        .min(row.raw_column_offsets.len().saturating_sub(1));
    row.raw_column_offsets[relative_raw]
}
