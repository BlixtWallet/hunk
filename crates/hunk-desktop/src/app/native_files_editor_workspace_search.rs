use std::collections::BTreeMap;
use std::path::PathBuf;

use hunk_editor::EditorCommand;
use hunk_editor::WorkspaceDocumentId;
use hunk_text::{Selection, TextPosition};

#[allow(clippy::duplicate_mod)]
#[path = "workspace_display_buffers.rs"]
mod workspace_display_buffers;

use workspace_display_buffers::find_workspace_search_matches;

use super::FilesEditor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WorkspaceSearchTarget {
    path: PathBuf,
    document_id: WorkspaceDocumentId,
    surface_order: usize,
    byte_range: std::ops::Range<usize>,
    start: TextPosition,
    end: TextPosition,
}

impl FilesEditor {
    pub(super) fn workspace_search_matches(
        &self,
        query: &str,
    ) -> Option<Vec<WorkspaceSearchTarget>> {
        let layout = self.workspace_session.layout()?;
        let document_snapshots = layout
            .documents()
            .iter()
            .filter_map(|document| {
                let snapshot = if self.active_path() == Some(document.path()) {
                    Some(self.editor.buffer().snapshot())
                } else {
                    self.workspace_buffers
                        .get(document.path())
                        .map(|buffer| buffer.snapshot())
                }?;
                Some((document.id, snapshot))
            })
            .collect::<BTreeMap<_, _>>();

        let document_surface_order = layout.excerpts().iter().enumerate().fold(
            BTreeMap::new(),
            |mut orders, (surface_order, excerpt)| {
                orders
                    .entry(excerpt.spec.document_id)
                    .or_insert(surface_order);
                orders
            },
        );

        Some(
            find_workspace_search_matches(layout, query, &document_snapshots)
                .into_iter()
                .filter_map(|candidate| {
                    let document = layout.document(candidate.document_id)?;
                    let snapshot = document_snapshots.get(&candidate.document_id)?;
                    let start = snapshot.byte_to_position(candidate.byte_range.start).ok()?;
                    let end = snapshot.byte_to_position(candidate.byte_range.end).ok()?;
                    Some(WorkspaceSearchTarget {
                        path: document.path.clone(),
                        document_id: candidate.document_id,
                        surface_order: *document_surface_order.get(&candidate.document_id)?,
                        byte_range: candidate.byte_range,
                        start,
                        end,
                    })
                })
                .collect(),
        )
    }

    pub(super) fn select_next_workspace_search_match(
        &mut self,
        matches: &[WorkspaceSearchTarget],
        forward: bool,
    ) -> bool {
        let Some(target) = self.next_workspace_search_target(matches, forward) else {
            return false;
        };
        if self.active_path() != Some(target.path.as_path())
            && self.activate_workspace_path(target.path.as_path()).ok() != Some(true)
        {
            return false;
        }
        self.editor
            .apply(EditorCommand::SetSelection(Selection::new(
                target.start,
                target.end,
            )))
            .selection_changed
    }

    fn next_workspace_search_target(
        &self,
        matches: &[WorkspaceSearchTarget],
        forward: bool,
    ) -> Option<WorkspaceSearchTarget> {
        if matches.is_empty() {
            return None;
        }

        let current_path = self.active_path()?;
        let current_doc_id = self
            .workspace_session
            .layout()?
            .documents()
            .iter()
            .find(|document| document.path.as_path() == current_path)?
            .id;
        let current_surface_order = self
            .workspace_session
            .layout()?
            .excerpts()
            .iter()
            .enumerate()
            .find_map(|(surface_order, excerpt)| {
                (excerpt.spec.document_id == current_doc_id).then_some(surface_order)
            })?;
        let snapshot = self.editor.buffer().snapshot();
        let selection = self.editor.selection().range();
        let caret_start = snapshot.position_to_byte(selection.start).ok()?;
        let caret_end = snapshot.position_to_byte(selection.end).ok()?;

        if forward {
            matches
                .iter()
                .find(|target| {
                    target.surface_order > current_surface_order
                        || (target.document_id == current_doc_id
                            && target.byte_range.start > caret_end)
                })
                .or_else(|| matches.first())
                .cloned()
        } else {
            matches
                .iter()
                .rev()
                .find(|target| {
                    target.surface_order < current_surface_order
                        || (target.document_id == current_doc_id
                            && target.byte_range.end < caret_start)
                })
                .or_else(|| matches.last())
                .cloned()
        }
    }
}
