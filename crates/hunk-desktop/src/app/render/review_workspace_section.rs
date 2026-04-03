#[derive(Clone)]
struct ReviewWorkspaceViewportElement {
    view: Entity<DiffViewer>,
    viewport: std::rc::Rc<review_workspace_session::ReviewWorkspaceViewportSnapshot>,
    viewport_origin_px: usize,
    selected_row_range: Option<(usize, usize)>,
    layout: Option<DiffColumnLayout>,
    left_line_number_width: f32,
    right_line_number_width: f32,
    center_divider: gpui::Hsla,
    mono_font_family: SharedString,
}

#[derive(Clone)]
struct ReviewWorkspaceSectionLayout {
    hitbox: gpui::Hitbox,
}

impl IntoElement for ReviewWorkspaceViewportElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ReviewWorkspaceViewportElement {
    type RequestLayoutState = ();
    type PrepaintState = ReviewWorkspaceSectionLayout;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = gpui::Style::default();
        style.size.width = gpui::relative(1.).into();
        style.size.height = gpui::relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<gpui::Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        ReviewWorkspaceSectionLayout {
            hitbox: window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal),
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<gpui::Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        layout: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let viewport = self.viewport.clone();
        let viewport_origin_px = self.viewport_origin_px;
        let hitbox = layout.hitbox.clone();
        let view = self.view.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase != gpui::DispatchPhase::Bubble || !hitbox.is_hovered(window) {
                return;
            }
            let Some(row_ix) = review_workspace_row_at_position(
                viewport.as_ref(),
                viewport_origin_px,
                event.position,
                hitbox.bounds.origin,
            ) else {
                return;
            };
            view.update(cx, |this, cx| match event.button {
                gpui::MouseButton::Left | gpui::MouseButton::Middle => {
                    this.on_diff_row_mouse_down(row_ix, event, window, cx);
                }
                gpui::MouseButton::Right => {
                    this.open_diff_row_context_menu(row_ix, event.position, window, cx);
                    cx.stop_propagation();
                }
                _ => {}
            });
        });

        let viewport = self.viewport.clone();
        let viewport_origin_px = self.viewport_origin_px;
        let hitbox = layout.hitbox.clone();
        let view = self.view.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if phase != gpui::DispatchPhase::Bubble || !hitbox.is_hovered(window) {
                return;
            }
            let Some(row_ix) = review_workspace_row_at_position(
                viewport.as_ref(),
                viewport_origin_px,
                event.position,
                hitbox.bounds.origin,
            ) else {
                return;
            };
            view.update(cx, |this, cx| {
                this.on_diff_row_mouse_move(row_ix, event, window, cx);
            });
        });

        let view = self.view.clone();
        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
            if phase != gpui::DispatchPhase::Bubble {
                return;
            }
            if matches!(event.button, gpui::MouseButton::Left | gpui::MouseButton::Middle) {
                view.update(cx, |this, cx| {
                    this.on_diff_row_mouse_up(event, window, cx);
                });
            }
        });

        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for viewport_row in self
                .viewport
                .sections
                .iter()
                .flat_map(|section| section.rows.iter())
            {
                let row_bounds = Bounds {
                    origin: point(
                        bounds.origin.x,
                        bounds.origin.y
                            + px(
                                viewport_row
                                    .surface_top_px
                                    .saturating_sub(self.viewport_origin_px)
                                    as f32,
                            ),
                    ),
                    size: gpui::size(bounds.size.width, px(viewport_row.height_px as f32)),
                };
                let is_selected = review_workspace_row_is_selected(
                    self.selected_row_range,
                    viewport_row.row_index,
                );

                if viewport_row.stream_kind == DiffStreamRowKind::FileHeader {
                    let Some(path) = viewport_row.file_path.as_deref() else {
                        continue;
                    };
                    let status = viewport_row.file_status.unwrap_or(FileStatus::Unknown);
                    let stats = viewport_row.file_line_stats.unwrap_or_default();
                    let paint = build_review_workspace_file_header_paint(
                        cx.theme(),
                        path,
                        status,
                        stats,
                        is_selected,
                    );
                    paint_review_workspace_file_header_row(
                        window,
                        cx,
                        row_bounds,
                        &paint,
                        self.mono_font_family.clone(),
                    );
                    continue;
                }

                match viewport_row.row_kind {
                    DiffRowKind::Code => {
                        let left = build_review_workspace_code_row_cell_paint(
                            cx.theme(),
                            self.left_line_number_width,
                            viewport_row.stable_id,
                            is_selected,
                            DiffCellRenderSpec {
                                side: "left",
                                line: viewport_row.left_line,
                                cell_kind: viewport_row.left_cell_kind,
                                peer_kind: viewport_row.right_cell_kind,
                                panel_width: self.layout.map(|layout| layout.left_panel_width),
                            },
                            viewport_row,
                        );
                        let right = build_review_workspace_code_row_cell_paint(
                            cx.theme(),
                            self.right_line_number_width,
                            viewport_row.stable_id,
                            is_selected,
                            DiffCellRenderSpec {
                                side: "right",
                                line: viewport_row.right_line,
                                cell_kind: viewport_row.right_cell_kind,
                                peer_kind: viewport_row.left_cell_kind,
                                panel_width: self.layout.map(|layout| layout.right_panel_width),
                            },
                            viewport_row,
                        );
                        paint_review_workspace_code_row(
                            window,
                            cx,
                            row_bounds,
                            &left,
                            &right,
                            self.center_divider,
                            self.mono_font_family.clone(),
                        );
                    }
                    DiffRowKind::HunkHeader | DiffRowKind::Meta | DiffRowKind::Empty => {
                        let meta = build_review_workspace_meta_row_paint(
                            cx.theme(),
                            viewport_row.row_kind,
                            &viewport_row.text,
                            is_selected,
                        );
                        paint_review_workspace_meta_row(
                            window,
                            cx,
                            row_bounds,
                            &meta,
                            self.mono_font_family.clone(),
                        );
                    }
                }
            }
        });
    }
}

fn review_workspace_row_is_selected(
    selected_row_range: Option<(usize, usize)>,
    row_index: usize,
) -> bool {
    selected_row_range
        .is_some_and(|(start, end)| row_index >= start && row_index <= end)
}

fn review_workspace_row_at_position(
    viewport: &review_workspace_session::ReviewWorkspaceViewportSnapshot,
    viewport_origin_px: usize,
    position: gpui::Point<gpui::Pixels>,
    origin: gpui::Point<gpui::Pixels>,
) -> Option<usize> {
    let local_y = (position.y - origin.y).max(gpui::Pixels::ZERO).as_f32().floor() as usize;
    viewport
        .row_at_viewport_position(viewport_origin_px, local_y)
        .map(|row| row.row_index)
}

impl DiffViewer {
    fn render_review_workspace_viewport_element(
        &self,
        viewport: &review_workspace_session::ReviewWorkspaceViewportSnapshot,
        viewport_origin_px: usize,
        layout: Option<DiffColumnLayout>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let chrome = hunk_diff_chrome(cx.theme(), cx.theme().mode.is_dark());
        ReviewWorkspaceViewportElement {
            view: cx.entity(),
            viewport: std::rc::Rc::new(viewport.clone()),
            viewport_origin_px,
            selected_row_range: self.selected_row_range(),
            layout,
            left_line_number_width: self.review_surface.diff_left_line_number_width,
            right_line_number_width: self.review_surface.diff_right_line_number_width,
            center_divider: chrome.center_divider,
            mono_font_family: cx.theme().mono_font_family.clone(),
        }
        .into_any_element()
    }
}
