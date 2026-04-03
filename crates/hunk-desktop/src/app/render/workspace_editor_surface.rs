impl DiffViewer {
    fn workspace_editor_font_size(&self, cx: &mut Context<Self>) -> Pixels {
        cx.theme().mono_font_size
    }

    fn workspace_editor_text_style(
        &self,
        foreground: Hsla,
        editor_font_size: Pixels,
        cx: &mut Context<Self>,
    ) -> gpui::TextStyle {
        gpui::TextStyle {
            color: foreground,
            font_family: cx.theme().mono_font_family.clone(),
            font_size: editor_font_size.into(),
            line_height: gpui::relative(1.45),
            ..Default::default()
        }
    }

    fn workspace_editor_palette(
        &self,
        is_dark: bool,
        cx: &mut Context<Self>,
    ) -> crate::app::native_files_editor::FilesEditorPalette {
        let editor_chrome = hunk_editor_chrome_colors(cx.theme(), is_dark);
        crate::app::native_files_editor::FilesEditorPalette {
            background: editor_chrome.background,
            active_line_background: editor_chrome.active_line,
            line_number: editor_chrome.line_number,
            current_line_number: editor_chrome.active_line_number,
            border: hunk_opacity(cx.theme().border, is_dark, 0.92, 0.78),
            default_foreground: editor_chrome.foreground,
            muted_foreground: editor_chrome.line_number,
            selection_background: editor_chrome.selection,
            cursor: cx.theme().primary,
            invisible: editor_chrome.invisible,
            indent_guide: editor_chrome.indent_guide,
            fold_marker: editor_chrome.line_number,
            current_scope: editor_chrome.current_scope,
            bracket_match: editor_chrome.bracket_match,
            diagnostic_error: cx.theme().danger,
            diagnostic_warning: cx.theme().warning,
            diagnostic_info: cx.theme().accent,
            diff_addition: cx.theme().success,
            diff_deletion: cx.theme().danger,
            diff_modification: cx.theme().warning,
        }
    }

    fn workspace_editor_element(
        &self,
        editor: crate::app::native_files_editor::SharedFilesEditor,
        on_secondary_mouse_down: impl Fn(
            crate::app::native_files_editor::FilesEditorSecondaryClickTarget,
            Point<Pixels>,
            &mut Window,
            &mut App,
        ) + 'static,
        is_focused: bool,
        editor_font_size: Pixels,
        is_dark: bool,
        cx: &mut Context<Self>,
    ) -> crate::app::native_files_editor::FilesEditorElement {
        let editor_chrome = hunk_editor_chrome_colors(cx.theme(), is_dark);
        crate::app::native_files_editor::FilesEditorElement::new(
            editor,
            on_secondary_mouse_down,
            is_focused,
            self.workspace_editor_text_style(editor_chrome.foreground, editor_font_size, cx),
            self.workspace_editor_palette(is_dark, cx),
        )
    }
}
