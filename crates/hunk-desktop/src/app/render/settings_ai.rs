impl DiffViewer {
    fn render_settings_ai_category(
        &self,
        settings: &SettingsDraft,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();
        let card_surface = hunk_card_surface(cx.theme(), is_dark);
        let dropdown_bg = hunk_dropdown_fill(cx.theme(), is_dark);
        let prevent_idle_sleep_label = if settings.ai.prevent_idle_sleep {
            "On"
        } else {
            "Off"
        };
        let desktop_notifications_enabled_label = if settings.desktop_notifications.enabled {
            "On"
        } else {
            "Off"
        };
        let desktop_notifications_focus_label = if settings.desktop_notifications.only_when_unfocused
        {
            "Only When Unfocused"
        } else {
            "Always"
        };
        let ai_notifications_agent_finished_label = if settings.desktop_notifications.ai.agent_finished
        {
            "On"
        } else {
            "Off"
        };
        let ai_notifications_plan_ready_label = if settings.desktop_notifications.ai.plan_ready {
            "On"
        } else {
            "Off"
        };
        let ai_notifications_user_input_label =
            if settings.desktop_notifications.ai.user_input_required {
                "On"
            } else {
                "Off"
            };
        let ai_notifications_approval_label =
            if settings.desktop_notifications.ai.approval_required {
                "On"
            } else {
                "Off"
            };
        let desktop_notification_status_note = self.desktop_notification_settings_status_note();

        v_flex()
            .w_full()
            .gap_3()
            .child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .child(
                        div()
                            .text_base()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("AI"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Runtime behavior and desktop notifications for Codex."),
                    ),
            )
            .child(
                v_flex()
                    .w_full()
                    .gap_3()
                    .p_3()
                    .rounded(px(10.0))
                    .border_1()
                    .border_color(card_surface.border)
                    .bg(card_surface.background)
                    .child(
                        v_flex()
                            .w_full()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("Long-Running Tasks"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Controls for AI turns that may run for a long time."),
                            ),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                v_flex()
                                    .min_w_0()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_semibold()
                                            .text_color(cx.theme().foreground)
                                            .child("Keep Awake During AI Turns"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("Prevents system idle sleep while Codex is running. The display may still sleep."),
                                    ),
                            )
                            .child({
                                let view = view.clone();
                                let prevent_idle_sleep = settings.ai.prevent_idle_sleep;
                                Button::new("settings-ai-prevent-idle-sleep-dropdown")
                                    .outline()
                                    .compact()
                                    .rounded(px(8.0))
                                    .bg(dropdown_bg)
                                    .dropdown_caret(true)
                                    .label(prevent_idle_sleep_label)
                                    .dropdown_menu(move |menu, _, _| {
                                        menu.item(
                                            PopupMenuItem::new("On")
                                                .checked(prevent_idle_sleep)
                                                .on_click({
                                                    let view = view.clone();
                                                    move |_, _, cx| {
                                                        view.update(cx, |this, cx| {
                                                            this.set_settings_ai_prevent_idle_sleep(
                                                                true, cx,
                                                            );
                                                        });
                                                    }
                                                }),
                                        )
                                        .item(
                                            PopupMenuItem::new("Off")
                                                .checked(!prevent_idle_sleep)
                                                .on_click({
                                                    let view = view.clone();
                                                    move |_, _, cx| {
                                                        view.update(cx, |this, cx| {
                                                            this.set_settings_ai_prevent_idle_sleep(
                                                                false, cx,
                                                            );
                                                        });
                                                    }
                                                }),
                                        )
                                    })
                            }),
                    ),
            )
            .child(
                v_flex()
                    .w_full()
                    .gap_3()
                    .p_3()
                    .rounded(px(10.0))
                    .border_1()
                    .border_color(card_surface.border)
                    .bg(card_surface.background)
                    .child(
                        v_flex()
                            .w_full()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("AI Notifications"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Desktop notifications for AI states where Codex is waiting on you."),
                            ),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("Desktop notifications"),
                            )
                            .child({
                                let view = view.clone();
                                let enabled = settings.desktop_notifications.enabled;
                                Button::new("settings-ai-desktop-notifications-dropdown")
                                    .outline()
                                    .compact()
                                    .rounded(px(8.0))
                                    .bg(dropdown_bg)
                                    .dropdown_caret(true)
                                    .label(desktop_notifications_enabled_label)
                                    .dropdown_menu(move |menu, _, _| {
                                        menu.item(
                                            PopupMenuItem::new("On")
                                                .checked(enabled)
                                                .on_click({
                                                    let view = view.clone();
                                                    move |_, _, cx| {
                                                        view.update(cx, |this, cx| {
                                                            this.set_settings_desktop_notifications_enabled(
                                                                true, cx,
                                                            );
                                                        });
                                                    }
                                                }),
                                        )
                                        .item(
                                            PopupMenuItem::new("Off")
                                                .checked(!enabled)
                                                .on_click({
                                                    let view = view.clone();
                                                    move |_, _, cx| {
                                                        view.update(cx, |this, cx| {
                                                            this.set_settings_desktop_notifications_enabled(
                                                                false, cx,
                                                            );
                                                        });
                                                    }
                                                }),
                                        )
                                    })
                            }),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("Delivery policy"),
                            )
                            .child({
                                let view = view.clone();
                                let only_when_unfocused =
                                    settings.desktop_notifications.only_when_unfocused;
                                Button::new("settings-ai-desktop-notifications-focus-dropdown")
                                    .outline()
                                    .compact()
                                    .rounded(px(8.0))
                                    .bg(dropdown_bg)
                                    .dropdown_caret(true)
                                    .label(desktop_notifications_focus_label)
                                    .dropdown_menu(move |menu, _, _| {
                                        menu.item(
                                            PopupMenuItem::new("Only When Unfocused")
                                                .checked(only_when_unfocused)
                                                .on_click({
                                                    let view = view.clone();
                                                    move |_, _, cx| {
                                                        view.update(cx, |this, cx| {
                                                            this.set_settings_desktop_notifications_only_when_unfocused(
                                                                true, cx,
                                                            );
                                                        });
                                                    }
                                                }),
                                        )
                                        .item(
                                            PopupMenuItem::new("Always")
                                                .checked(!only_when_unfocused)
                                                .on_click({
                                                    let view = view.clone();
                                                    move |_, _, cx| {
                                                        view.update(cx, |this, cx| {
                                                            this.set_settings_desktop_notifications_only_when_unfocused(
                                                                false, cx,
                                                            );
                                                        });
                                                    }
                                                }),
                                        )
                                    })
                            }),
                    )
                    .child(self.render_settings_ai_notification_toggle(
                        "settings-ai-notify-agent-finished-dropdown",
                        "Agent finished",
                        ai_notifications_agent_finished_label,
                        settings.desktop_notifications.ai.agent_finished,
                        |ai| ai.agent_finished = true,
                        |ai| ai.agent_finished = false,
                        cx,
                    ))
                    .child(self.render_settings_ai_notification_toggle(
                        "settings-ai-notify-plan-ready-dropdown",
                        "Plan ready",
                        ai_notifications_plan_ready_label,
                        settings.desktop_notifications.ai.plan_ready,
                        |ai| ai.plan_ready = true,
                        |ai| ai.plan_ready = false,
                        cx,
                    ))
                    .child(self.render_settings_ai_notification_toggle(
                        "settings-ai-notify-user-input-dropdown",
                        "Agent input requests",
                        ai_notifications_user_input_label,
                        settings.desktop_notifications.ai.user_input_required,
                        |ai| ai.user_input_required = true,
                        |ai| ai.user_input_required = false,
                        cx,
                    ))
                    .child(self.render_settings_ai_notification_toggle(
                        "settings-ai-notify-approval-dropdown",
                        "Approvals",
                        ai_notifications_approval_label,
                        settings.desktop_notifications.ai.approval_required,
                        |ai| ai.approval_required = true,
                        |ai| ai.approval_required = false,
                        cx,
                    ))
                    .child(
                        v_flex()
                            .w_full()
                            .gap_2()
                            .child(
                                div()
                                    .w_full()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Sends a real OS notification through the current platform backend and reports the result in an in-app toast."),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .justify_end()
                                    .child({
                                        let view = view.clone();
                                        Button::new("settings-ai-notifications-test")
                                            .outline()
                                            .rounded(px(8.0))
                                            .label("Send Test Notification")
                                            .disabled(self.desktop_notification_test_button_disabled())
                                            .on_click(move |_, _, cx| {
                                                view.update(cx, |this, cx| {
                                                    this.send_test_ai_desktop_notification(cx);
                                                });
                                            })
                                    }),
                            ),
                    )
                    .children(desktop_notification_status_note.map(|note| {
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(note)
                            .into_any_element()
                    })),
            )
            .into_any_element()
    }

    fn render_settings_ai_notification_toggle(
        &self,
        id: &'static str,
        label: &'static str,
        value_label: &'static str,
        enabled: bool,
        enable: fn(&mut AiDesktopNotificationsConfig),
        disable: fn(&mut AiDesktopNotificationsConfig),
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();
        let dropdown_bg = hunk_dropdown_fill(cx.theme(), is_dark);

        h_flex()
            .w_full()
            .items_center()
            .justify_between()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .text_color(cx.theme().foreground)
                    .child(label),
            )
            .child(
                Button::new(id)
                    .outline()
                    .compact()
                    .rounded(px(8.0))
                    .bg(dropdown_bg)
                    .dropdown_caret(true)
                    .label(value_label)
                    .dropdown_menu(move |menu, _, _| {
                        menu.item(
                            PopupMenuItem::new("On")
                                .checked(enabled)
                                .on_click({
                                    let view = view.clone();
                                    move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.set_settings_ai_desktop_notification(enable, cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            PopupMenuItem::new("Off")
                                .checked(!enabled)
                                .on_click({
                                    let view = view.clone();
                                    move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.set_settings_ai_desktop_notification(disable, cx);
                                        });
                                    }
                                }),
                        )
                    }),
            )
            .into_any_element()
    }
}
