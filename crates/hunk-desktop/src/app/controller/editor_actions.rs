impl DiffViewer {
    fn workspace_editor_copy_via(
        &self,
        window: &mut Window,
        focus_handle: &gpui::FocusHandle,
        cx: &mut Context<Self>,
        copy: impl FnOnce(&Self) -> Option<String>,
    ) -> bool {
        if !focus_handle.is_focused(window) {
            return false;
        }
        let Some(text) = copy(self) else {
            return false;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        true
    }

    fn workspace_editor_cut_via(
        &mut self,
        window: &mut Window,
        focus_handle: &gpui::FocusHandle,
        cx: &mut Context<Self>,
        cut: impl FnOnce(&mut Self) -> Option<String>,
        after_cut: impl FnOnce(&mut Self, &mut Context<Self>),
    ) -> bool {
        if !focus_handle.is_focused(window) {
            return false;
        }
        let Some(text) = cut(self) else {
            return false;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        after_cut(self, cx);
        true
    }

    fn workspace_editor_paste_via(
        &mut self,
        window: &mut Window,
        focus_handle: &gpui::FocusHandle,
        cx: &mut Context<Self>,
        paste: impl FnOnce(&mut Self, &str) -> bool,
        after_paste: impl FnOnce(&mut Self, &mut Context<Self>),
    ) -> bool {
        if !focus_handle.is_focused(window) {
            return false;
        }
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return false;
        };
        if !paste(self, text.as_str()) {
            return false;
        }
        after_paste(self, cx);
        true
    }

    fn workspace_editor_motion_via(
        &mut self,
        window: &mut Window,
        focus_handle: &gpui::FocusHandle,
        cx: &mut Context<Self>,
        apply: impl FnOnce(&mut Self) -> bool,
        after_motion: impl FnOnce(&mut Self, &mut Context<Self>),
    ) -> bool {
        if !focus_handle.is_focused(window) {
            return false;
        }
        if !apply(self) {
            return false;
        }
        after_motion(self, cx);
        true
    }
}
