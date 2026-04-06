use std::rc::Rc;
use std::time::Instant;

use gpui::{
    App, Bounds, ContentMask, DispatchPhase, Element, ElementId, FontWeight, GlobalElementId,
    Hitbox, HitboxBehavior, InspectorElementId, IntoElement, LayoutId, MouseButton, MouseDownEvent,
    Pixels, Point, SharedString, TextStyle, Window, fill, point, px, relative, size,
};
use gpui_component::ActiveTheme as _;

use crate::app::native_files_editor::paint::{
    paint_editor_line, shape_editor_line, single_color_text_run,
};
use crate::app::{DiffViewer, ai_workspace_session};

pub(crate) struct AiWorkspaceSurfaceElement {
    pub(crate) view: gpui::Entity<DiffViewer>,
    pub(crate) snapshot: Rc<ai_workspace_session::AiWorkspaceSurfaceSnapshot>,
    pub(crate) selection: Option<ai_workspace_session::AiWorkspaceSelection>,
    pub(crate) ui_font_family: SharedString,
    pub(crate) mono_font_family: SharedString,
}

#[derive(Clone)]
pub(crate) struct AiWorkspaceSurfaceLayout {
    hitbox: Hitbox,
}

const AI_WORKSPACE_BLOCK_TITLE_REGION_END_PX: f32 = 34.0;
const AI_WORKSPACE_BLOCK_PREVIEW_REGION_START_PX: f32 = 34.0;

impl IntoElement for AiWorkspaceSurfaceElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for AiWorkspaceSurfaceElement {
    type RequestLayoutState = ();
    type PrepaintState = AiWorkspaceSurfaceLayout;

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
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        AiWorkspaceSurfaceLayout {
            hitbox: window.insert_hitbox(bounds, HitboxBehavior::Normal),
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        layout: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let paint_started_at = Instant::now();
        let hitbox = layout.hitbox.clone();
        let snapshot = self.snapshot.clone();
        let view = self.view.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, _window, cx| {
            if phase != DispatchPhase::Bubble
                || !matches!(event.button, MouseButton::Left | MouseButton::Middle)
                || !hitbox.is_hovered(_window)
            {
                return;
            }

            view.read(cx).record_ai_workspace_surface_hit_test();
            let Some(selection) = ai_workspace_selection_at_position(
                snapshot.as_ref(),
                event.position,
                hitbox.bounds,
            ) else {
                return;
            };

            view.update(cx, |this, cx| {
                this.ai_select_workspace_selection(selection.clone(), cx);
                cx.stop_propagation();
            });
        });

        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for block in &self.snapshot.viewport.visible_blocks {
                paint_ai_workspace_block(
                    window,
                    cx,
                    bounds,
                    self.snapshot.scroll_top_px,
                    block,
                    self.selection
                        .as_ref()
                        .is_some_and(|selection| selection.matches_block(block.block.id.as_str())),
                    self.ui_font_family.clone(),
                    self.mono_font_family.clone(),
                );
            }
        });
        self.view.read(cx).record_ai_workspace_surface_paint_timing(
            paint_started_at.elapsed(),
            self.snapshot.viewport.visible_blocks.len(),
        );
    }
}

fn ai_workspace_selection_at_position(
    snapshot: &ai_workspace_session::AiWorkspaceSurfaceSnapshot,
    position: Point<Pixels>,
    bounds: Bounds<Pixels>,
) -> Option<ai_workspace_session::AiWorkspaceSelection> {
    if !bounds.contains(&position) {
        return None;
    }

    let local_y_px = (position.y - bounds.origin.y)
        .max(Pixels::ZERO)
        .as_f32()
        .round() as usize;
    let surface_y_px = snapshot.scroll_top_px.saturating_add(local_y_px);
    let block = snapshot.viewport.visible_blocks.iter().find(|block| {
        let bottom_px = block.top_px.saturating_add(block.height_px);
        surface_y_px >= block.top_px && surface_y_px < bottom_px
    })?;

    let block_local_y_px = surface_y_px.saturating_sub(block.top_px) as f32;
    let (region, line_index) = if block_local_y_px <= AI_WORKSPACE_BLOCK_TITLE_REGION_END_PX {
        (
            ai_workspace_session::AiWorkspaceSelectionRegion::Title,
            Some(0),
        )
    } else if !block.block.preview.trim().is_empty()
        && block_local_y_px >= AI_WORKSPACE_BLOCK_PREVIEW_REGION_START_PX
    {
        (
            ai_workspace_session::AiWorkspaceSelectionRegion::Preview,
            Some(1),
        )
    } else {
        (
            ai_workspace_session::AiWorkspaceSelectionRegion::Block,
            None,
        )
    };

    Some(ai_workspace_session::AiWorkspaceSelection {
        block_id: block.block.id.clone(),
        block_kind: block.block.kind,
        line_index,
        region,
    })
}

#[allow(clippy::too_many_arguments)]
fn paint_ai_workspace_block(
    window: &mut Window,
    cx: &mut App,
    bounds: Bounds<Pixels>,
    scroll_top_px: usize,
    block: &ai_workspace_session::AiWorkspaceViewportBlock,
    selected: bool,
    ui_font_family: SharedString,
    mono_font_family: SharedString,
) {
    let role = block.block.role;
    let surface_top = px(block.top_px as f32 - scroll_top_px as f32);
    let block_height = px(block.height_px as f32);
    let horizontal_padding =
        px(ai_workspace_session::AI_WORKSPACE_SURFACE_BLOCK_SIDE_PADDING_PX as f32);
    let available_width = (bounds.size.width - horizontal_padding * 2.).max(px(180.0));
    let desired_width = match role {
        ai_workspace_session::AiWorkspaceBlockRole::User => (available_width * 0.72).min(px(520.0)),
        ai_workspace_session::AiWorkspaceBlockRole::Assistant => available_width.min(px(620.0)),
        ai_workspace_session::AiWorkspaceBlockRole::Tool => available_width.min(px(700.0)),
        ai_workspace_session::AiWorkspaceBlockRole::System => available_width.min(px(640.0)),
    };
    let block_width = desired_width.max(px(200.0)).min(available_width);
    let block_x = match role {
        ai_workspace_session::AiWorkspaceBlockRole::User => {
            bounds.origin.x + bounds.size.width - horizontal_padding - block_width
        }
        _ => bounds.origin.x + horizontal_padding,
    };
    let block_bounds = Bounds {
        origin: point(block_x, bounds.origin.y + surface_top),
        size: size(block_width, block_height),
    };

    let is_dark = cx.theme().mode.is_dark();
    let (background, border, accent, title_color, preview_color) =
        ai_workspace_block_palette(block.block.kind, role, selected, is_dark, cx);

    window.paint_quad(fill(block_bounds, background));
    window.paint_quad(fill(
        Bounds {
            origin: block_bounds.origin,
            size: size(px(3.0), block_bounds.size.height),
        },
        accent,
    ));
    if selected {
        window.paint_quad(fill(
            Bounds {
                origin: point(block_bounds.origin.x, block_bounds.origin.y),
                size: size(block_bounds.size.width, px(1.0)),
            },
            border,
        ));
        window.paint_quad(fill(
            Bounds {
                origin: point(
                    block_bounds.origin.x,
                    block_bounds.origin.y + block_bounds.size.height - px(1.0),
                ),
                size: size(block_bounds.size.width, px(1.0)),
            },
            border,
        ));
    }

    let title_style = TextStyle {
        color: title_color,
        font_family: ui_font_family.clone(),
        font_size: px(12.0).into(),
        font_weight: FontWeight::SEMIBOLD,
        line_height: relative(1.35),
        ..Default::default()
    };
    let title_font = title_style.font();
    let title_font_size = title_style.font_size.to_pixels(window.rem_size());
    let title_line_height = title_style.line_height_in_pixels(window.rem_size());
    let title_runs = vec![single_color_text_run(
        block.block.title.len().max(1),
        title_color,
        title_font.clone(),
    )];
    let title_shape = shape_editor_line(
        window,
        SharedString::from(block.block.title.clone()),
        title_font_size,
        &title_runs,
    );
    paint_editor_line(
        window,
        cx,
        &title_shape,
        point(
            block_bounds.origin.x + px(14.0),
            block_bounds.origin.y + px(10.0),
        ),
        title_line_height,
    );

    if !block.block.preview.is_empty() {
        let preview_font_family =
            if block.block.kind == ai_workspace_session::AiWorkspaceBlockKind::DiffSummary {
                mono_font_family.clone()
            } else {
                ui_font_family.clone()
            };
        let preview_style = TextStyle {
            color: preview_color,
            font_family: preview_font_family,
            font_size: px(12.0).into(),
            line_height: relative(1.45),
            ..Default::default()
        };
        let preview_font = preview_style.font();
        let preview_font_size = preview_style.font_size.to_pixels(window.rem_size());
        let preview_line_height = preview_style.line_height_in_pixels(window.rem_size());
        let preview_runs = vec![single_color_text_run(
            block.block.preview.len().max(1),
            preview_color,
            preview_font.clone(),
        )];
        let preview_shape = shape_editor_line(
            window,
            SharedString::from(block.block.preview.clone()),
            preview_font_size,
            &preview_runs,
        );
        paint_editor_line(
            window,
            cx,
            &preview_shape,
            point(
                block_bounds.origin.x + px(14.0),
                block_bounds.origin.y + px(10.0) + title_line_height + px(8.0),
            ),
            preview_line_height,
        );
    }
}

fn ai_workspace_block_palette(
    kind: ai_workspace_session::AiWorkspaceBlockKind,
    role: ai_workspace_session::AiWorkspaceBlockRole,
    selected: bool,
    is_dark: bool,
    cx: &App,
) -> (gpui::Hsla, gpui::Hsla, gpui::Hsla, gpui::Hsla, gpui::Hsla) {
    let accent = match (kind, role) {
        (ai_workspace_session::AiWorkspaceBlockKind::DiffSummary, _) => cx.theme().warning,
        (_, ai_workspace_session::AiWorkspaceBlockRole::User) => cx.theme().accent,
        (_, ai_workspace_session::AiWorkspaceBlockRole::Assistant) => cx.theme().success,
        (_, ai_workspace_session::AiWorkspaceBlockRole::Tool) => cx.theme().warning,
        (_, ai_workspace_session::AiWorkspaceBlockRole::System) => cx.theme().muted_foreground,
    };
    let background = crate::app::theme::hunk_opacity(accent, is_dark, 0.12, 0.08);
    let border = crate::app::theme::hunk_opacity(accent, is_dark, 0.58, 0.42);
    let title_color = if selected {
        cx.theme().foreground
    } else {
        accent
    };
    let preview_color = if role == ai_workspace_session::AiWorkspaceBlockRole::User {
        cx.theme().foreground
    } else {
        cx.theme().muted_foreground
    };
    (background, border, accent, title_color, preview_color)
}
