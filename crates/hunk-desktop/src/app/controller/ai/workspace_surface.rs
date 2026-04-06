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
            .filter_map(|row_id| self.ai_workspace_source_row(row_id.as_str()))
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
            .flat_map(|row_id| self.ai_workspace_blocks_for_row(row_id.as_str()))
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

    fn ai_workspace_blocks_for_row(
        &self,
        row_id: &str,
    ) -> Vec<ai_workspace_session::AiWorkspaceBlock> {
        if let Some(pending) = self.ai_pending_steer_for_row_id(row_id) {
            return vec![ai_workspace_session::AiWorkspaceBlock {
                id: row_id.to_string(),
                source_row_id: row_id.to_string(),
                role: ai_workspace_session::AiWorkspaceBlockRole::User,
                kind: ai_workspace_session::AiWorkspaceBlockKind::Message,
                nested: false,
                mono_preview: false,
                open_review_tab: false,
                expandable: false,
                expanded: true,
                title: "You".to_string(),
                preview: ai_workspace_prompt_preview(
                    pending.prompt.as_str(),
                    pending.local_images.as_slice(),
                ),
                copy_text: Some(ai_workspace_prompt_preview(
                    pending.prompt.as_str(),
                    pending.local_images.as_slice(),
                )),
                copy_tooltip: Some("Copy message"),
                copy_success_message: Some("Copied message."),
                last_sequence: ai_workspace_pending_steer_signature(&pending),
            }];
        }
        if let Some(queued) = self.ai_queued_message_for_row_id(row_id) {
            let preview = ai_workspace_prompt_preview(
                queued.prompt.as_str(),
                queued.local_images.as_slice(),
            );
            return vec![ai_workspace_session::AiWorkspaceBlock {
                id: row_id.to_string(),
                source_row_id: row_id.to_string(),
                role: ai_workspace_session::AiWorkspaceBlockRole::User,
                kind: ai_workspace_session::AiWorkspaceBlockKind::Message,
                nested: false,
                mono_preview: false,
                open_review_tab: false,
                expandable: false,
                expanded: true,
                title: match queued.status {
                    AiQueuedUserMessageStatus::Queued => "Queued".to_string(),
                    AiQueuedUserMessageStatus::PendingConfirmation { .. } => {
                        "Pending Confirmation".to_string()
                    }
                },
                preview: preview.clone(),
                copy_text: Some(preview),
                copy_tooltip: Some("Copy message"),
                copy_success_message: Some("Copied message."),
                last_sequence: ai_workspace_queued_message_signature(&queued),
            }];
        }

        let Some(row) = self.ai_timeline_row(row_id) else {
            return Vec::new();
        };
        match &row.source {
            AiTimelineRowSource::Item { item_key } => {
                self.ai_state_snapshot
                    .items
                    .get(item_key.as_str())
                    .and_then(|item| self.ai_workspace_block_for_item_row(row, item, false))
                    .into_iter()
                    .collect()
            }
            AiTimelineRowSource::Group { group_id } => {
                self.ai_timeline_group(group_id.as_str())
                    .map(|group| self.ai_workspace_blocks_for_group_row(row, group))
                    .unwrap_or_default()
            }
            AiTimelineRowSource::TurnDiff { turn_key } => {
                self.ai_state_snapshot
                    .turn_diffs
                    .get(turn_key.as_str())
                    .map(|diff| vec![ai_workspace_diff_block(
                        row.id.clone(),
                        row.id.clone(),
                        row.last_sequence,
                        &crate::app::ai_workspace_timeline_projection::ai_workspace_turn_diff_summary(
                            diff,
                        ),
                        false,
                    )])
                    .unwrap_or_default()
            }
            AiTimelineRowSource::TurnPlan { turn_key } => {
                self.ai_state_snapshot
                    .turn_plans
                    .get(turn_key.as_str())
                    .map(|plan| vec![ai_workspace_session::AiWorkspaceBlock {
                    id: row.id.clone(),
                    source_row_id: row.id.clone(),
                    role: ai_workspace_session::AiWorkspaceBlockRole::Assistant,
                    kind: ai_workspace_session::AiWorkspaceBlockKind::Plan,
                    nested: false,
                    mono_preview: false,
                    open_review_tab: false,
                    expandable: false,
                    expanded: true,
                    title: "Updated Plan".to_string(),
                    preview: ai_workspace_plan_preview(plan),
                    copy_text: None,
                    copy_tooltip: None,
                    copy_success_message: None,
                    last_sequence: row.last_sequence,
                    }])
                    .unwrap_or_default()
            }
        }
    }

    pub(super) fn ai_select_workspace_selection(
        &mut self,
        selection: ai_workspace_session::AiWorkspaceSelection,
        cx: &mut Context<Self>,
    ) {
        self.ai_workspace_selection = Some(selection);
        self.ai_text_selection = None;
        cx.notify();
    }

    fn ai_workspace_selected_block(&self) -> Option<&ai_workspace_session::AiWorkspaceBlock> {
        let selection = self.ai_workspace_selection.as_ref()?;
        self.ai_workspace_session
            .as_ref()
            .and_then(|session| session.block(selection.block_id.as_str()))
    }

    pub(super) fn current_ai_workspace_selected_text(&self) -> Option<String> {
        let block = self.ai_workspace_selected_block()?;
        let mut sections = Vec::with_capacity(2);
        if !block.title.trim().is_empty() {
            sections.push(block.title.trim().to_string());
        }
        if !block.preview.trim().is_empty() {
            sections.push(block.preview.trim().to_string());
        }
        (!sections.is_empty()).then(|| sections.join("\n"))
    }

    pub(super) fn ai_select_all_workspace_block_text(&mut self, cx: &mut Context<Self>) -> bool {
        let Some((block_id, surfaces)) = self.ai_workspace_selected_block().map(|block| {
            (block.id.clone(), ai_workspace_selection_surfaces(block))
        }) else {
            return false;
        };
        if surfaces.is_empty() {
            return false;
        }

        self.ai_select_all_text_for_surfaces(block_id.as_str(), surfaces, cx)
    }

    pub(super) fn ai_move_workspace_selection_by(
        &mut self,
        delta: isize,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(next_selection) = self.ai_workspace_session.as_ref().and_then(|session| {
            if session.block_count() == 0 {
                return None;
            }

            let current_index = self
                .ai_workspace_selection
                .as_ref()
                .and_then(|selection| session.block_index(selection.block_id.as_str()));
            let next_index =
                ai_workspace_selection_index(current_index, session.block_count(), delta)?;
            let block = session.block_at(next_index)?;
            Some(ai_workspace_session::AiWorkspaceSelection {
                block_id: block.id.clone(),
                block_kind: block.kind,
                line_index: None,
                region: ai_workspace_session::AiWorkspaceSelectionRegion::Block,
            })
        }) else {
            return false;
        };

        let selected_block_id = next_selection.block_id.clone();
        self.ai_select_workspace_selection(next_selection, cx);
        self.ai_reveal_workspace_block_if_needed(selected_block_id.as_str());
        true
    }

    fn ai_workspace_source_row(
        &self,
        row_id: &str,
    ) -> Option<ai_workspace_session::AiWorkspaceSourceRow> {
        if let Some(row) = self.ai_timeline_row(row_id) {
            let last_sequence = match &row.source {
                AiTimelineRowSource::Group { group_id } => self
                    .ai_timeline_group(group_id.as_str())
                    .map(|group| self.ai_workspace_group_source_signature(row, group))
                    .unwrap_or_else(|| {
                        ai_workspace_row_signature(
                            row.last_sequence,
                            self.ai_workspace_row_is_expanded(row.id.as_str()),
                        )
                    }),
                _ => ai_workspace_row_signature(
                    row.last_sequence,
                    self.ai_workspace_row_is_expanded(row.id.as_str()),
                ),
            };
            return Some(ai_workspace_session::AiWorkspaceSourceRow {
                row_id: row.id.clone(),
                last_sequence,
            });
        }
        if let Some(pending) = self.ai_pending_steer_for_row_id(row_id) {
            return Some(ai_workspace_session::AiWorkspaceSourceRow {
                row_id: row_id.to_string(),
                last_sequence: ai_workspace_pending_steer_signature(&pending),
            });
        }
        if let Some(queued) = self.ai_queued_message_for_row_id(row_id) {
            return Some(ai_workspace_session::AiWorkspaceSourceRow {
                row_id: row_id.to_string(),
                last_sequence: ai_workspace_queued_message_signature(&queued),
            });
        }

        None
    }

    pub(super) fn current_ai_inline_review_row_id_for_thread(
        &self,
        thread_id: &str,
    ) -> Option<&str> {
        self.ai_inline_review_selected_row_id_by_thread
            .get(thread_id)
            .map(String::as_str)
    }

    pub(super) fn ai_inline_review_is_open(&self) -> bool {
        self.ai_selected_thread_id
            .as_deref()
            .and_then(|thread_id| self.current_ai_inline_review_row_id_for_thread(thread_id))
            .is_some()
    }

    #[allow(dead_code)]
    pub(super) fn ai_open_inline_review_for_row(&mut self, row_id: String, cx: &mut Context<Self>) {
        let Some(thread_id) = self.ai_selected_thread_id.clone() else {
            return;
        };
        let Some(row) = self.ai_timeline_row(row_id.as_str()) else {
            return;
        };
        if !matches!(row.source, AiTimelineRowSource::TurnDiff { .. }) {
            return;
        }

        self.ai_inline_review_selected_row_id_by_thread
            .insert(thread_id, row_id);
        self.ai_sync_review_compare_to_selected_thread(cx);
        self.invalidate_ai_visible_frame_state_with_reason("timeline");
        cx.notify();
    }

    pub(super) fn ai_close_inline_review_action(&mut self, cx: &mut Context<Self>) {
        let Some(thread_id) = self.ai_selected_thread_id.as_deref() else {
            return;
        };
        if self
            .ai_inline_review_selected_row_id_by_thread
            .remove(thread_id)
            .is_some()
        {
            self.invalidate_ai_visible_frame_state_with_reason("timeline");
            cx.notify();
        }
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

    fn ai_reveal_workspace_block_if_needed(&mut self, block_id: &str) {
        let viewport_bounds = self.ai_workspace_surface_scroll_handle.bounds();
        let viewport_height_px = viewport_bounds
            .size
            .height
            .max(Pixels::ZERO)
            .as_f32()
            .round() as usize;
        let viewport_width_px = viewport_bounds
            .size
            .width
            .max(Pixels::ZERO)
            .as_f32()
            .round() as usize;
        let scroll_top_px = self.current_ai_workspace_surface_scroll_top_px();
        let Some(geometry) = self
            .ai_workspace_session
            .as_mut()
            .and_then(|session| session.block_geometry(block_id, viewport_width_px.max(1)))
        else {
            return;
        };

        let viewport_bottom_px = scroll_top_px.saturating_add(viewport_height_px);
        let next_scroll_top_px = if geometry.top_px < scroll_top_px {
            Some(geometry.top_px)
        } else if geometry.bottom_px() > viewport_bottom_px {
            Some(geometry.bottom_px().saturating_sub(viewport_height_px))
        } else {
            None
        };
        let Some(next_scroll_top_px) = next_scroll_top_px else {
            return;
        };

        self.ai_workspace_surface_scroll_handle
            .set_offset(point(px(0.), -px(next_scroll_top_px as f32)));
        self.refresh_ai_timeline_follow_output_from_surface_scroll();
    }

    pub(super) fn ai_workspace_toggle_row_expansion(
        &mut self,
        row_id: String,
        cx: &mut Context<Self>,
    ) {
        self.ai_toggle_timeline_row_expansion_action(row_id, cx);
    }

    fn ai_workspace_row_is_expanded(&self, row_id: &str) -> bool {
        self.ai_expanded_timeline_row_ids.contains(row_id)
    }
}

impl DiffViewer {
    fn ai_workspace_block_for_item_row(
        &self,
        row: &AiTimelineRow,
        item: &hunk_codex::state::ItemSummary,
        nested: bool,
    ) -> Option<ai_workspace_session::AiWorkspaceBlock> {
        let expanded = self.ai_workspace_row_is_expanded(row.id.as_str());
        match item.kind.as_str() {
            "userMessage" | "agentMessage" => {
                let preview = ai_workspace_message_preview(item);
                Some(ai_workspace_session::AiWorkspaceBlock {
                id: row.id.clone(),
                source_row_id: row.id.clone(),
                role: if item.kind == "userMessage" {
                    ai_workspace_session::AiWorkspaceBlockRole::User
                } else {
                    ai_workspace_session::AiWorkspaceBlockRole::Assistant
                },
                kind: ai_workspace_session::AiWorkspaceBlockKind::Message,
                nested,
                mono_preview: false,
                open_review_tab: false,
                expandable: false,
                expanded: true,
                title: if item.kind == "userMessage" {
                    "You".to_string()
                } else {
                    "Assistant".to_string()
                },
                preview: preview.clone(),
                copy_text: Some(preview),
                copy_tooltip: Some("Copy message"),
                copy_success_message: Some("Copied message."),
                last_sequence: row.last_sequence,
                })
            }
            "fileChange" => crate::app::ai_workspace_timeline_projection::ai_workspace_file_change_summary(item)
                .map(|summary| {
                    ai_workspace_diff_block(
                        row.id.clone(),
                        row.id.clone(),
                        row.last_sequence,
                        &summary,
                        nested,
                    )
                }),
            "commandExecution" => {
                let raw_content_text = item.content.trim_end();
                let command_details =
                    crate::app::ai_workspace_timeline_projection::ai_workspace_command_execution_display_details(item);
                let has_details = command_details.is_some() || !raw_content_text.is_empty();
                let title = ai_workspace_tool_header_line(item, raw_content_text);
                let preview = if expanded {
                    command_details
                        .as_ref()
                        .map(|details| {
                            crate::app::ai_workspace_timeline_projection::ai_workspace_command_execution_terminal_text(
                                details,
                                raw_content_text,
                                Some(
                                    crate::app::ai_workspace_timeline_projection::AI_WORKSPACE_COMMAND_PREVIEW_MAX_OUTPUT_LINES,
                                ),
                            )
                            .0
                        })
                        .unwrap_or_else(|| ai_workspace_expanded_tool_text(raw_content_text))
                } else {
                    String::new()
                };
                let copy_text = expanded.then_some(preview.clone());
                Some(ai_workspace_session::AiWorkspaceBlock {
                    id: row.id.clone(),
                    source_row_id: row.id.clone(),
                    role: ai_workspace_session::AiWorkspaceBlockRole::Tool,
                    kind: ai_workspace_session::AiWorkspaceBlockKind::Tool,
                    nested,
                    mono_preview: true,
                    open_review_tab: false,
                    expandable: has_details,
                    expanded,
                    title,
                    preview,
                    copy_text,
                    copy_tooltip: expanded.then_some("Copy command transcript"),
                    copy_success_message: expanded.then_some("Copied command transcript."),
                    last_sequence: row.last_sequence,
                })
            }
            "reasoning" | "webSearch" | "dynamicToolCall" | "mcpToolCall"
            | "collabAgentToolCall" => {
                let details_text = crate::app::ai_workspace_timeline_projection::ai_workspace_timeline_item_details_json(item)
                    .unwrap_or(item.content.as_str());
                let details_text = details_text.trim();
                let preview = if expanded {
                    ai_workspace_expanded_tool_text(details_text)
                } else {
                    String::new()
                };
                let has_details = !details_text.is_empty();
                Some(ai_workspace_session::AiWorkspaceBlock {
                    id: row.id.clone(),
                    source_row_id: row.id.clone(),
                    role: if item.kind == "reasoning" || item.kind == "webSearch" {
                        ai_workspace_session::AiWorkspaceBlockRole::Assistant
                    } else if item.kind == "dynamicToolCall"
                        || item.kind == "mcpToolCall"
                        || item.kind == "collabAgentToolCall"
                    {
                        ai_workspace_session::AiWorkspaceBlockRole::Tool
                    } else {
                        ai_workspace_session::AiWorkspaceBlockRole::System
                    },
                    kind: if item.kind == "reasoning" {
                        ai_workspace_session::AiWorkspaceBlockKind::Status
                    } else {
                        ai_workspace_session::AiWorkspaceBlockKind::Tool
                    },
                    nested,
                    mono_preview: item.kind != "reasoning" && item.kind != "webSearch",
                    open_review_tab: false,
                    expandable: has_details,
                    expanded,
                    title: ai_workspace_tool_header_line(item, item.content.trim()),
                    preview,
                    copy_text: None,
                    copy_tooltip: None,
                    copy_success_message: None,
                    last_sequence: row.last_sequence,
                })
            }
            _ => Some(ai_workspace_session::AiWorkspaceBlock {
                id: row.id.clone(),
                source_row_id: row.id.clone(),
                role: ai_workspace_session::AiWorkspaceBlockRole::System,
                kind: ai_workspace_session::AiWorkspaceBlockKind::Status,
                nested,
                mono_preview: false,
                open_review_tab: false,
                expandable: false,
                expanded: true,
                title: ai_workspace_tool_header_line(item, item.content.trim()),
                preview: String::new(),
                copy_text: None,
                copy_tooltip: None,
                copy_success_message: None,
                last_sequence: row.last_sequence,
            }),
        }
    }

    fn ai_workspace_blocks_for_group_row(
        &self,
        row: &AiTimelineRow,
        group: &AiTimelineGroup,
    ) -> Vec<ai_workspace_session::AiWorkspaceBlock> {
        if group.kind == "file_change_batch"
            && let Some(summary) = ai_workspace_file_change_group_summary(self, group)
        {
            return vec![ai_workspace_diff_block(
                row.id.clone(),
                row.id.clone(),
                row.last_sequence,
                &summary,
                false,
            )];
        }

        let expanded = self.ai_workspace_row_is_expanded(row.id.as_str());
        let mut blocks = vec![ai_workspace_session::AiWorkspaceBlock {
            id: row.id.clone(),
            source_row_id: row.id.clone(),
            role: ai_workspace_session::AiWorkspaceBlockRole::Tool,
            kind: ai_workspace_session::AiWorkspaceBlockKind::Group,
            nested: false,
            mono_preview: false,
            open_review_tab: false,
            expandable: true,
            expanded,
            title: crate::app::ai_workspace_timeline_projection::ai_workspace_format_header_line(
                group.title.as_str(),
                group.summary.as_deref(),
                None,
            ),
            preview: String::new(),
            copy_text: None,
            copy_tooltip: None,
            copy_success_message: None,
            last_sequence: row.last_sequence,
        }];

        if !expanded {
            return blocks;
        }

        blocks.extend(group.child_row_ids.iter().filter_map(|child_row_id| {
            let child_row = self.ai_timeline_row(child_row_id.as_str())?;
            let AiTimelineRowSource::Item { item_key } = &child_row.source else {
                return None;
            };
            let item = self.ai_state_snapshot.items.get(item_key.as_str())?;
            self.ai_workspace_block_for_item_row(child_row, item, true)
        }));
        blocks
    }

    fn ai_workspace_group_source_signature(
        &self,
        row: &AiTimelineRow,
        group: &AiTimelineGroup,
    ) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(
            &ai_workspace_row_signature(
                row.last_sequence,
                self.ai_workspace_row_is_expanded(row.id.as_str()),
            ),
            &mut hasher,
        );

        if self.ai_workspace_row_is_expanded(row.id.as_str()) {
            for child_row_id in &group.child_row_ids {
                if let Some(child_row) = self.ai_timeline_row(child_row_id.as_str()) {
                    std::hash::Hash::hash(
                        &ai_workspace_row_signature(
                            child_row.last_sequence,
                            self.ai_workspace_row_is_expanded(child_row.id.as_str()),
                        ),
                        &mut hasher,
                    );
                }
            }
        }

        std::hash::Hasher::finish(&hasher)
    }
}

fn ai_workspace_message_preview(item: &hunk_codex::state::ItemSummary) -> String {
    item.content
        .trim()
        .is_empty()
        .then(|| {
            item.display_metadata
                .as_ref()
                .and_then(|metadata| metadata.summary.as_deref())
                .map(ai_workspace_full_preview_text)
        })
        .flatten()
        .unwrap_or_else(|| ai_workspace_full_preview_text(item.content.as_str()))
}

fn ai_workspace_tool_header_line(
    item: &hunk_codex::state::ItemSummary,
    content_text: &str,
) -> String {
    let title = crate::app::ai_workspace_timeline_projection::ai_workspace_tool_header_title(item);
    let summary = crate::app::ai_workspace_timeline_projection::ai_workspace_tool_compact_summary(
        item,
        content_text,
    );
    let status = (item.status != hunk_codex::state::ItemStatus::Completed).then(|| {
        crate::app::ai_workspace_timeline_projection::ai_workspace_item_status_label(item.status)
    });

    crate::app::ai_workspace_timeline_projection::ai_workspace_format_header_line(
        title.as_str(),
        summary.as_deref(),
        status,
    )
}

fn ai_workspace_plan_preview(plan: &hunk_codex::state::TurnPlanSummary) -> String {
    let mut sections = Vec::new();
    if let Some(explanation) = plan
        .explanation
        .as_deref()
        .map(ai_workspace_full_preview_text)
        .filter(|value| !value.is_empty())
    {
        sections.push(explanation);
    }
    if !plan.steps.is_empty() {
        sections.extend(plan.steps.iter().map(|step| {
            format!(
                "{} {}",
                ai_workspace_plan_step_marker(step.status),
                step.step.trim()
            )
        }));
    }

    if sections.is_empty() {
        "Plan pending".to_string()
    } else {
        sections.join("\n")
    }
}

fn ai_workspace_diff_block(
    block_id: String,
    source_row_id: String,
    last_sequence: u64,
    summary: &crate::app::ai_workspace_timeline_projection::AiWorkspaceDiffSummary,
    nested: bool,
) -> ai_workspace_session::AiWorkspaceBlock {
    let preview =
        crate::app::ai_workspace_timeline_projection::ai_workspace_diff_summary_preview(summary);
    let mut preview_lines = preview.lines();
    let title = preview_lines
        .next()
        .map(str::to_string)
        .unwrap_or_else(|| "Code Changes".to_string());
    let preview = preview_lines.collect::<Vec<_>>().join("\n");

    ai_workspace_session::AiWorkspaceBlock {
        id: block_id,
        source_row_id,
        role: ai_workspace_session::AiWorkspaceBlockRole::Tool,
        kind: ai_workspace_session::AiWorkspaceBlockKind::DiffSummary,
        nested,
        mono_preview: false,
        open_review_tab: true,
        expandable: false,
        expanded: false,
        title,
        preview,
        copy_text: None,
        copy_tooltip: None,
        copy_success_message: None,
        last_sequence,
    }
}

fn ai_workspace_file_change_group_summary(
    this: &DiffViewer,
    group: &AiTimelineGroup,
) -> Option<crate::app::ai_workspace_timeline_projection::AiWorkspaceDiffSummary> {
    let mut summary = crate::app::ai_workspace_timeline_projection::AiWorkspaceDiffSummary {
        files: Vec::new(),
        total_added: 0,
        total_removed: 0,
    };

    for child_row_id in &group.child_row_ids {
        let row = this.ai_timeline_row(child_row_id.as_str())?;
        let AiTimelineRowSource::Item { item_key } = &row.source else {
            continue;
        };
        let item = this.ai_state_snapshot.items.get(item_key.as_str())?;
        let item_summary =
            crate::app::ai_workspace_timeline_projection::ai_workspace_file_change_summary(item)?;
        for file in item_summary.files {
            summary.total_added = summary.total_added.saturating_add(file.added);
            summary.total_removed = summary.total_removed.saturating_add(file.removed);
            if let Some(existing) = summary.files.iter_mut().find(|entry| entry.path == file.path) {
                existing.added = existing.added.saturating_add(file.added);
                existing.removed = existing.removed.saturating_add(file.removed);
            } else {
                summary.files.push(file);
            }
        }
    }

    (!summary.files.is_empty()).then_some(summary)
}

fn ai_workspace_full_preview_text(value: &str) -> String {
    let normalized = value
        .replace("\r\n", "\n")
        .lines()
        .take(160)
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    truncate_ai_workspace_preview(normalized.as_str(), 12_000)
}

fn ai_workspace_expanded_tool_text(value: &str) -> String {
    let normalized = value
        .replace("\r\n", "\n")
        .lines()
        .take(96)
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    truncate_ai_workspace_preview(normalized.as_str(), 8_000)
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

fn ai_workspace_prompt_preview(prompt: &str, local_images: &[PathBuf]) -> String {
    let prompt = prompt.trim();
    let image_names = local_images
        .iter()
        .map(|path| ai_pending_steer_local_image_name(path.as_path()))
        .collect::<Vec<_>>();

    let mut content = String::new();
    if !prompt.is_empty() {
        content.push_str(prompt);
    }
    if !image_names.is_empty() {
        if !content.is_empty() {
            content.push('\n');
        }
        let prefix = if image_names.len() == 1 {
            "[image] "
        } else {
            "[images] "
        };
        content.push_str(prefix);
        content.push_str(image_names.join(", ").as_str());
    }
    if content.is_empty() {
        return "Message pending".to_string();
    }

    ai_workspace_full_preview_text(content.as_str())
}

fn ai_workspace_selection_surfaces(
    block: &ai_workspace_session::AiWorkspaceBlock,
) -> Arc<[AiTextSelectionSurfaceSpec]> {
    let mut surfaces = Vec::with_capacity(2);
    if !block.title.is_empty() {
        surfaces.push(AiTextSelectionSurfaceSpec::new(
            format!("ai-workspace:{}:title", block.id),
            block.title.clone(),
        ));
    }
    if !block.preview.is_empty() {
        let surface = AiTextSelectionSurfaceSpec::new(
            format!("ai-workspace:{}:preview", block.id),
            block.preview.clone(),
        );
        surfaces.push(if surfaces.is_empty() {
            surface
        } else {
            surface.with_separator_before("\n")
        });
    }

    Arc::<[AiTextSelectionSurfaceSpec]>::from(surfaces)
}

fn ai_workspace_selection_index(
    current_index: Option<usize>,
    block_count: usize,
    delta: isize,
) -> Option<usize> {
    if block_count == 0 {
        return None;
    }

    let baseline = current_index.unwrap_or_else(|| {
        if delta.is_negative() {
            block_count.saturating_sub(1)
        } else {
            0
        }
    });
    let next_index = baseline.saturating_add_signed(delta);
    Some(next_index.min(block_count.saturating_sub(1)))
}

fn ai_workspace_pending_steer_signature(pending: &AiPendingSteer) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&pending.thread_id, &mut hasher);
    std::hash::Hash::hash(&pending.turn_id, &mut hasher);
    std::hash::Hash::hash(&pending.prompt, &mut hasher);
    std::hash::Hash::hash(&pending.accepted_after_sequence, &mut hasher);
    for image in &pending.local_images {
        std::hash::Hash::hash(&image, &mut hasher);
    }
    std::hash::Hasher::finish(&hasher)
}

fn ai_workspace_queued_message_signature(queued: &AiQueuedUserMessage) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&queued.thread_id, &mut hasher);
    std::hash::Hash::hash(&queued.prompt, &mut hasher);
    for image in &queued.local_images {
        std::hash::Hash::hash(&image, &mut hasher);
    }
    match queued.status {
        AiQueuedUserMessageStatus::Queued => std::hash::Hash::hash(&0u64, &mut hasher),
        AiQueuedUserMessageStatus::PendingConfirmation {
            accepted_after_sequence,
        } => std::hash::Hash::hash(&accepted_after_sequence, &mut hasher),
    }
    std::hash::Hasher::finish(&hasher)
}

fn ai_workspace_row_signature(last_sequence: u64, expanded: bool) -> u64 {
    last_sequence
        .wrapping_shl(1)
        .wrapping_add(u64::from(expanded))
}

fn ai_workspace_plan_step_marker(status: hunk_codex::state::TurnPlanStepStatus) -> &'static str {
    match status {
        hunk_codex::state::TurnPlanStepStatus::Pending => "[ ]",
        hunk_codex::state::TurnPlanStepStatus::InProgress => "[>]",
        hunk_codex::state::TurnPlanStepStatus::Completed => "[x]",
    }
}
