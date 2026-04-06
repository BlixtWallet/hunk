struct AiWorkspaceOverlayCopyButton {
    id: String,
    left_px: usize,
    top_px: usize,
    tooltip: &'static str,
    text: String,
    success_message: &'static str,
    message_copy: bool,
}

impl DiffViewer {
    fn current_ai_workspace_surface_snapshot(
        &mut self,
    ) -> Option<ai_workspace_session::AiWorkspaceSurfaceSnapshot> {
        let scroll_top_px = self.current_ai_workspace_surface_scroll_top_px();
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
        let snapshot_result = {
            let session = self.ai_workspace_session.as_mut()?;
            session.surface_snapshot_with_stats(
                scroll_top_px,
                viewport_height_px.max(1),
                viewport_width_px.max(1),
            )
        };
        if let Some(duration) = snapshot_result.geometry_rebuild_duration {
            self.record_ai_workspace_surface_geometry_rebuild_timing(duration);
        }
        Some(snapshot_result.snapshot)
    }

    fn render_ai_workspace_surface_scroller(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let viewport_bounds = self.ai_workspace_surface_scroll_handle.bounds();
        let viewport_width_px = viewport_bounds
            .size
            .width
            .max(Pixels::ZERO)
            .as_f32()
            .round() as usize;
        let surface = self.current_ai_workspace_surface_snapshot()?;
        let scroll_handle = self.ai_workspace_surface_scroll_handle.clone();
        let viewport_height_px = surface.viewport.total_surface_height_px;
        let workspace_root = self
            .ai_workspace_cwd()
            .or_else(|| self.selected_git_workspace_root())
            .or_else(|| self.repo_root.clone());

        Some(
            div()
                .id("ai-workspace-surface-scroll")
                .size_full()
                .track_scroll(&scroll_handle)
                .overflow_y_scroll()
                .child(
                    div()
                        .relative()
                        .w_full()
                        .h(px(viewport_height_px as f32))
                        .when_some(surface.viewport.visible_pixel_range.clone(), |this, range| {
                            this.child(
                                div()
                                    .absolute()
                                    .top(px(range.start as f32))
                                    .left_0()
                                    .right_0()
                                    .h(px(range.len() as f32))
                                    .child(
                                        crate::app::ai_workspace_surface::AiWorkspaceSurfaceElement {
                                            view: cx.entity(),
                                            snapshot: std::rc::Rc::new(surface.clone()),
                                            selection: self.ai_workspace_selection.clone(),
                                            ui_font_family: cx.theme().font_family.clone(),
                                            mono_font_family: cx.theme().mono_font_family.clone(),
                                            workspace_root: workspace_root.clone(),
                                        }
                                        .into_any_element(),
                                    ),
                            )
                            .children(ai_workspace_overlay_copy_actions(
                                cx.entity(),
                                &surface,
                                viewport_width_px.max(1),
                                cx.theme().muted_foreground,
                            ))
                        }),
                )
                .into_any_element(),
        )
    }
}

fn ai_workspace_overlay_copy_actions(
    view: Entity<DiffViewer>,
    surface: &ai_workspace_session::AiWorkspaceSurfaceSnapshot,
    viewport_width_px: usize,
    muted_foreground: gpui::Hsla,
) -> Vec<AnyElement> {
    let mut actions = Vec::new();

    for block in &surface.viewport.visible_blocks {
        if let Some(copy_text) = block.block.copy_text.clone() {
            let (left_px, top_px) =
                ai_workspace_overlay_button_position(block, viewport_width_px, 0);
            actions.push(ai_workspace_overlay_copy_button(
                view.clone(),
                AiWorkspaceOverlayCopyButton {
                    id: format!("ai-workspace-copy-{}", block.block.id),
                    left_px,
                    top_px,
                    tooltip: block.block.copy_tooltip.unwrap_or("Copy"),
                    text: copy_text,
                    success_message: block.block.copy_success_message.unwrap_or("Copied."),
                    message_copy: block.block.copy_tooltip == Some("Copy message"),
                },
                muted_foreground,
            ));
        }

        for (copy_index, copy_region) in block.text_layout.preview_copy_regions.iter().enumerate() {
            let preview_top_px = ai_workspace_session::AI_WORKSPACE_BLOCK_CONTENT_TOP_PADDING_PX
                + ai_workspace_session::AI_WORKSPACE_BLOCK_TITLE_LINE_HEIGHT_PX
                    * block.text_layout.title_lines.len()
                + if block.text_layout.preview_lines.is_empty() {
                    0
                } else {
                    ai_workspace_session::AI_WORKSPACE_BLOCK_SECTION_GAP_PX
                };
            let (left_px, top_px) = ai_workspace_overlay_button_position(
                block,
                viewport_width_px,
                preview_top_px
                    + ai_workspace_session::AI_WORKSPACE_BLOCK_PREVIEW_LINE_HEIGHT_PX
                        * copy_region.line_range.start,
            );
            actions.push(ai_workspace_overlay_copy_button(
                view.clone(),
                AiWorkspaceOverlayCopyButton {
                    id: format!("ai-workspace-copy-{}-{copy_index}", block.block.id),
                    left_px,
                    top_px,
                    tooltip: copy_region.tooltip,
                    text: copy_region.text.clone(),
                    success_message: copy_region.success_message,
                    message_copy: false,
                },
                muted_foreground,
            ));
        }
    }

    actions
}

fn ai_workspace_overlay_copy_button(
    view: Entity<DiffViewer>,
    spec: AiWorkspaceOverlayCopyButton,
    muted_foreground: gpui::Hsla,
) -> AnyElement {
    div()
        .absolute()
        .left(px(spec.left_px as f32))
        .top(px(spec.top_px as f32))
        .child(
            Button::new(spec.id)
                .ghost()
                .compact()
                .rounded(px(7.0))
                .icon(Icon::new(IconName::Copy).size(px(12.0)))
                .text_color(muted_foreground)
                .min_w(px(22.0))
                .h(px(20.0))
                .tooltip(spec.tooltip)
                .on_click(move |_, window, cx| {
                    view.update(cx, |this, cx| {
                        if spec.message_copy {
                            this.ai_copy_message_action(spec.text.clone(), window, cx);
                        } else {
                            this.ai_copy_text_action(
                                spec.text.clone(),
                                spec.success_message,
                                window,
                                cx,
                            );
                        }
                    });
                }),
        )
        .into_any_element()
}

fn ai_workspace_overlay_button_position(
    block: &ai_workspace_session::AiWorkspaceViewportBlock,
    viewport_width_px: usize,
    additional_top_px: usize,
) -> (usize, usize) {
    const BUTTON_WIDTH_PX: usize = 22;
    const BUTTON_RIGHT_PADDING_PX: usize = 4;

    let lane_max_width = if block.block.role == ai_workspace_session::AiWorkspaceBlockRole::User {
        crate::app::ai_workspace_timeline_projection::AI_WORKSPACE_USER_CONTENT_LANE_MAX_WIDTH_PX
    } else {
        crate::app::ai_workspace_timeline_projection::AI_WORKSPACE_CONTENT_LANE_MAX_WIDTH_PX
    };
    let lane_width = viewport_width_px
        .saturating_sub(ai_workspace_session::AI_WORKSPACE_SURFACE_BLOCK_SIDE_PADDING_PX * 2)
        .min(lane_max_width);
    let lane_x = viewport_width_px.saturating_sub(lane_width) / 2;
    let block_x = match block.block.role {
        ai_workspace_session::AiWorkspaceBlockRole::User => lane_x
            .saturating_add(lane_width)
            .saturating_sub(block.text_layout.block_width_px),
        _ => lane_x.saturating_add(usize::from(block.block.nested) * 16),
    };

    (
        block_x
            .saturating_add(block.text_layout.block_width_px)
            .saturating_sub(BUTTON_WIDTH_PX + BUTTON_RIGHT_PADDING_PX),
        block.top_px.saturating_add(4).saturating_add(additional_top_px),
    )
}
