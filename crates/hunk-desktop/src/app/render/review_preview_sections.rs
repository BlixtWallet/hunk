struct ReviewPreviewDiffCellRenderSpec<'a> {
    row_ix: usize,
    side: &'static str,
    cell: &'a DiffCell,
    peer_kind: DiffCellKind,
    panel_width: Option<Pixels>,
    editor_font_size: Pixels,
}

const REVIEW_PREVIEW_MAX_TEXT_CHARS_PER_CELL: usize = 240;

impl DiffViewer {
    #[allow(clippy::too_many_arguments)]
    fn render_review_preview_section(
        &self,
        file: ChangedFile,
        index: usize,
        total_files: usize,
        status_label: &'static str,
        status_color: Hsla,
        border_color: Hsla,
        is_active: bool,
        preview_section: crate::app::review_preview_model::ReviewPreviewSection,
        hunk_count: usize,
        line_stats: LineStats,
        editor_loading: bool,
        editor_error: Option<String>,
        editor_font_size: gpui::Pixels,
        _layout: Option<DiffColumnLayout>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let is_dark = cx.theme().mode.is_dark();
        let preview_rows = self.render_review_preview_rows(&preview_section, editor_font_size, cx);
        let preview_truncated = preview_section.truncated();
        let hidden_rows = preview_section.hidden_row_count();
        let hidden_hunks = preview_section.hidden_hunk_count();

        v_flex()
            .id(("review-editor-section", index))
            .w_full()
            .pb_3()
            .child(
                v_flex()
                    .w_full()
                    .rounded(px(10.0))
                    .border_1()
                    .border_color(border_color)
                    .bg(hunk_blend(
                        cx.theme().background,
                        cx.theme().muted,
                        is_dark,
                        0.10,
                        0.04,
                    ))
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .px_3()
                            .py_2()
                            .border_b_1()
                            .border_color(hunk_opacity(cx.theme().border, is_dark, 0.82, 0.68))
                            .on_mouse_down(MouseButton::Left, {
                                let view = cx.entity();
                                let path = file.path.clone();
                                move |_, window, cx| {
                                    view.update(cx, |this, cx| {
                                        this.review_editor_activate_path(path.clone(), Some(window), cx);
                                    });
                                }
                            })
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .px_1p5()
                                            .py_0p5()
                                            .rounded(px(6.0))
                                            .bg(hunk_opacity(status_color, is_dark, 0.20, 0.12))
                                            .text_xs()
                                            .font_semibold()
                                            .text_color(status_color)
                                            .child(status_label),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_family(cx.theme().mono_font_family.clone())
                                            .text_color(cx.theme().foreground)
                                            .truncate()
                                            .child(file.path),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_3()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("{}/{} files", index + 1, total_files))
                                    .child(format!("{hunk_count} hunks"))
                                    .child(self.render_line_stats("diff", line_stats, cx))
                                    .when(is_active && editor_loading, |this| {
                                        this.child(
                                            div()
                                                .px_2()
                                                .py_1()
                                                .rounded(px(6.0))
                                                .bg(hunk_opacity(
                                                    cx.theme().warning,
                                                    is_dark,
                                                    0.18,
                                                    0.14,
                                                ))
                                                .text_xs()
                                                .text_color(cx.theme().warning)
                                                .child("Preparing editor..."),
                                        )
                                    })
                                    .when_some(editor_error.as_ref(), |this, _| {
                                        this.child(
                                            div()
                                                .px_2()
                                                .py_1()
                                                .rounded(px(6.0))
                                                .bg(hunk_opacity(
                                                    cx.theme().danger,
                                                    is_dark,
                                                    0.18,
                                                    0.14,
                                                ))
                                                .text_xs()
                                                .text_color(cx.theme().danger)
                                                .child("Editor unavailable"),
                                        )
                                    }),
                            ),
                    )
                    .child(
                        v_flex()
                            .w_full()
                            .children(preview_rows)
                            .when(preview_truncated, |this| {
                                this.child(
                                    h_flex()
                                        .w_full()
                                        .items_center()
                                        .justify_between()
                                        .gap_3()
                                        .px_3()
                                        .py_2()
                                        .border_t_1()
                                        .border_color(hunk_opacity(
                                            cx.theme().border,
                                            is_dark,
                                            0.82,
                                            0.68,
                                        ))
                                        .bg(hunk_blend(
                                            cx.theme().background,
                                            cx.theme().muted,
                                            is_dark,
                                            0.08,
                                            0.04,
                                        ))
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(format!(
                                                    "{hidden_rows} rows and {hidden_hunks} hunks remain collapsed in preview."
                                                )),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child("Open this section to edit"),
                                        ),
                                )
                            })
                            .when_some(editor_error, |this, error| {
                                this.child(
                                    div()
                                        .w_full()
                                        .px_3()
                                        .py_2()
                                        .border_t_1()
                                        .border_color(hunk_opacity(
                                            cx.theme().border,
                                            is_dark,
                                            0.82,
                                            0.68,
                                        ))
                                        .bg(hunk_opacity(
                                            cx.theme().danger,
                                            is_dark,
                                            0.08,
                                            0.05,
                                        ))
                                        .text_xs()
                                        .text_color(cx.theme().danger)
                                        .child(error),
                                )
                            }),
                    ),
            )
            .into_any_element()
    }

    fn render_review_preview_rows(
        &self,
        section: &crate::app::review_preview_model::ReviewPreviewSection,
        editor_font_size: Pixels,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        if section.rendered_row_indices.is_empty() {
            return Vec::new();
        }

        section
            .rendered_row_indices
            .iter()
            .filter_map(|row_ix| {
                let row = self.diff_rows.get(*row_ix)?;
                Some(match row.kind {
                    DiffRowKind::Code => {
                        self.render_review_preview_code_row(*row_ix, row, editor_font_size, cx)
                    }
                    DiffRowKind::HunkHeader | DiffRowKind::Meta | DiffRowKind::Empty => {
                        self.render_review_preview_meta_row(*row_ix, row, editor_font_size, cx)
                    }
                })
            })
            .collect()
    }

    fn render_review_preview_meta_row(
        &self,
        row_ix: usize,
        row: &SideBySideRow,
        editor_font_size: Pixels,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let stable_row_id = self.diff_row_stable_id(row_ix);
        let is_dark = cx.theme().mode.is_dark();

        if row.kind == DiffRowKind::HunkHeader {
            return div()
                .id(("review-preview-hunk-divider-row", stable_row_id))
                .h(px(6.0))
                .border_b_1()
                .border_color(hunk_opacity(cx.theme().border, is_dark, 0.92, 0.70))
                .bg(hunk_opacity(cx.theme().muted, is_dark, 0.26, 0.40))
                .w_full()
                .into_any_element();
        }

        let (background, foreground, accent) = match row.kind {
            DiffRowKind::Meta => {
                let line = row.text.as_str();
                if line.starts_with("new file mode") || line.starts_with("+++ b/") {
                    (
                        hunk_blend(cx.theme().background, cx.theme().success, is_dark, 0.22, 0.12),
                        hunk_tone(cx.theme().success, is_dark, 0.45, 0.10),
                        cx.theme().success,
                    )
                } else if line.starts_with("deleted file mode") || line.starts_with("--- a/") {
                    (
                        hunk_blend(cx.theme().background, cx.theme().danger, is_dark, 0.22, 0.12),
                        hunk_tone(cx.theme().danger, is_dark, 0.45, 0.10),
                        cx.theme().danger,
                    )
                } else if line.starts_with("diff --git") {
                    (
                        hunk_blend(cx.theme().background, cx.theme().accent, is_dark, 0.18, 0.10),
                        cx.theme().foreground,
                        cx.theme().accent,
                    )
                } else {
                    (
                        cx.theme().muted,
                        cx.theme().muted_foreground,
                        cx.theme().border,
                    )
                }
            }
            DiffRowKind::Empty => (
                cx.theme().background,
                cx.theme().muted_foreground,
                cx.theme().border,
            ),
            DiffRowKind::Code | DiffRowKind::HunkHeader => unreachable!(),
        };

        div()
            .id(("review-preview-meta-row", stable_row_id))
            .relative()
            .overflow_x_hidden()
            .px_3()
            .py_0p5()
            .border_b_1()
            .border_color(hunk_opacity(cx.theme().border, is_dark, 0.82, 0.70))
            .bg(background)
            .text_size(editor_font_size)
            .line_height(gpui::relative(1.45))
            .text_color(foreground)
            .font_family(cx.theme().mono_font_family.clone())
            .w_full()
            .whitespace_normal()
            .child(row.text.clone())
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .bottom_0()
                    .w(px(2.0))
                    .bg(accent),
            )
            .into_any_element()
    }

    fn render_review_preview_code_row(
        &self,
        row_ix: usize,
        row: &SideBySideRow,
        editor_font_size: Pixels,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let stable_row_id = self.diff_row_stable_id(row_ix);
        let layout = self.diff_column_layout();
        let chrome = hunk_diff_chrome(cx.theme(), cx.theme().mode.is_dark());

        h_flex()
            .id(("review-preview-code-row", stable_row_id))
            .relative()
            .overflow_x_hidden()
            .items_stretch()
            .border_b_1()
            .border_color(chrome.row_divider)
            .w_full()
            .child(self.render_review_preview_cell(
                stable_row_id,
                ReviewPreviewDiffCellRenderSpec {
                    row_ix,
                    side: "left",
                    cell: &row.left,
                    peer_kind: row.right.kind,
                    panel_width: layout.map(|layout| layout.left_panel_width),
                    editor_font_size,
                },
                cx,
            ))
            .child(self.render_review_preview_cell(
                stable_row_id,
                ReviewPreviewDiffCellRenderSpec {
                    row_ix,
                    side: "right",
                    cell: &row.right,
                    peer_kind: row.left.kind,
                    panel_width: layout.map(|layout| layout.right_panel_width),
                    editor_font_size,
                },
                cx,
            ))
            .into_any_element()
    }

    fn render_review_preview_cell(
        &self,
        row_stable_id: u64,
        spec: ReviewPreviewDiffCellRenderSpec<'_>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let side = spec.side;
        let cell = spec.cell;
        let peer_kind = spec.peer_kind;
        let cell_id = if side == "left" {
            ("review-preview-cell-left", row_stable_id)
        } else {
            ("review-preview-cell-right", row_stable_id)
        };

        let is_dark = cx.theme().mode.is_dark();
        let chrome = hunk_diff_chrome(cx.theme(), is_dark);
        let dark_add_tint: gpui::Hsla = gpui::rgb(0x2e4736).into();
        let dark_remove_tint: gpui::Hsla = gpui::rgb(0x4a3038).into();
        let dark_add_accent: gpui::Hsla = gpui::rgb(0x8fcea0).into();
        let dark_remove_accent: gpui::Hsla = gpui::rgb(0xeea9b4).into();

        let (mut background, marker_color, line_color, text_color, marker) =
            match (cell.kind, peer_kind) {
                (DiffCellKind::Added, _) => (
                    hunk_pick(
                        is_dark,
                        cx.theme().background.blend(dark_add_tint.opacity(0.62)),
                        hunk_blend(cx.theme().background, cx.theme().success, is_dark, 0.24, 0.11),
                    ),
                    hunk_pick(is_dark, dark_add_accent, cx.theme().success.darken(0.18)),
                    hunk_pick(
                        is_dark,
                        dark_add_accent.lighten(0.08),
                        cx.theme().success.darken(0.16),
                    ),
                    cx.theme().foreground,
                    "+",
                ),
                (DiffCellKind::Removed, _) => (
                    hunk_pick(
                        is_dark,
                        cx.theme().background.blend(dark_remove_tint.opacity(0.62)),
                        hunk_blend(cx.theme().background, cx.theme().danger, is_dark, 0.24, 0.11),
                    ),
                    hunk_pick(is_dark, dark_remove_accent, cx.theme().danger.darken(0.18)),
                    hunk_pick(
                        is_dark,
                        dark_remove_accent.lighten(0.06),
                        cx.theme().danger.darken(0.16),
                    ),
                    cx.theme().foreground,
                    "-",
                ),
                (DiffCellKind::Context, _) => (
                    cx.theme().background,
                    hunk_tone(cx.theme().muted_foreground, is_dark, 0.14, 0.10),
                    hunk_tone(cx.theme().muted_foreground, is_dark, 0.18, 0.12),
                    cx.theme().foreground,
                    "",
                ),
                (DiffCellKind::None, _) => (
                    cx.theme().background,
                    hunk_tone(cx.theme().muted_foreground, is_dark, 0.14, 0.10),
                    hunk_tone(cx.theme().muted_foreground, is_dark, 0.18, 0.12),
                    hunk_tone(cx.theme().muted_foreground, is_dark, 0.08, 0.06),
                    "",
                ),
            };

        if matches!(cell.kind, DiffCellKind::Context | DiffCellKind::None)
            && row_stable_id.is_multiple_of(2)
        {
            background = hunk_blend(background, cx.theme().muted, is_dark, 0.06, 0.10);
        }

        let line_number = cell.line.map(|line| line.to_string()).unwrap_or_default();
        let preview_text = self.review_preview_text(&cell.text);
        let editor_font_size = spec.editor_font_size;
        let line_number_width = if side == "left" {
            self.diff_left_line_number_width
        } else {
            self.diff_right_line_number_width
        };
        let cached_row_segments = self
            .diff_row_segment_cache
            .get(spec.row_ix)
            .and_then(Option::as_ref);
        let segment_cache = if side == "left" {
            cached_row_segments.map(|segments| &segments.left)
        } else {
            cached_row_segments.map(|segments| &segments.right)
        };
        let render_syntax = self.last_scroll_activity_at.elapsed() >= AUTO_REFRESH_SCROLL_DEBOUNCE;
        let should_draw_right_divider = side == "left";
        let gutter_background = match cell.kind {
            DiffCellKind::Added => {
                hunk_blend(cx.theme().background, cx.theme().success, is_dark, 0.12, 0.07)
            }
            DiffCellKind::Removed => {
                hunk_blend(cx.theme().background, cx.theme().danger, is_dark, 0.12, 0.07)
            }
            DiffCellKind::None => hunk_blend(cx.theme().background, cx.theme().muted, is_dark, 0.04, 0.06),
            DiffCellKind::Context => hunk_blend(cx.theme().background, cx.theme().muted, is_dark, 0.06, 0.08),
        };
        let gutter_width = line_number_width + DIFF_MARKER_GUTTER_WIDTH + 16.0;

        h_flex()
            .id(cell_id)
            .overflow_x_hidden()
            .items_stretch()
            .bg(background)
            .when_some(spec.panel_width, |this, width| {
                this.w(width).min_w(width).max_w(width).flex_none()
            })
            .when(spec.panel_width.is_none(), |this| this.flex_1().min_w_0())
            .when(should_draw_right_divider, |this| {
                this.border_r_1().border_color(chrome.center_divider)
            })
            .child(
                h_flex()
                    .items_start()
                    .gap_1()
                    .w(px(gutter_width))
                    .min_w(px(gutter_width))
                    .px_2()
                    .py_0p5()
                    .bg(gutter_background)
                    .border_r_1()
                    .border_color(chrome.center_divider)
                    .child(
                        h_flex()
                            .w(px(line_number_width))
                            .justify_end()
                            .child(
                                div()
                                    .text_size(editor_font_size)
                                    .line_height(gpui::relative(1.45))
                                    .text_color(line_color)
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .whitespace_nowrap()
                                    .child(line_number),
                            ),
                    )
                    .child(
                        h_flex()
                            .w(px(DIFF_MARKER_GUTTER_WIDTH))
                            .justify_center()
                            .child(
                                div()
                                    .text_size(editor_font_size)
                                    .line_height(gpui::relative(1.45))
                                    .text_color(marker_color)
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .whitespace_nowrap()
                                    .child(marker),
                            ),
                    ),
            )
            .child(
                if render_syntax
                    && cached_row_segments
                        .is_some_and(|cache| cache.quality >= DiffSegmentQuality::SyntaxOnly)
                {
                    h_flex()
                        .flex_1()
                        .min_w_0()
                        .items_start()
                        .gap_0()
                        .px_2()
                        .py_0p5()
                        .text_size(editor_font_size)
                        .line_height(gpui::relative(1.45))
                        .font_family(cx.theme().mono_font_family.clone())
                        .text_color(text_color)
                        .overflow_x_hidden()
                        .flex_wrap()
                        .whitespace_normal()
                        .children(segment_cache.into_iter().flat_map(|segments| {
                            segments.iter().map(|segment| {
                                let segment_color =
                                    diff_syntax_color(cx.theme(), text_color, segment.syntax);
                                div()
                                    .flex_none()
                                    .whitespace_nowrap()
                                    .text_color(segment_color)
                                    .when(segment.changed, |this| {
                                        this.bg(hunk_opacity(marker_color, is_dark, 0.20, 0.11))
                                    })
                                    .child(segment.plain_text.clone())
                            })
                        }))
                        .into_any_element()
                } else {
                    div()
                        .flex_1()
                        .min_w_0()
                        .px_2()
                        .py_0p5()
                        .text_size(editor_font_size)
                        .line_height(gpui::relative(1.45))
                        .font_family(cx.theme().mono_font_family.clone())
                        .text_color(text_color)
                        .overflow_x_hidden()
                        .whitespace_nowrap()
                        .child(preview_text)
                        .into_any_element()
                },
            )
            .into_any_element()
    }

    fn review_preview_text(&self, text: &str) -> SharedString {
        if text.chars().count() <= REVIEW_PREVIEW_MAX_TEXT_CHARS_PER_CELL {
            return SharedString::from(text.to_owned());
        }

        let truncated = text
            .chars()
            .take(REVIEW_PREVIEW_MAX_TEXT_CHARS_PER_CELL.saturating_sub(1))
            .collect::<String>();
        SharedString::from(format!("{truncated}..."))
    }
}
