use crate::app::markdown_links::{MarkdownLinkTarget, resolve_markdown_link_target};

impl DiffViewer {
    pub(super) fn activate_markdown_link(
        &mut self,
        raw_target: String,
        _window: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) -> bool {
        let workspace_root = if self.workspace_view_mode == WorkspaceViewMode::Ai {
            self.ai_workspace_cwd()
                .or_else(|| self.selected_git_workspace_root())
                .or_else(|| self.repo_root.clone())
        } else {
            self.selected_git_workspace_root()
                .or_else(|| self.repo_root.clone())
        };
        let Some(target) =
            resolve_markdown_link_target(raw_target.as_str(), workspace_root.as_deref(), None)
        else {
            return false;
        };

        match target {
            MarkdownLinkTarget::ExternalUrl(url) => match open_url_in_browser(url.as_str()) {
                Ok(()) => true,
                Err(err) => {
                    error!("failed to open markdown URL '{}': {err:#}", url);
                    Self::push_error_notification(
                        format!("Open URL failed: {}", err),
                        cx,
                    );
                    false
                }
            },
            MarkdownLinkTarget::WorkspaceFile(_) => false,
        }
    }
}
