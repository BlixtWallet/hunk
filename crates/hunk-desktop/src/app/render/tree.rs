impl DiffViewer {
    fn render_tree(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_dark = cx.theme().mode.is_dark();
        let tree_summary = format!("{} changed files", self.repo_tree.file_count);

        v_flex()
            .size_full()
            .relative()
            .key_context("RepoTree ReviewWorkspace TreeWorkspace")
            .track_focus(&self.repo_tree_focus_handle)
            .child(
                h_flex()
                    .w_full()
                    .h(px(crate::app::FILES_WORKSPACE_RAIL_HEIGHT))
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(hunk_blend(cx.theme().sidebar, cx.theme().muted, is_dark, 0.18, 0.30))
                    .child(
                        div()
                            .text_xs()
                            .font_medium()
                            .text_color(cx.theme().muted_foreground)
                            .child(tree_summary),
                    ),
            )
            .child(div().flex_1().min_h_0().child(self.render_repo_tree_content(cx)))
    }

    fn render_repo_tree_content(&mut self, cx: &mut Context<Self>) -> AnyElement {
        if self.repo_tree.rows.is_empty() {
            return v_flex()
                .w_full()
                .px_2()
                .py_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("No changed files."),
                )
                .into_any_element();
        }

        self.sync_sidebar_repo_list_state(self.repo_tree.rows.len());
        let list_state = self.repo_tree.list_state.clone();
        let list = list(list_state, {
            cx.processor(move |this, ix: usize, _window, cx| {
                this.repo_tree
                    .rows
                    .get(ix)
                    .map(|row| this.render_repo_tree_row(row, cx))
                    .unwrap_or_else(|| div().into_any_element())
            })
        })
        .size_full()
        .map(|mut list| {
            list.style().restrict_scroll_to_axis = Some(true);
            list
        })
        .with_sizing_behavior(ListSizingBehavior::Auto);

        div()
            .size_full()
            .overflow_y_scrollbar()
            .px_1()
            .py_1()
            .child(list)
            .into_any_element()
    }

    fn sync_sidebar_repo_list_state(&mut self, row_count: usize) {
        if self.repo_tree.row_count == row_count && self.repo_tree.scroll_anchor_path.is_none() {
            return;
        }
        self.repo_tree.row_count = row_count;
        let anchor_path = self.repo_tree.scroll_anchor_path.take();
        Self::sync_sidebar_list_state(
            &self.repo_tree.list_state,
            &self.repo_tree.rows,
            anchor_path.as_deref(),
        );
    }

    fn sync_sidebar_list_state(
        list_state: &ListState,
        rows: &[super::data::RepoTreeRow],
        anchor_path: Option<&str>,
    ) {
        let row_count = rows.len();
        let previous_top = list_state.logical_scroll_top();
        list_state.reset(row_count);
        let fallback_item_ix = if row_count == 0 {
            0
        } else {
            previous_top.item_ix.min(row_count.saturating_sub(1))
        };
        let item_ix = if let Some(path) = anchor_path {
            rows.iter()
                .position(|row| row.path == path)
                .unwrap_or(fallback_item_ix)
        } else {
            fallback_item_ix
        };
        let offset_in_item = if row_count == 0 || item_ix != previous_top.item_ix {
            px(0.)
        } else {
            previous_top.offset_in_item
        };
        list_state.scroll_to(ListOffset {
            item_ix,
            offset_in_item,
        });
    }

    pub(crate) fn capture_sidebar_repo_scroll_anchor(&mut self) {
        let top_row_ix = self.repo_tree.list_state.logical_scroll_top().item_ix;
        self.repo_tree.scroll_anchor_path = self
            .repo_tree
            .rows
            .get(top_row_ix)
            .map(|row| row.path.clone());
    }

    fn render_repo_tree_row(
        &self,
        row: &super::data::RepoTreeRow,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();
        let is_selected =
            self.current_tree_selected_path().as_deref() == Some(row.path.as_str());
        let row_bg = if is_selected {
            hunk_opacity(cx.theme().accent, is_dark, 0.30, 0.14)
        } else {
            cx.theme().background.opacity(0.0)
        };
        let row_hover_bg = if is_selected {
            cx.theme().secondary_active
        } else {
            cx.theme().secondary_hover
        };
        let icon = file_icon_for_path(row.path.as_str());
        let path = row.path.clone();

        h_flex()
            .id(("repo-tree-row", stable_row_id_for_path(row.path.as_str())))
            .w_full()
            .h(px(SIDEBAR_REPO_LIST_ESTIMATED_ROW_HEIGHT))
            .items_center()
            .gap_1()
            .px_1()
            .rounded_sm()
            .bg(row_bg)
            .child(
                div().w(px(18.0)).child(
                    Icon::new(icon)
                        .size(px(14.0))
                        .text_color(cx.theme().muted_foreground),
                ),
            )
            .when_some(row.file_status, |this, status| {
                let (status_label, status_color) = change_status_label_color(status, cx);
                this.child(
                    div()
                        .px_1()
                        .py_0p5()
                        .rounded(px(4.0))
                        .text_xs()
                        .font_semibold()
                        .bg(hunk_opacity(status_color, is_dark, 0.24, 0.16))
                        .text_color(cx.theme().foreground)
                        .child(status_label),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_xs()
                    .truncate()
                    .text_color(cx.theme().foreground)
                    .child(row.name.clone()),
            )
            .hover(move |style| style.bg(row_hover_bg).cursor_pointer())
            .on_click(move |_, window, cx| {
                view.update(cx, |this, cx| {
                    this.select_repo_tree_file(path.clone(), cx);
                    this.repo_tree_focus_handle.focus(window, cx);
                });
            })
            .into_any_element()
    }
}

fn stable_row_id_for_path(path: &str) -> u64 {
    use std::hash::{Hash as _, Hasher as _};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

fn path_extension(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
}

fn file_icon_for_path(path: &str) -> IconName {
    match path_extension(path).as_deref() {
        Some("toml") | Some("yaml") | Some("yml") | Some("json") | Some("lock") => {
            IconName::Settings
        }
        Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("svg") => {
            IconName::GalleryVerticalEnd
        }
        Some("md") => IconName::BookOpen,
        _ => IconName::File,
    }
}
