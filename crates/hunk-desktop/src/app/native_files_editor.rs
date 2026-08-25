use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;

use hunk_language::{HighlightCapture, LanguageRegistry};
use hunk_text::{BufferId, TextBuffer};

#[path = "native_files_editor_paint.rs"]
pub(crate) mod paint;
#[path = "native_files_editor_workspace_buffers.rs"]
mod workspace_buffers_impl;
#[path = "native_files_editor_workspace_display.rs"]
mod workspace_display_impl;
#[path = "native_files_editor_workspace.rs"]
mod workspace_session;
#[path = "native_files_editor_workspace_syntax.rs"]
mod workspace_syntax_impl;

use paint::RowSyntaxSpan;
#[cfg(test)]
pub(crate) use workspace_display_impl::WorkspaceProjectedRenderSnapshot;
pub(crate) use workspace_session::WorkspaceEditorSession;

pub(crate) type SharedFilesEditor = Rc<RefCell<FilesEditor>>;

pub(crate) struct FilesEditor {
    registry: LanguageRegistry,
    workspace_session: WorkspaceEditorSession,
    workspace_buffers: BTreeMap<PathBuf, TextBuffer>,
    workspace_syntax_by_path:
        BTreeMap<PathBuf, workspace_syntax_impl::WorkspaceDocumentSyntaxState>,
    search_query: Option<String>,
}

#[derive(Clone)]
struct VisibleHighlightCache {
    buffer_id: BufferId,
    buffer_version: u64,
    byte_range: Range<usize>,
    captures: Vec<HighlightCapture>,
}

impl FilesEditor {
    pub(crate) fn new() -> Self {
        Self {
            registry: LanguageRegistry::builtin(),
            workspace_session: WorkspaceEditorSession::new(),
            workspace_buffers: BTreeMap::new(),
            workspace_syntax_by_path: BTreeMap::new(),
            search_query: None,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.workspace_session.clear();
        self.workspace_buffers.clear();
        self.workspace_syntax_by_path.clear();
        self.search_query = None;
    }

    pub(crate) fn set_search_query(&mut self, query: Option<&str>) {
        self.search_query = query
            .map(str::trim)
            .filter(|query| !query.is_empty())
            .map(ToOwned::to_owned);
    }

    pub(crate) fn active_workspace_path_buf(&self) -> Option<PathBuf> {
        self.workspace_session.active_path_buf()
    }
}

fn compact_highlight_captures(captures: Vec<HighlightCapture>) -> Vec<HighlightCapture> {
    let mut compacted: Vec<HighlightCapture> = Vec::with_capacity(captures.len());
    for capture in captures {
        if let Some(previous) = compacted.last_mut()
            && previous.style_key == capture.style_key
            && previous.name == capture.name
            && previous.byte_range.end >= capture.byte_range.start
        {
            previous.byte_range.end = previous.byte_range.end.max(capture.byte_range.end);
            continue;
        }
        compacted.push(capture);
    }
    compacted
}
