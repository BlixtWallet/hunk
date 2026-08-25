use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use hunk_editor::{WorkspaceExcerptId, WorkspaceLayout};
use hunk_text::TextBuffer;

use super::FilesEditor;

impl FilesEditor {
    pub(crate) fn open_workspace_layout_documents(
        &mut self,
        layout: WorkspaceLayout,
        documents: Vec<(PathBuf, String)>,
        preferred_path: Option<&Path>,
    ) -> Result<()> {
        if documents.is_empty() {
            self.clear();
            return Ok(());
        }

        let buffer_id_by_path = layout
            .documents()
            .iter()
            .map(|document| (document.path().to_path_buf(), document.buffer_id))
            .collect::<BTreeMap<_, _>>();
        let workspace_buffers = documents
            .into_iter()
            .map(|(path, contents)| {
                let buffer_id = buffer_id_by_path.get(&path).copied().ok_or_else(|| {
                    anyhow!("missing workspace layout document for {}", path.display())
                })?;
                Ok((path, TextBuffer::new(buffer_id, contents.as_str())))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;

        validate_workspace_layout_buffers(&layout, &workspace_buffers)?;
        self.workspace_session
            .open_workspace_layout(layout, preferred_path);
        self.workspace_buffers = workspace_buffers;
        self.workspace_syntax_by_path.clear();
        Ok(())
    }

    pub(crate) fn activate_workspace_path(&mut self, path: &Path) -> Result<bool> {
        Ok(self.workspace_session.activate_path(path))
    }

    pub(crate) fn activate_workspace_excerpt(
        &mut self,
        excerpt_id: WorkspaceExcerptId,
    ) -> Result<bool> {
        Ok(self.workspace_session.activate_excerpt(excerpt_id))
    }
}

fn validate_workspace_layout_buffers(
    layout: &WorkspaceLayout,
    workspace_buffers: &BTreeMap<PathBuf, TextBuffer>,
) -> Result<()> {
    for document in layout.documents() {
        let buffer = workspace_buffers
            .get(document.path())
            .ok_or_else(|| anyhow!("missing workspace buffer for {}", document.path.display()))?;
        if buffer.id() != document.buffer_id {
            return Err(anyhow!(
                "workspace buffer id mismatch for {}: layout={} buffer={}",
                document.path.display(),
                document.buffer_id.get(),
                buffer.id().get(),
            ));
        }
        if buffer.line_count() != document.line_count {
            return Err(anyhow!(
                "workspace buffer line count mismatch for {}: layout={} buffer={}",
                document.path.display(),
                document.line_count,
                buffer.line_count(),
            ));
        }
    }
    Ok(())
}
