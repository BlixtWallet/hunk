impl DiffViewer {
    fn review_preview_section(
        &self,
        path: &str,
    ) -> Option<&crate::app::review_preview_model::ReviewPreviewSection> {
        self.review_preview_sections.get(path)
    }

    fn review_preview_hunk_count(&self, path: &str) -> usize {
        self.review_preview_section(path)
            .map(|section| section.total_hunk_count)
            .unwrap_or(0)
    }

    fn render_review_editor_preview(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if let Some(error) = self.review_compare_error.clone() {
            return v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .p_4()
                .child(div().text_sm().text_color(cx.theme().danger).child(error))
                .into_any_element();
        }

        if self.review_compare_loading && self.review_files.is_empty() {
            return v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Loading compared files..."),
                )
                .into_any_element();
        }

        if self.review_files.is_empty() {
            return v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("No compared files."),
                )
                .into_any_element();
        }

        let is_dark = cx.theme().mode.is_dark();
        let layout = self.diff_column_layout();
        let editor_font_size = self.workspace_editor_font_size(cx);
        let view = cx.entity();
        let is_review_editor_focused = self.review_editor_focus_handle.is_focused(window);
        let total_files = self.review_files.len();
        let list_state = self.review_editor_list_state.clone();
        let review_list = list(list_state.clone(), {
            cx.processor(move |this, ix: usize, window, cx| {
                let Some(file) = this.review_files.get(ix).cloned() else {
                    return div().w_full().h(px(0.0)).into_any_element();
                };
                this.render_review_editor_section(
                    file,
                    ix,
                    total_files,
                    layout,
                    editor_font_size,
                    is_dark,
                    is_review_editor_focused,
                    window,
                    cx,
                )
            })
        })
        .size_full()
        .with_sizing_behavior(ListSizingBehavior::Auto);

        v_flex()
            .flex_1()
            .min_h_0()
            .items_stretch()
            .track_focus(&self.review_editor_focus_handle)
            .key_context("ReviewEditor DiffWorkspace")
            .on_action(cx.listener(Self::review_editor_copy_action))
            .on_action(cx.listener(Self::review_editor_cut_action))
            .on_action(cx.listener(Self::review_editor_paste_action))
            .on_action(cx.listener(Self::review_editor_move_up_action))
            .on_action(cx.listener(Self::review_editor_move_down_action))
            .on_action(cx.listener(Self::review_editor_move_left_action))
            .on_action(cx.listener(Self::review_editor_move_right_action))
            .on_action(cx.listener(Self::review_editor_select_up_action))
            .on_action(cx.listener(Self::review_editor_select_down_action))
            .on_action(cx.listener(Self::review_editor_select_left_action))
            .on_action(cx.listener(Self::review_editor_select_right_action))
            .on_action(cx.listener(Self::review_editor_move_to_beginning_of_line_action))
            .on_action(cx.listener(Self::review_editor_move_to_end_of_line_action))
            .on_action(cx.listener(Self::review_editor_move_to_beginning_of_document_action))
            .on_action(cx.listener(Self::review_editor_move_to_end_of_document_action))
            .on_action(cx.listener(Self::review_editor_select_to_beginning_of_line_action))
            .on_action(cx.listener(Self::review_editor_select_to_end_of_line_action))
            .on_action(cx.listener(Self::review_editor_select_to_beginning_of_document_action))
            .on_action(cx.listener(Self::review_editor_select_to_end_of_document_action))
            .on_action(cx.listener(Self::review_editor_move_to_previous_word_start_action))
            .on_action(cx.listener(Self::review_editor_move_to_next_word_end_action))
            .on_action(cx.listener(Self::review_editor_select_to_previous_word_start_action))
            .on_action(cx.listener(Self::review_editor_select_to_next_word_end_action))
            .on_action(cx.listener(Self::review_editor_page_up_action))
            .on_action(cx.listener(Self::review_editor_page_down_action))
            .on_mouse_down(MouseButton::Left, {
                let view = view.clone();
                move |_, window, cx| {
                    view.update(cx, |this, cx| {
                        this.review_editor_focus_handle.focus(window, cx);
                    });
                }
            })
            .on_key_down({
                let view = view.clone();
                move |event, window, cx| {
                    let handled = view.update(cx, |this, cx| {
                        if !this.review_editor_focus_handle.is_focused(window)
                            || is_desktop_clipboard_shortcut(&event.keystroke)
                        {
                            return false;
                        }
                        if uses_review_editor_action_dispatch(&event.keystroke) {
                            return false;
                        }
                        this.review_editor_handle_keystroke(&event.keystroke, cx)
                    });
                    if handled {
                        cx.stop_propagation();
                    }
                }
            })
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(hunk_opacity(cx.theme().border, is_dark, 0.86, 0.70))
                    .bg(hunk_blend(
                        cx.theme().background,
                        cx.theme().muted,
                        is_dark,
                        0.14,
                        0.08,
                    ))
                    .child(
                        div()
                            .text_sm()
                            .font_family(cx.theme().mono_font_family.clone())
                            .text_color(cx.theme().foreground)
                            .child("Continuous Review"),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_3()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{total_files} files"))
                            .child(format!(
                                "{} -> {}",
                                self.review_compare_source_label(self.review_left_source_id.as_deref()),
                                self.review_compare_source_label(self.review_right_source_id.as_deref())
                            )),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .child(review_list)
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .right_0()
                            .bottom_0()
                            .w(px(16.0))
                            .child(
                                Scrollbar::vertical(&list_state)
                                    .scrollbar_show(ScrollbarShow::Always),
                            ),
                    ),
            )
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_review_editor_section(
        &self,
        file: ChangedFile,
        index: usize,
        total_files: usize,
        layout: Option<DiffColumnLayout>,
        editor_font_size: gpui::Pixels,
        is_dark: bool,
        is_review_editor_focused: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let is_active = self.selected_path.as_deref() == Some(file.path.as_str());
        let (status_label, status_color) = change_status_label_color(file.status, cx);
        let line_stats = self
            .review_file_line_stats
            .get(file.path.as_str())
            .copied()
            .unwrap_or_default();
        let preview_section = self
            .review_preview_section(file.path.as_str())
            .cloned()
            .unwrap_or_default();
        let preview_hunk_count = self.review_preview_hunk_count(file.path.as_str());
        let border_color = if is_active {
            hunk_opacity(cx.theme().accent, is_dark, 0.82, 0.62)
        } else {
            hunk_opacity(cx.theme().border, is_dark, 0.86, 0.72)
        };

        let session_loading = is_active
            && self
                .review_editor_session(file.path.as_str())
                .is_some_and(|session| {
                    session.loading || session.presentation_loading || session.save_loading
                });
        let session_error = if is_active {
            self.review_editor_session(file.path.as_str())
                .and_then(|session| session.error.clone())
        } else {
            None
        };
        let content_loaded = is_active
            && self
                .review_editor_session(file.path.as_str())
                .is_some_and(|session| {
                    session.left_source_id == self.review_left_source_id
                        && session.right_source_id == self.review_right_source_id
                        && !session.loading
                        && session.error.is_none()
                });

        if !content_loaded {
            return self.render_review_preview_section(
                file,
                index,
                total_files,
                status_label,
                status_color,
                border_color,
                is_active,
                preview_section,
                preview_hunk_count,
                line_stats,
                session_loading,
                session_error,
                editor_font_size,
                layout,
                cx,
            );
        }

        let Some(session) = self.review_editor_session(file.path.as_str()) else {
            return div().into_any_element();
        };
        let editor_height = self.review_editor_section_height(file.path.as_str(), editor_font_size);
        let hunk_count = session.right_hunk_lines.len();

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
                                            .child(file.path.clone()),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_3()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("{}/{} files", index + 1, total_files))
                                    .child(format!("{hunk_count} hunks")),
                            ),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .items_stretch()
                            .relative()
                            .h(editor_height)
                            .min_h(editor_height)
                            .max_h(editor_height)
                            .child(
                                self.render_review_editor_side(
                                    file.path.clone(),
                                    "review-editor-left",
                                    session.left_editor.clone(),
                                    content_loaded && !session.left_present,
                                    "Missing in base",
                                    editor_font_size,
                                    is_dark,
                                    layout.map(|layout| layout.left_panel_width),
                                    false,
                                    false,
                                    editor_height,
                                    cx,
                                ),
                            )
                            .child(
                                self.render_review_editor_side(
                                    file.path.clone(),
                                    "review-editor-right",
                                    session.right_editor.clone(),
                                    content_loaded && !session.right_present,
                                    "Missing in compare",
                                    editor_font_size,
                                    is_dark,
                                    layout.map(|layout| layout.right_panel_width),
                                    is_review_editor_focused && is_active,
                                    true,
                                    editor_height,
                                    cx,
                                ),
                            )
                            .when(
                                session.loading
                                    || session.presentation_loading
                                    || session.save_loading,
                                |this| {
                                    this.child(
                                        h_flex()
                                            .absolute()
                                            .top_2()
                                            .right_3()
                                            .gap_2()
                                            .when(session.loading, |this| {
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
                                                        .child("Refreshing..."),
                                                )
                                            })
                                            .when(session.presentation_loading, |this| {
                                                this.child(
                                                    div()
                                                        .px_2()
                                                        .py_1()
                                                        .rounded(px(6.0))
                                                        .bg(hunk_opacity(
                                                            cx.theme().accent,
                                                            is_dark,
                                                            0.18,
                                                            0.14,
                                                        ))
                                                        .text_xs()
                                                        .text_color(cx.theme().accent)
                                                        .child("Updating diff..."),
                                                )
                                            })
                                            .when(session.save_loading, |this| {
                                                this.child(
                                                    div()
                                                        .px_2()
                                                        .py_1()
                                                        .rounded(px(6.0))
                                                        .bg(hunk_opacity(
                                                            cx.theme().accent,
                                                            is_dark,
                                                            0.18,
                                                            0.14,
                                                        ))
                                                        .text_xs()
                                                        .text_color(cx.theme().accent)
                                                        .child("Saving..."),
                                                )
                                            }),
                                    )
                                },
                            )
                            .when_some(session.error.clone(), |this, error| {
                                this.child(
                                    v_flex()
                                        .absolute()
                                        .inset_0()
                                        .items_center()
                                        .justify_center()
                                        .bg(hunk_opacity(
                                            cx.theme().background,
                                            is_dark,
                                            0.86,
                                            0.90,
                                        ))
                                        .child(
                                            div()
                                                .max_w(px(520.0))
                                                .px_4()
                                                .text_sm()
                                                .text_color(cx.theme().danger)
                                                .child(error),
                                        ),
                                )
                            }),
                    ),
            )
            .into_any_element()
    }

    fn review_editor_section_height(
        &self,
        path: &str,
        editor_font_size: gpui::Pixels,
    ) -> gpui::Pixels {
        let line_height = (editor_font_size * 1.45).max(px(14.0));
        let Some(session) = self.review_editor_session(path) else {
            return line_height * 8.;
        };
        let row_count = session
            .left_editor
            .borrow()
            .display_row_count()
            .max(session.right_editor.borrow().display_row_count())
            .max(1);
        line_height * row_count as f32 + px(2.0)
    }

    #[allow(clippy::too_many_arguments)]
    fn render_review_editor_side(
        &self,
        path: String,
        _element_id_prefix: &'static str,
        editor: crate::app::native_files_editor::SharedFilesEditor,
        show_missing_badge: bool,
        missing_message: &'static str,
        editor_font_size: gpui::Pixels,
        is_dark: bool,
        width: Option<gpui::Pixels>,
        is_focused: bool,
        editable: bool,
        height: gpui::Pixels,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let editor_chrome = crate::app::theme::hunk_editor_chrome_colors(cx.theme(), is_dark);
        let view = cx.entity();
        let element = self.workspace_editor_element(
            editor.clone(),
            {
                let path = path.clone();
                move |target, position, window, cx| {
                    if !editable {
                        return;
                    }
                    view.update(cx, |this, cx| {
                        this.review_editor_activate_path(path.clone(), Some(window), cx);
                        this.open_workspace_text_context_menu(
                            WorkspaceTextContextMenuTarget::ReviewEditor(
                                ReviewEditorContextMenuTarget {
                                    path: path.clone(),
                                    can_cut: target.can_cut,
                                    can_copy: target.can_copy,
                                    can_paste: target.can_paste,
                                    can_select_all: target.can_select_all,
                                },
                            ),
                            position,
                            cx,
                        );
                    });
                }
            },
            is_focused,
            editor_font_size,
            is_dark,
            cx,
        );

        v_flex()
            .flex_1()
            .min_h(height)
            .max_h(height)
            .h(height)
            .items_stretch()
            .relative()
            .when_some(width, |this, width| {
                this.w(width).min_w(width).max_w(width).flex_none()
            })
            .bg(editor_chrome.background)
            .on_mouse_down(MouseButton::Left, {
                let view = cx.entity();
                let path = path.clone();
                move |_, window, cx| {
                    view.update(cx, |this, cx| {
                        this.review_editor_activate_path(path.clone(), Some(window), cx);
                    });
                }
            })
            .child(
                div()
                    .h(height)
                    .min_h(height)
                    .max_h(height)
                    .child(element),
            )
            .when(show_missing_badge, |this| {
                this.child(
                    div()
                        .absolute()
                        .top_2()
                        .right_2()
                        .px_2()
                        .py_1()
                        .rounded(px(6.0))
                        .bg(crate::app::theme::hunk_opacity(
                            editor_chrome.line_number,
                            is_dark,
                            0.14,
                            0.10,
                        ))
                        .text_xs()
                        .text_color(editor_chrome.line_number)
                        .child(missing_message),
                )
            })
            .when(editable, |this| {
                let is_comment_editor_open =
                    self.active_review_editor_comment_line.is_some()
                        && self.selected_path.as_deref() == Some(path.as_str());
                let note_id = self
                    .review_files
                    .iter()
                    .position(|file| file.path == path)
                    .unwrap_or(0);
                this.child(
                    h_flex()
                        .absolute()
                        .top_2()
                        .left_2()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .px_2()
                                .py_1()
                                .rounded(px(6.0))
                                .bg(crate::app::theme::hunk_opacity(
                                    cx.theme().success,
                                    is_dark,
                                    0.14,
                                    0.10,
                                ))
                                .text_xs()
                                .text_color(cx.theme().success)
                                .child("Live"),
                        )
                        .when(self.review_editor_supports_comments(), |this| {
                            let view = cx.entity();
                            let path = path.clone();
                            this.child(
                                Button::new(("review-editor-note", note_id))
                                    .compact()
                                    .outline()
                                    .rounded(px(6.0))
                                    .label("Note")
                                    .on_click(move |_, window, cx| {
                                        view.update(cx, |this, cx| {
                                            this.review_editor_activate_path(
                                                path.clone(),
                                                Some(window),
                                                cx,
                                            );
                                            this.open_review_editor_comment_editor(window, cx);
                                        });
                                    }),
                            )
                        }),
                )
                .when(is_comment_editor_open, |this| {
                    this.child(self.render_review_editor_comment_editor(cx))
                })
            })
            .into_any_element()
    }

    fn render_review_editor_comment_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(line_ix) = self.active_review_editor_comment_line else {
            return div().into_any_element();
        };

        let view = cx.entity();
        let anchor = self.build_review_editor_comment_anchor(line_ix);
        let file_path = anchor
            .as_ref()
            .map(|anchor| anchor.file_path.clone())
            .or_else(|| self.active_review_editor_path().map(ToOwned::to_owned))
            .unwrap_or_else(|| "file".to_string());
        let line_hint = anchor.as_ref().map_or_else(
            || "old - | new -".to_string(),
            |anchor| {
                format!(
                    "old {} | new {}",
                    anchor
                        .old_line
                        .map(|line| line.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    anchor
                        .new_line
                        .map(|line| line.to_string())
                        .unwrap_or_else(|| "-".to_string())
                )
            },
        );
        let is_dark = cx.theme().mode.is_dark();

        v_flex()
            .absolute()
            .top(px(44.0))
            .left_2()
            .w(px(380.0))
            .max_w(px(420.0))
            .gap_2()
            .px_2p5()
            .py_2()
            .rounded(px(9.0))
            .border_1()
            .border_color(hunk_opacity(cx.theme().border, is_dark, 0.90, 0.74))
            .bg(hunk_blend(cx.theme().popover, cx.theme().muted, is_dark, 0.16, 0.10))
            .child(
                v_flex()
                    .gap_0p5()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child(file_path),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(line_hint),
                    ),
            )
            .child(
                Input::new(&self.comment_input_state)
                    .rounded(px(8.0))
                    .h(px(64.0))
                    .border_1()
                    .border_color(hunk_opacity(cx.theme().border, is_dark, 0.88, 0.72))
                    .bg(hunk_blend(cx.theme().background, cx.theme().muted, is_dark, 0.20, 0.08)),
            )
            .child(
                h_flex()
                    .items_center()
                    .justify_end()
                    .gap_2()
                    .child({
                        let view = view.clone();
                        Button::new("review-editor-comment-cancel")
                            .compact()
                            .outline()
                            .rounded(px(7.0))
                            .label("Cancel")
                            .on_click(move |_, window, cx| {
                                view.update(cx, |this, cx| {
                                    this.cancel_review_editor_comment_editor(window, cx);
                                });
                            })
                    })
                    .child({
                        let view = view.clone();
                        Button::new("review-editor-comment-save")
                            .compact()
                            .primary()
                            .rounded(px(7.0))
                            .label("Save Comment")
                            .on_click(move |_, window, cx| {
                                view.update(cx, |this, cx| {
                                    this.save_active_review_editor_comment(window, cx);
                                });
                            })
                    }),
            )
            .when_some(self.comment_status_message.as_ref(), |this, message| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(message.clone()),
                )
            })
            .into_any_element()
    }
}

fn uses_review_editor_action_dispatch(keystroke: &Keystroke) -> bool {
    match keystroke.key.as_str() {
        "up" | "down" | "left" | "right" | "home" | "end" => true,
        "pageup" | "pagedown" => {
            !keystroke.modifiers.shift
                && !keystroke.modifiers.alt
                && !keystroke.modifiers.control
                && !keystroke.modifiers.platform
        }
        _ => false,
    }
}
