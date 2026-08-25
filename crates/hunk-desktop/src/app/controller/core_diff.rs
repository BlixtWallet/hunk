impl DiffViewer {
    fn request_selected_diff_reload(&mut self, cx: &mut Context<Self>) {
        if self.workspace_view_mode == WorkspaceViewMode::Diff {
            self.request_review_compare_refresh(cx);
        }
    }
}

fn should_send_ai_prompt_from_input_event(event: &InputEvent) -> bool {
    matches!(event, InputEvent::PressEnter { secondary: false })
}

#[cfg(test)]
mod ai_input_tests {
    use super::should_send_ai_prompt_from_input_event;
    use gpui_component::input::InputEvent;

    #[test]
    fn enter_sends_prompt() {
        assert!(should_send_ai_prompt_from_input_event(&InputEvent::PressEnter {
            secondary: false,
        }));
    }

    #[test]
    fn secondary_enter_does_not_send_prompt() {
        assert!(!should_send_ai_prompt_from_input_event(
            &InputEvent::PressEnter { secondary: true }
        ));
    }

    #[test]
    fn non_enter_events_do_not_send_prompt() {
        assert!(!should_send_ai_prompt_from_input_event(&InputEvent::Change));
        assert!(!should_send_ai_prompt_from_input_event(&InputEvent::Focus));
        assert!(!should_send_ai_prompt_from_input_event(&InputEvent::Blur));
    }
}
