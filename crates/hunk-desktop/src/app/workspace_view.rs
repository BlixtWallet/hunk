pub(crate) const SHORTCUT_CONTEXT_REVIEW_WORKSPACE: &str = "ReviewWorkspace";
pub(crate) const SHORTCUT_CONTEXT_GIT_WORKSPACE: &str = "GitWorkspace";
pub(crate) const SHORTCUT_CONTEXT_AI_WORKSPACE: &str = "AiWorkspace";
pub(crate) const SHORTCUT_CONTEXT_TREE_WORKSPACE: &str = "TreeWorkspace";
pub(crate) const SHORTCUT_CONTEXT_SELECTABLE_WORKSPACE: &str = "SelectableWorkspace";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkspaceViewMode {
    Diff,
    GitWorkspace,
    Ai,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkspaceSidebarKind {
    Review,
    AiThreads,
}

impl WorkspaceSidebarKind {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Review => "changed files",
            Self::AiThreads => "threads",
        }
    }

    pub(crate) const fn uses_changed_files(self) -> bool {
        matches!(self, Self::Review)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkspaceSwitchAction {
    Review,
    Git,
    Ai,
}

impl WorkspaceViewMode {
    pub(crate) const fn collapsible_sidebar_kind(self) -> Option<WorkspaceSidebarKind> {
        match self {
            Self::Diff => Some(WorkspaceSidebarKind::Review),
            Self::Ai => Some(WorkspaceSidebarKind::AiThreads),
            Self::GitWorkspace => None,
        }
    }

    pub(super) const fn supports_sidebar_tree(self) -> bool {
        matches!(self, Self::Diff)
    }

    pub(super) const fn supports_diff_stream(self) -> bool {
        matches!(self, Self::Diff)
    }

    pub(super) const fn shows_toolbar_workspace_identity(self) -> bool {
        !matches!(self, Self::Ai)
    }

    pub(super) const fn shows_toolbar_change_summary(self) -> bool {
        matches!(self, Self::Diff)
    }

    pub(crate) const fn shortcut_context(self) -> &'static str {
        match self {
            Self::Diff => SHORTCUT_CONTEXT_REVIEW_WORKSPACE,
            Self::GitWorkspace => SHORTCUT_CONTEXT_GIT_WORKSPACE,
            Self::Ai => SHORTCUT_CONTEXT_AI_WORKSPACE,
        }
    }

    pub(crate) const fn root_key_context(self) -> &'static str {
        match self {
            Self::Diff => "DiffViewer ReviewWorkspace TreeWorkspace SelectableWorkspace",
            Self::GitWorkspace => "DiffViewer GitWorkspace",
            Self::Ai => "DiffViewer AiWorkspace SelectableWorkspace",
        }
    }
}

impl WorkspaceSwitchAction {
    pub(super) const fn target_mode(self) -> WorkspaceViewMode {
        match self {
            Self::Review => WorkspaceViewMode::Diff,
            Self::Git => WorkspaceViewMode::GitWorkspace,
            Self::Ai => WorkspaceViewMode::Ai,
        }
    }
}
