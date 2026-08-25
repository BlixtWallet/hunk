use std::path::PathBuf;

use hunk_git::workspace::{GitWorkspaceCommand, execute_git_workspace_command};
use qtbridge::{QObjectHolder, invoke_method, qtbridge_type_lib::QString};

use crate::Backend;

impl Backend {
    pub(super) fn run_git_command(&mut self, label: &str, command: GitWorkspaceCommand) {
        if self.git_loading || self.git_busy {
            return;
        }
        self.git_busy = true;
        self.git_error.clear();
        self.git_action_label = label.to_owned();
        self.notify_git_state_changed();

        let root = PathBuf::from(self.git_root.clone());
        let invoker = self.get_qml_method_invoker();
        let spawn_result = std::thread::Builder::new()
            .name("hunk-qt-git-command".to_owned())
            .spawn(move || {
                let result = execute_git_workspace_command(root.as_path(), command);
                let (success, message) = match result {
                    Ok(outcome) => (true, outcome.message),
                    Err(error) => (false, format!("{error:#}")),
                };
                invoke_method!(
                    invoker,
                    "complete_git_command",
                    success,
                    QString::from(message)
                );
            });

        if let Err(error) = spawn_result {
            self.git_busy = false;
            self.git_action_label.clear();
            self.git_error = format!("Failed to start Git command: {error}");
            self.notify_git_state_changed();
        }
    }
}
