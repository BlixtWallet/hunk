impl DiffViewer {
    fn render_ai_workspace_screen(&mut self, cx: &mut Context<Self>) -> AnyElement {
        if self.repo_discovery_failed {
            return self.render_open_project_empty_state(cx);
        }

        if let Some(error_message) = &self.error_message {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .p_4()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .child(error_message.clone()),
                )
                .into_any_element();
        }

        let is_dark = cx.theme().mode.is_dark();
        let active_bookmark = self
            .checked_out_bookmark_name()
            .map_or_else(|| "detached".to_string(), ToOwned::to_owned);

        v_flex()
            .size_full()
            .min_h_0()
            .key_context("AiWorkspace")
            .child(
                h_flex()
                    .w_full()
                    .h_10()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().muted.opacity(if is_dark { 0.32 } else { 0.62 }))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .child("Codex Agent Workspace"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Model routing + tool calls are powered by Codex App Server."),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("Active bookmark: {active_bookmark}")),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().success)
                                    .child("Server: bundled"),
                            ),
                    ),
            )
            .child(
                h_resizable("hunk-ai-workspace")
                    .child(
                        resizable_panel()
                            .size(px(280.0))
                            .size_range(px(220.0)..px(420.0))
                            .child(
                                v_flex()
                                    .size_full()
                                    .min_h_0()
                                    .border_r_1()
                                    .border_color(cx.theme().border)
                                    .bg(cx.theme().sidebar.opacity(if is_dark { 0.95 } else { 0.98 }))
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .h_9()
                                            .items_center()
                                            .justify_between()
                                            .px_2()
                                            .border_b_1()
                                            .border_color(cx.theme().border)
                                            .child(div().text_sm().font_semibold().child("Threads"))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("cwd only"),
                                            ),
                                    )
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_h_0()
                                            .gap_1()
                                            .p_2()
                                            .child(
                                                div()
                                                    .rounded_md()
                                                    .border_1()
                                                    .border_color(cx.theme().border)
                                                    .bg(cx.theme().accent.opacity(if is_dark {
                                                        0.16
                                                    } else {
                                                        0.08
                                                    }))
                                                    .p_2()
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .font_medium()
                                                            .child("New AI workspace session"),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .child("Start a thread to begin coding with Codex."),
                                                    ),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        resizable_panel().child(
                            v_flex()
                                .size_full()
                                .min_h_0()
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .min_h_0()
                                        .p_3()
                                        .gap_2()
                                        .bg(cx.theme().background)
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_semibold()
                                                .child("Timeline"),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(
                                                    "Streaming turns and tool output will appear here in later phases.",
                                                ),
                                        ),
                                )
                                .child(
                                    v_flex()
                                        .w_full()
                                        .h_32()
                                        .p_3()
                                        .gap_2()
                                        .border_t_1()
                                        .border_color(cx.theme().border)
                                        .bg(cx.theme().muted.opacity(if is_dark { 0.2 } else { 0.45 }))
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_semibold()
                                                .child("Composer"),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(
                                                    "Prompt input, accept/decline controls, and Mad Max mode toggles land in upcoming phases.",
                                                ),
                                        ),
                                ),
                        ),
                    ),
            )
            .into_any_element()
    }
}
