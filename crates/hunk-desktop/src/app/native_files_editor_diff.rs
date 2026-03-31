use hunk_editor::{EditorCommand, SpacerDescriptor};

use super::FilesEditor;

impl FilesEditor {
    pub(crate) fn set_manual_spacers(&mut self, spacers: Vec<SpacerDescriptor>) {
        let first_visible_source_line = self.first_visible_source_line();
        self.editor.apply(EditorCommand::SetSpacers(spacers));
        if let Some(first_visible_source_line) = first_visible_source_line {
            self.set_first_visible_source_line(first_visible_source_line);
        }
    }
}
