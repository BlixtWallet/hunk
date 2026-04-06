use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use gpui::{
    App, Bounds, ContentMask, DispatchPhase, Element, ElementId, Font, FontWeight, GlobalElementId,
    Hitbox, HitboxBehavior, InspectorElementId, IntoElement, LayoutId, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Point, SharedString, TextRun, TextStyle, Window, fill,
    point, px, relative, size,
};
use gpui_component::ActiveTheme as _;

use crate::app::markdown_links::{MarkdownLinkRange, resolve_markdown_link_target};
use crate::app::native_files_editor::paint::{
    paint_editor_line, shape_editor_line, single_color_text_run,
};
use crate::app::{
    AiPressedMarkdownLink, AiTextSelectionSurfaceSpec, DiffViewer, ai_workspace_session,
};

pub(crate) struct AiWorkspaceSurfaceElement {
    pub(crate) view: gpui::Entity<DiffViewer>,
    pub(crate) snapshot: Rc<ai_workspace_session::AiWorkspaceSurfaceSnapshot>,
    pub(crate) selection: Option<ai_workspace_session::AiWorkspaceSelection>,
    pub(crate) ui_font_family: SharedString,
    pub(crate) mono_font_family: SharedString,
    pub(crate) workspace_root: Option<PathBuf>,
}

#[derive(Clone)]
pub(crate) struct AiWorkspaceSurfaceLayout {
    hitbox: Hitbox,
}

#[derive(Clone)]
struct AiWorkspaceBlockRenderLayout {
    block_bounds: Bounds<Pixels>,
    text_origin_x: Pixels,
    title_origin_y: Pixels,
    preview_origin_y: Pixels,
    title_line_height: Pixels,
    preview_line_height: Pixels,
    title_char_width: Pixels,
    preview_char_width: Pixels,
    toggle_bounds: Option<Bounds<Pixels>>,
}

#[derive(Clone)]
struct AiWorkspacePaintLine {
    surface_id: String,
    text: String,
    surface_byte_range: Range<usize>,
    column_byte_offsets: Arc<[usize]>,
    link_ranges: Arc<[MarkdownLinkRange]>,
    origin: Point<Pixels>,
    line_height: Pixels,
    char_width: Pixels,
    title: bool,
    monospace: bool,
}

#[derive(Clone)]
struct AiWorkspaceTextHit {
    surface_id: String,
    index: usize,
    link_target: Option<String>,
    selection_surfaces: Arc<[AiTextSelectionSurfaceSpec]>,
}

#[derive(Clone)]
struct AiWorkspaceBlockHit {
    selection: ai_workspace_session::AiWorkspaceSelection,
    text_hit: Option<AiWorkspaceTextHit>,
    toggle_row_id: Option<String>,
}

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
        let workspace_root = self.workspace_root.clone();

        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble
                || !matches!(event.button, MouseButton::Left | MouseButton::Middle)
                || !hitbox.is_hovered(window)
            {
                return;
            }

            view.read(cx).record_ai_workspace_surface_hit_test();
            let Some(hit) = ai_workspace_hit_test(
                snapshot.as_ref(),
                event.position,
                hitbox.bounds,
                workspace_root.as_deref(),
            ) else {
                return;
            };

            view.update(cx, |this, cx| {
                if let Some(toggle_row_id) = hit.toggle_row_id.clone()
                    && event.button == MouseButton::Left
                {
                    this.ai_workspace_toggle_row_expansion(toggle_row_id, cx);
                    cx.stop_propagation();
                    return;
                }

                let pressed_link = hit.text_hit.as_ref().and_then(|text_hit| {
                    text_hit
                        .link_target
                        .clone()
                        .map(|raw_target| AiPressedMarkdownLink {
                            surface_id: text_hit.surface_id.clone(),
                            raw_target,
                            mouse_down_position: event.position,
                            dragged: false,
                        })
                });
                this.ai_set_pressed_markdown_link(pressed_link);
                this.ai_select_workspace_selection(hit.selection.clone(), cx);
                if let Some(text_hit) = hit.text_hit.as_ref() {
                    this.ai_begin_text_selection(
                        hit.selection.block_id.clone(),
                        text_hit.selection_surfaces.clone(),
                        text_hit.surface_id.as_str(),
                        text_hit.index,
                        window,
                        cx,
                    );
                }
                cx.stop_propagation();
            });
        });

        let snapshot_for_mouse_move = self.snapshot.clone();
        let view_for_mouse_move = self.view.clone();
        let workspace_root_for_mouse_move = self.workspace_root.clone();
        let hitbox_for_mouse_move = layout.hitbox.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _window, cx| {
            if phase != DispatchPhase::Bubble {
                return;
            }

            view_for_mouse_move.update(cx, |this, _| {
                this.ai_mark_pressed_markdown_link_dragged(event.position);
            });

            let dragging_selection = view_for_mouse_move
                .read(cx)
                .ai_text_selection
                .as_ref()
                .is_some_and(|selection| selection.dragging);
            if !dragging_selection {
                return;
            }

            let Some(hit) = ai_workspace_hit_test(
                snapshot_for_mouse_move.as_ref(),
                event.position,
                hitbox_for_mouse_move.bounds,
                workspace_root_for_mouse_move.as_deref(),
            ) else {
                return;
            };
            let Some(text_hit) = hit.text_hit.as_ref() else {
                return;
            };

            view_for_mouse_move.update(cx, |this, cx| {
                this.ai_update_text_selection(text_hit.surface_id.as_str(), text_hit.index, cx);
            });
        });

        let snapshot_for_mouse_up = self.snapshot.clone();
        let view_for_mouse_up = self.view.clone();
        let workspace_root_for_mouse_up = self.workspace_root.clone();
        let hitbox_for_mouse_up = layout.hitbox.clone();
        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble || event.button != MouseButton::Left {
                return;
            }

            view_for_mouse_up.update(cx, |this, cx| {
                this.ai_end_text_selection(cx);
                if !hitbox_for_mouse_up.is_hovered(window) {
                    let _ = this.ai_take_pressed_markdown_link();
                }
            });

            if !hitbox_for_mouse_up.is_hovered(window) {
                return;
            }

            view_for_mouse_up.update(cx, |this, cx| {
                let Some(pressed_link) = this.ai_take_pressed_markdown_link() else {
                    return;
                };
                let Some(hit) = ai_workspace_hit_test(
                    snapshot_for_mouse_up.as_ref(),
                    event.position,
                    hitbox_for_mouse_up.bounds,
                    workspace_root_for_mouse_up.as_deref(),
                ) else {
                    return;
                };
                let Some(text_hit) = hit.text_hit.as_ref() else {
                    return;
                };
                if pressed_link.dragged || pressed_link.surface_id != text_hit.surface_id {
                    return;
                }
                let activated = text_hit
                    .link_target
                    .as_ref()
                    .is_some_and(|target| target == &pressed_link.raw_target);
                if activated {
                    this.activate_markdown_link(pressed_link.raw_target, Some(window), cx);
                }
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
                    self.view.clone(),
                    self.ui_font_family.clone(),
                    self.mono_font_family.clone(),
                    self.workspace_root.as_deref(),
                );
            }
        });
        self.view.read(cx).record_ai_workspace_surface_paint_timing(
            paint_started_at.elapsed(),
            self.snapshot.viewport.visible_blocks.len(),
        );
    }
}

fn ai_workspace_hit_test(
    snapshot: &ai_workspace_session::AiWorkspaceSurfaceSnapshot,
    position: Point<Pixels>,
    bounds: Bounds<Pixels>,
    workspace_root: Option<&Path>,
) -> Option<AiWorkspaceBlockHit> {
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
    let render_layout = ai_workspace_block_render_layout(bounds, snapshot.scroll_top_px, block);
    let block_local_y_px = surface_y_px.saturating_sub(block.top_px);
    let title_start_px = ai_workspace_session::AI_WORKSPACE_BLOCK_CONTENT_TOP_PADDING_PX;
    let title_height_px = block.text_layout.title_lines.len()
        * ai_workspace_session::AI_WORKSPACE_BLOCK_TITLE_LINE_HEIGHT_PX;
    let preview_start_px = title_start_px
        .saturating_add(title_height_px)
        .saturating_add(if block.text_layout.preview_lines.is_empty() {
            0
        } else {
            ai_workspace_session::AI_WORKSPACE_BLOCK_SECTION_GAP_PX
        });
    let preview_height_px = block.text_layout.preview_lines.len()
        * ai_workspace_session::AI_WORKSPACE_BLOCK_PREVIEW_LINE_HEIGHT_PX;

    let (region, line_index) = if block_local_y_px >= title_start_px
        && block_local_y_px < title_start_px.saturating_add(title_height_px)
    {
        let line_index = (block_local_y_px.saturating_sub(title_start_px)
            / ai_workspace_session::AI_WORKSPACE_BLOCK_TITLE_LINE_HEIGHT_PX)
            .min(block.text_layout.title_lines.len().saturating_sub(1));
        (
            ai_workspace_session::AiWorkspaceSelectionRegion::Title,
            Some(line_index),
        )
    } else if block_local_y_px >= preview_start_px
        && block_local_y_px < preview_start_px.saturating_add(preview_height_px)
    {
        let line_index = (block_local_y_px.saturating_sub(preview_start_px)
            / ai_workspace_session::AI_WORKSPACE_BLOCK_PREVIEW_LINE_HEIGHT_PX)
            .min(block.text_layout.preview_lines.len().saturating_sub(1));
        (
            ai_workspace_session::AiWorkspaceSelectionRegion::Preview,
            Some(line_index),
        )
    } else {
        (
            ai_workspace_session::AiWorkspaceSelectionRegion::Block,
            None,
        )
    };

    let selection = ai_workspace_session::AiWorkspaceSelection {
        block_id: block.block.id.clone(),
        block_kind: block.block.kind,
        line_index,
        region,
    };
    let toggle_row_id = render_layout
        .toggle_bounds
        .filter(|toggle_bounds| toggle_bounds.contains(&position))
        .map(|_| block.block.source_row_id.clone());
    let text_hit = ai_workspace_text_hit(block, &render_layout, position, workspace_root);

    Some(AiWorkspaceBlockHit {
        selection,
        text_hit,
        toggle_row_id,
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
    view: gpui::Entity<DiffViewer>,
    ui_font_family: SharedString,
    mono_font_family: SharedString,
    workspace_root: Option<&Path>,
) {
    let render_layout = ai_workspace_block_render_layout(bounds, scroll_top_px, block);
    let is_dark = cx.theme().mode.is_dark();
    let (background, border, accent, title_color, preview_color, link_color) =
        ai_workspace_block_palette(block.block.kind, block.block.role, selected, is_dark, cx);

    window.paint_quad(fill(render_layout.block_bounds, background));
    window.paint_quad(fill(
        Bounds {
            origin: render_layout.block_bounds.origin,
            size: size(px(3.0), render_layout.block_bounds.size.height),
        },
        accent,
    ));
    if selected {
        window.paint_quad(fill(
            Bounds {
                origin: point(
                    render_layout.block_bounds.origin.x,
                    render_layout.block_bounds.origin.y,
                ),
                size: size(render_layout.block_bounds.size.width, px(1.0)),
            },
            border,
        ));
        window.paint_quad(fill(
            Bounds {
                origin: point(
                    render_layout.block_bounds.origin.x,
                    render_layout.block_bounds.origin.y + render_layout.block_bounds.size.height
                        - px(1.0),
                ),
                size: size(render_layout.block_bounds.size.width, px(1.0)),
            },
            border,
        ));
    }

    if let Some(toggle_bounds) = render_layout.toggle_bounds {
        let toggle_background = crate::app::theme::hunk_opacity(accent, is_dark, 0.20, 0.14);
        let toggle_label = if block.block.expanded {
            "Show less"
        } else {
            "Show more"
        };
        window.paint_quad(fill(toggle_bounds, toggle_background));
        let toggle_runs = vec![single_color_text_run(
            toggle_label.len(),
            accent,
            TextStyle {
                color: accent,
                font_family: ui_font_family.clone(),
                font_size: px(10.0).into(),
                font_weight: FontWeight::SEMIBOLD,
                line_height: relative(1.0),
                ..Default::default()
            }
            .font(),
        )];
        let toggle_shape = shape_editor_line(
            window,
            SharedString::from(toggle_label),
            px(10.0),
            &toggle_runs,
        );
        paint_editor_line(
            window,
            cx,
            &toggle_shape,
            point(
                toggle_bounds.origin.x + px(8.0),
                toggle_bounds.origin.y + px(3.0),
            ),
            px(12.0),
        );
    }

    let lines = ai_workspace_paint_lines_for_block(block, &render_layout, workspace_root);
    let current_selection = view.read(cx).ai_text_selection.clone();

    for line in &lines {
        ai_workspace_paint_selection(
            line,
            current_selection
                .as_ref()
                .and_then(|selection| selection.range_for_surface(line.surface_id.as_str())),
            window,
        );
        if line.text.is_empty() {
            continue;
        }
        let (font_family, font_weight, color) = if line.title {
            (ui_font_family.clone(), FontWeight::SEMIBOLD, title_color)
        } else if line.monospace {
            (mono_font_family.clone(), FontWeight::NORMAL, preview_color)
        } else {
            (ui_font_family.clone(), FontWeight::NORMAL, preview_color)
        };
        let style = TextStyle {
            color,
            font_family,
            font_size: px(12.0).into(),
            font_weight,
            line_height: if line.title {
                relative(1.35)
            } else {
                relative(1.45)
            },
            ..Default::default()
        };
        let font = style.font();
        let runs = ai_workspace_text_runs_for_line(line, color, link_color, font);
        let shape = shape_editor_line(
            window,
            SharedString::from(line.text.clone()),
            style.font_size.to_pixels(window.rem_size()),
            &runs,
        );
        paint_editor_line(window, cx, &shape, line.origin, line.line_height);
    }
}

fn ai_workspace_block_render_layout(
    bounds: Bounds<Pixels>,
    scroll_top_px: usize,
    block: &ai_workspace_session::AiWorkspaceViewportBlock,
) -> AiWorkspaceBlockRenderLayout {
    let role = block.block.role;
    let surface_top = px(block.top_px as f32 - scroll_top_px as f32);
    let block_height = px(block.height_px as f32);
    let horizontal_padding =
        px(ai_workspace_session::AI_WORKSPACE_SURFACE_BLOCK_SIDE_PADDING_PX as f32);
    let block_width = px(block.text_layout.block_width_px as f32);
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
    let text_origin_x = block_bounds.origin.x
        + px(ai_workspace_session::AI_WORKSPACE_BLOCK_TEXT_SIDE_PADDING_PX as f32);
    let title_origin_y = block_bounds.origin.y
        + px(ai_workspace_session::AI_WORKSPACE_BLOCK_CONTENT_TOP_PADDING_PX as f32);
    let title_line_height =
        px(ai_workspace_session::AI_WORKSPACE_BLOCK_TITLE_LINE_HEIGHT_PX as f32);
    let preview_origin_y = title_origin_y
        + title_line_height * block.text_layout.title_lines.len() as f32
        + px(ai_workspace_session::AI_WORKSPACE_BLOCK_SECTION_GAP_PX as f32);
    let preview_line_height =
        px(ai_workspace_session::AI_WORKSPACE_BLOCK_PREVIEW_LINE_HEIGHT_PX as f32);
    let text_width_px =
        ai_workspace_session::ai_workspace_block_text_width_px(block.text_layout.block_width_px);
    let title_char_width = px((text_width_px as f32)
        / ai_workspace_session::ai_workspace_chars_per_line(text_width_px, true, false).max(1)
            as f32);
    let preview_char_width = px((text_width_px as f32)
        / ai_workspace_session::ai_workspace_chars_per_line(
            text_width_px,
            false,
            block.block.kind == ai_workspace_session::AiWorkspaceBlockKind::DiffSummary,
        )
        .max(1) as f32);
    let toggle_bounds = block.block.expandable.then_some(Bounds {
        origin: point(
            block_bounds.origin.x + block_bounds.size.width - px(88.0),
            block_bounds.origin.y + px(8.0),
        ),
        size: size(px(76.0), px(18.0)),
    });

    AiWorkspaceBlockRenderLayout {
        block_bounds,
        text_origin_x,
        title_origin_y,
        preview_origin_y,
        title_line_height,
        preview_line_height,
        title_char_width,
        preview_char_width,
        toggle_bounds,
    }
}

fn ai_workspace_paint_lines_for_block(
    block: &ai_workspace_session::AiWorkspaceViewportBlock,
    render_layout: &AiWorkspaceBlockRenderLayout,
    workspace_root: Option<&Path>,
) -> Vec<AiWorkspacePaintLine> {
    let title_surface_id = ai_workspace_title_surface_id(block.block.id.as_str());
    let preview_surface_id = ai_workspace_preview_surface_id(block.block.id.as_str());
    let title_surface_text = block.text_layout.title_lines.join("\n");
    let preview_surface_text = block.text_layout.preview_lines.join("\n");
    let mut lines = Vec::new();

    let mut title_offset = 0usize;
    for (line_index, line_text) in block.text_layout.title_lines.iter().enumerate() {
        let line_len = line_text.len();
        lines.push(AiWorkspacePaintLine {
            surface_id: title_surface_id.clone(),
            text: line_text.clone(),
            surface_byte_range: title_offset..title_offset.saturating_add(line_len),
            column_byte_offsets: Arc::<[usize]>::from(ai_workspace_column_byte_offsets(line_text)),
            link_ranges: Arc::<[MarkdownLinkRange]>::from(ai_workspace_link_ranges(
                line_text,
                workspace_root,
            )),
            origin: point(
                render_layout.text_origin_x,
                render_layout.title_origin_y + render_layout.title_line_height * line_index as f32,
            ),
            line_height: render_layout.title_line_height,
            char_width: render_layout.title_char_width,
            title: true,
            monospace: false,
        });
        title_offset = title_offset.saturating_add(line_len).saturating_add(1);
    }

    let preview_monospace =
        block.block.kind == ai_workspace_session::AiWorkspaceBlockKind::DiffSummary;
    let mut preview_offset = 0usize;
    for (line_index, line_text) in block.text_layout.preview_lines.iter().enumerate() {
        let line_len = line_text.len();
        lines.push(AiWorkspacePaintLine {
            surface_id: preview_surface_id.clone(),
            text: line_text.clone(),
            surface_byte_range: preview_offset..preview_offset.saturating_add(line_len),
            column_byte_offsets: Arc::<[usize]>::from(ai_workspace_column_byte_offsets(line_text)),
            link_ranges: Arc::<[MarkdownLinkRange]>::from(ai_workspace_link_ranges(
                line_text,
                workspace_root,
            )),
            origin: point(
                render_layout.text_origin_x,
                render_layout.preview_origin_y
                    + render_layout.preview_line_height * line_index as f32,
            ),
            line_height: render_layout.preview_line_height,
            char_width: render_layout.preview_char_width,
            title: false,
            monospace: preview_monospace,
        });
        preview_offset = preview_offset.saturating_add(line_len).saturating_add(1);
    }

    if title_surface_text.is_empty() && preview_surface_text.is_empty() {
        return Vec::new();
    }

    lines
}

fn ai_workspace_text_hit(
    block: &ai_workspace_session::AiWorkspaceViewportBlock,
    render_layout: &AiWorkspaceBlockRenderLayout,
    position: Point<Pixels>,
    workspace_root: Option<&Path>,
) -> Option<AiWorkspaceTextHit> {
    let lines = ai_workspace_paint_lines_for_block(block, render_layout, workspace_root);
    let line = lines.iter().find(|line| {
        let line_bounds = Bounds {
            origin: point(line.origin.x, line.origin.y),
            size: size(
                px(
                    (line.column_byte_offsets.len().saturating_sub(1) as f32 + 1.0)
                        * line.char_width.as_f32(),
                ),
                line.line_height,
            ),
        };
        position.y >= line_bounds.origin.y
            && position.y < line_bounds.origin.y + line_bounds.size.height
            && position.x >= render_layout.text_origin_x
            && position.x
                < render_layout.block_bounds.origin.x + render_layout.block_bounds.size.width
    })?;
    let relative_x = (position.x - render_layout.text_origin_x).max(Pixels::ZERO);
    let max_column = line.column_byte_offsets.len().saturating_sub(1);
    let column = ((relative_x / line.char_width).floor() as usize).min(max_column);
    let index = line.surface_byte_range.start + line.column_byte_offsets[column];
    let line_local_index = line.column_byte_offsets[column];
    let link_target = line
        .link_ranges
        .iter()
        .find(|range| range.range.contains(&line_local_index))
        .map(|range| range.raw_target.clone());

    Some(AiWorkspaceTextHit {
        surface_id: line.surface_id.clone(),
        index,
        link_target,
        selection_surfaces: ai_workspace_selection_surfaces_for_viewport_block(block),
    })
}

fn ai_workspace_selection_surfaces_for_viewport_block(
    block: &ai_workspace_session::AiWorkspaceViewportBlock,
) -> Arc<[AiTextSelectionSurfaceSpec]> {
    let mut surfaces = Vec::with_capacity(2);
    if !block.text_layout.title_lines.is_empty() {
        surfaces.push(AiTextSelectionSurfaceSpec::new(
            ai_workspace_title_surface_id(block.block.id.as_str()),
            block.text_layout.title_lines.join("\n"),
        ));
    }
    if !block.text_layout.preview_lines.is_empty() {
        let preview_surface = AiTextSelectionSurfaceSpec::new(
            ai_workspace_preview_surface_id(block.block.id.as_str()),
            block.text_layout.preview_lines.join("\n"),
        );
        surfaces.push(if surfaces.is_empty() {
            preview_surface
        } else {
            preview_surface.with_separator_before("\n")
        });
    }
    Arc::<[AiTextSelectionSurfaceSpec]>::from(surfaces)
}

fn ai_workspace_title_surface_id(block_id: &str) -> String {
    format!("ai-workspace:{block_id}:title")
}

fn ai_workspace_preview_surface_id(block_id: &str) -> String {
    format!("ai-workspace:{block_id}:preview")
}

fn ai_workspace_column_byte_offsets(text: &str) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(text.chars().count() + 1);
    offsets.push(0);
    for (byte_index, ch) in text.char_indices() {
        offsets.push(byte_index + ch.len_utf8());
    }
    offsets
}

fn ai_workspace_link_ranges(text: &str, workspace_root: Option<&Path>) -> Vec<MarkdownLinkRange> {
    let mut link_ranges = Vec::new();
    let mut segment_start = None;

    for (index, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if let Some(start) = segment_start.take() {
                ai_workspace_push_link_range(&mut link_ranges, text, start, index, workspace_root);
            }
            continue;
        }

        if segment_start.is_none() {
            segment_start = Some(index);
        }
    }

    if let Some(start) = segment_start {
        ai_workspace_push_link_range(&mut link_ranges, text, start, text.len(), workspace_root);
    }

    link_ranges
}

fn ai_workspace_push_link_range(
    link_ranges: &mut Vec<MarkdownLinkRange>,
    text: &str,
    start: usize,
    end: usize,
    workspace_root: Option<&Path>,
) {
    let Some((range, raw_target)) =
        ai_workspace_normalize_link_candidate(text, start..end, workspace_root)
    else {
        return;
    };

    if let Some(previous) = link_ranges.last_mut()
        && previous.raw_target == raw_target
        && previous.range.end == range.start
    {
        previous.range.end = range.end;
        return;
    }

    link_ranges.push(MarkdownLinkRange { range, raw_target });
}

fn ai_workspace_normalize_link_candidate(
    text: &str,
    mut range: Range<usize>,
    workspace_root: Option<&Path>,
) -> Option<(Range<usize>, String)> {
    let trimmed_start = text[range.clone()]
        .find(|ch: char| !matches!(ch, '(' | '[' | '{' | '<' | '"' | '\''))
        .map(|offset| range.start + offset)?;
    range.start = trimmed_start;

    let trimmed_slice = &text[range.clone()];
    let trimmed_end = trimmed_slice
        .trim_end_matches(|ch: char| {
            matches!(ch, '.' | ',' | ';' | ')' | ']' | '}' | '>' | '"' | '\'')
        })
        .len();
    range.end = range.start + trimmed_end;
    if range.is_empty() {
        return None;
    }

    let raw_target = text[range.clone()].to_string();
    resolve_markdown_link_target(raw_target.as_str(), workspace_root, None)
        .map(|_| (range, raw_target))
}

fn ai_workspace_paint_selection(
    line: &AiWorkspacePaintLine,
    selection_range: Option<Range<usize>>,
    window: &mut Window,
) {
    let Some(selection_range) = selection_range else {
        return;
    };
    let Some(local_range) = ai_workspace_line_selection_range(line, selection_range) else {
        return;
    };
    let Some((start_column, end_column)) =
        ai_workspace_selection_columns(line.column_byte_offsets.as_ref(), &local_range)
    else {
        return;
    };
    if start_column == end_column {
        return;
    }

    window.paint_quad(fill(
        Bounds {
            origin: point(
                line.origin.x + line.char_width * start_column as f32,
                line.origin.y,
            ),
            size: size(
                line.char_width * (end_column - start_column) as f32,
                line.line_height,
            ),
        },
        gpui::hsla(0.58, 0.64, 0.58, 0.18),
    ));
}

fn ai_workspace_line_selection_range(
    line: &AiWorkspacePaintLine,
    selection_range: Range<usize>,
) -> Option<Range<usize>> {
    let start = selection_range.start.max(line.surface_byte_range.start);
    let end = selection_range.end.min(line.surface_byte_range.end);
    (start < end).then(|| {
        start.saturating_sub(line.surface_byte_range.start)
            ..end.saturating_sub(line.surface_byte_range.start)
    })
}

fn ai_workspace_selection_columns(
    column_byte_offsets: &[usize],
    range: &Range<usize>,
) -> Option<(usize, usize)> {
    if range.is_empty() || column_byte_offsets.len() < 2 {
        return None;
    }

    let start_column = column_byte_offsets
        .partition_point(|offset| *offset <= range.start)
        .saturating_sub(1);
    let end_column = column_byte_offsets.partition_point(|offset| *offset < range.end);
    (start_column < end_column).then_some((start_column, end_column))
}

fn ai_workspace_text_runs_for_line(
    line: &AiWorkspacePaintLine,
    default_color: gpui::Hsla,
    link_color: gpui::Hsla,
    font: Font,
) -> Vec<TextRun> {
    if line.link_ranges.is_empty() {
        return vec![single_color_text_run(
            line.text.len().max(1),
            default_color,
            font,
        )];
    }

    let mut runs = Vec::new();
    let mut cursor = 0usize;
    for link_range in line.link_ranges.iter() {
        if link_range.range.start > cursor {
            runs.push(single_color_text_run(
                link_range.range.start - cursor,
                default_color,
                font.clone(),
            ));
        }
        runs.push(TextRun {
            len: link_range.range.len(),
            color: link_color,
            font: font.clone(),
            background_color: None,
            underline: Some(gpui::UnderlineStyle {
                thickness: px(1.0),
                color: Some(link_color),
                wavy: false,
            }),
            strikethrough: None,
        });
        cursor = link_range.range.end;
    }

    if cursor < line.text.len() {
        runs.push(single_color_text_run(
            line.text.len() - cursor,
            default_color,
            font,
        ));
    }

    runs
}

fn ai_workspace_block_palette(
    kind: ai_workspace_session::AiWorkspaceBlockKind,
    role: ai_workspace_session::AiWorkspaceBlockRole,
    selected: bool,
    is_dark: bool,
    cx: &App,
) -> (
    gpui::Hsla,
    gpui::Hsla,
    gpui::Hsla,
    gpui::Hsla,
    gpui::Hsla,
    gpui::Hsla,
) {
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
    let link_color = crate::app::theme::hunk_opacity(cx.theme().accent, is_dark, 0.98, 0.88);
    (
        background,
        border,
        accent,
        title_color,
        preview_color,
        link_color,
    )
}
