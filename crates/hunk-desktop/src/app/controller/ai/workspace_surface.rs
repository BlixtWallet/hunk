impl DiffViewer {
    fn sync_ai_workspace_session_for_timeline(
        &mut self,
        selected_thread_id: Option<&str>,
        visible_row_ids: &[String],
    ) {
        let Some(thread_id) = selected_thread_id else {
            self.ai_workspace_session = None;
            self.ai_workspace_selection = None;
            return;
        };
        let rebuild_started_at = std::time::Instant::now();
        let source_rows = visible_row_ids
            .iter()
            .filter_map(|row_id| {
                self.ai_timeline_row(row_id.as_str()).map(|row| {
                    ai_workspace_session::AiWorkspaceSourceRow {
                        row_id: row.id.clone(),
                        last_sequence: row.last_sequence,
                    }
                })
            })
            .collect::<Vec<_>>();

        if self
            .ai_workspace_session
            .as_ref()
            .is_some_and(|session| session.matches_source(thread_id, source_rows.as_slice()))
        {
            return;
        }

        let blocks = visible_row_ids
            .iter()
            .filter_map(|row_id| self.ai_workspace_block_for_row(row_id.as_str()))
            .collect::<Vec<_>>();
        if self
            .ai_workspace_selection
            .as_ref()
            .is_some_and(|selection| !blocks.iter().any(|block| block.id == selection.block_id))
        {
            self.ai_workspace_selection = None;
        }
        self.ai_workspace_session = Some(ai_workspace_session::AiWorkspaceSession::new(
            thread_id.to_string(),
            Arc::<[ai_workspace_session::AiWorkspaceSourceRow]>::from(source_rows),
            blocks,
        ));
        self.record_ai_workspace_session_rebuild_timing(rebuild_started_at.elapsed());
    }

    fn ai_workspace_block_for_row(
        &self,
        row_id: &str,
    ) -> Option<ai_workspace_session::AiWorkspaceBlock> {
        let row = self.ai_timeline_row(row_id)?;
        match &row.source {
            AiTimelineRowSource::Item { item_key } => {
                let item = self.ai_state_snapshot.items.get(item_key.as_str())?;
                let (kind, role) =
                    ai_workspace_block_kind_and_role_for_item_kind(item.kind.as_str());
                let preview = ai_workspace_item_preview_text(item);
                Some(ai_workspace_session::AiWorkspaceBlock {
                    id: row.id.clone(),
                    source_row_id: row.id.clone(),
                    role,
                    kind,
                    title: ai_workspace_item_title(item.kind.as_str()).to_string(),
                    preview,
                    last_sequence: row.last_sequence,
                })
            }
            AiTimelineRowSource::Group { group_id } => {
                let group = self.ai_timeline_group(group_id.as_str())?;
                Some(ai_workspace_session::AiWorkspaceBlock {
                    id: row.id.clone(),
                    source_row_id: row.id.clone(),
                    role: ai_workspace_session::AiWorkspaceBlockRole::Tool,
                    kind: ai_workspace_session::AiWorkspaceBlockKind::Group,
                    title: group.title.clone(),
                    preview: group
                        .summary
                        .as_deref()
                        .map(ai_workspace_preview_text)
                        .unwrap_or_default(),
                    last_sequence: row.last_sequence,
                })
            }
            AiTimelineRowSource::TurnDiff { turn_key } => {
                let diff = self.ai_state_snapshot.turn_diffs.get(turn_key.as_str())?;
                Some(ai_workspace_session::AiWorkspaceBlock {
                    id: row.id.clone(),
                    source_row_id: row.id.clone(),
                    role: ai_workspace_session::AiWorkspaceBlockRole::Tool,
                    kind: ai_workspace_session::AiWorkspaceBlockKind::DiffSummary,
                    title: "Code Changes".to_string(),
                    preview: ai_workspace_diff_preview(diff),
                    last_sequence: row.last_sequence,
                })
            }
            AiTimelineRowSource::TurnPlan { turn_key } => {
                let plan = self.ai_state_snapshot.turn_plans.get(turn_key.as_str())?;
                Some(ai_workspace_session::AiWorkspaceBlock {
                    id: row.id.clone(),
                    source_row_id: row.id.clone(),
                    role: ai_workspace_session::AiWorkspaceBlockRole::Assistant,
                    kind: ai_workspace_session::AiWorkspaceBlockKind::Plan,
                    title: "Plan".to_string(),
                    preview: ai_workspace_plan_preview(plan),
                    last_sequence: row.last_sequence,
                })
            }
        }
    }

    pub(super) fn ai_select_workspace_selection(
        &mut self,
        selection: ai_workspace_session::AiWorkspaceSelection,
        cx: &mut Context<Self>,
    ) {
        let block_kind = selection.block_kind;
        self.ai_workspace_selection = Some(selection);
        self.ai_text_selection = None;
        if block_kind == ai_workspace_session::AiWorkspaceBlockKind::DiffSummary {
            self.ai_open_review_tab(cx);
        }
        cx.notify();
    }

    pub(super) fn current_ai_workspace_surface_scroll_offset(&self) -> Point<Pixels> {
        if self.workspace_view_mode == WorkspaceViewMode::Ai && self.ai_workspace_session.is_some()
        {
            return self.ai_workspace_surface_scroll_handle.offset();
        }

        point(px(0.), px(0.))
    }

    pub(super) fn current_ai_workspace_surface_scroll_top_px(&self) -> usize {
        self.ai_workspace_surface_scroll_handle
            .offset()
            .y
            .min(Pixels::ZERO)
            .abs()
            .as_f32()
            .round() as usize
    }

    pub(super) fn refresh_ai_timeline_follow_output_from_surface_scroll(&mut self) {
        let block_count = self
            .ai_workspace_session
            .as_ref()
            .map(|session| session.block_count())
            .unwrap_or(0);
        let scroll_offset_y = self.ai_workspace_surface_scroll_handle.offset().y.as_f32();
        let max_scroll_offset_y = self
            .ai_workspace_surface_scroll_handle
            .max_offset()
            .y
            .max(Pixels::ZERO)
            .as_f32();
        self.ai_timeline_follow_output =
            should_follow_timeline_output(block_count, scroll_offset_y, max_scroll_offset_y);
    }
}

fn ai_workspace_block_kind_and_role_for_item_kind(
    kind: &str,
) -> (
    ai_workspace_session::AiWorkspaceBlockKind,
    ai_workspace_session::AiWorkspaceBlockRole,
) {
    match kind {
        "userMessage" => (
            ai_workspace_session::AiWorkspaceBlockKind::Message,
            ai_workspace_session::AiWorkspaceBlockRole::User,
        ),
        "agentMessage" => (
            ai_workspace_session::AiWorkspaceBlockKind::Message,
            ai_workspace_session::AiWorkspaceBlockRole::Assistant,
        ),
        "reasoning" => (
            ai_workspace_session::AiWorkspaceBlockKind::Status,
            ai_workspace_session::AiWorkspaceBlockRole::Assistant,
        ),
        "plan" => (
            ai_workspace_session::AiWorkspaceBlockKind::Plan,
            ai_workspace_session::AiWorkspaceBlockRole::Assistant,
        ),
        "webSearch"
        | "dynamicToolCall"
        | "mcpToolCall"
        | "collabAgentToolCall"
        | "commandExecution"
        | "fileChange" => (
            ai_workspace_session::AiWorkspaceBlockKind::Tool,
            ai_workspace_session::AiWorkspaceBlockRole::Tool,
        ),
        _ => (
            ai_workspace_session::AiWorkspaceBlockKind::Status,
            ai_workspace_session::AiWorkspaceBlockRole::System,
        ),
    }
}

fn ai_workspace_item_title(kind: &str) -> &'static str {
    match kind {
        "userMessage" => "You",
        "agentMessage" => "Assistant",
        "reasoning" => "Thinking",
        "plan" => "Plan",
        "webSearch" => "Search",
        "dynamicToolCall" | "mcpToolCall" | "collabAgentToolCall" => "Tool",
        "commandExecution" => "Command",
        "fileChange" => "Code Changes",
        _ => "Update",
    }
}

fn ai_workspace_item_preview_text(item: &hunk_codex::state::ItemSummary) -> String {
    item.display_metadata
        .as_ref()
        .and_then(|metadata| metadata.summary.as_deref())
        .map(ai_workspace_preview_text)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            (!item.content.trim().is_empty())
                .then(|| ai_workspace_preview_text(item.content.as_str()))
        })
        .unwrap_or_else(|| ai_workspace_item_title(item.kind.as_str()).to_string())
}

fn ai_workspace_plan_preview(plan: &hunk_codex::state::TurnPlanSummary) -> String {
    plan.explanation
        .as_deref()
        .map(ai_workspace_preview_text)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| match plan.steps.len() {
            0 => "Plan pending".to_string(),
            1 => "1 planned step".to_string(),
            count => format!("{count} planned steps"),
        })
}

fn ai_workspace_diff_preview(diff: &str) -> String {
    let mut file_count = 0usize;
    let mut additions = 0usize;
    let mut removals = 0usize;
    for line in diff.lines() {
        if line.starts_with("diff --git ") {
            file_count = file_count.saturating_add(1);
        } else if line.starts_with('+') && !line.starts_with("+++") {
            additions = additions.saturating_add(1);
        } else if line.starts_with('-') && !line.starts_with("---") {
            removals = removals.saturating_add(1);
        }
    }

    match (file_count, additions, removals) {
        (0, 0, 0) => "Diff ready".to_string(),
        (0, adds, removes) => format!("{adds} additions, {removes} removals"),
        (1, adds, removes) => format!("1 file changed, +{adds} -{removes}"),
        (files, adds, removes) => format!("{files} files changed, +{adds} -{removes}"),
    }
}

fn ai_workspace_preview_text(value: &str) -> String {
    let normalized = value
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .unwrap_or_default();
    truncate_ai_workspace_preview(normalized.as_str(), 180)
}

fn truncate_ai_workspace_preview(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        return value.to_string();
    }

    let mut end = max_len;
    while !value.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    let trimmed = value[..end].trim_end();
    format!("{trimmed}...")
}
