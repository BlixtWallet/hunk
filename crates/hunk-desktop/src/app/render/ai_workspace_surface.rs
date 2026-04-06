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
                        }),
                )
                .into_any_element(),
        )
    }
}
