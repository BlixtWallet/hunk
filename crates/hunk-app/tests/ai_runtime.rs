use std::path::PathBuf;

use hunk_app::ai::{AiWorkerStartConfig, load_ai_workspace_thread_catalog};
use tempfile::tempdir;

#[test]
fn worker_start_config_uses_frontend_neutral_defaults() {
    let cwd = PathBuf::from("/repo/worktrees/task-a");
    let codex_executable = PathBuf::from("/bin/codex");
    let codex_home = PathBuf::from("/tmp/codex-home");

    let config =
        AiWorkerStartConfig::new(cwd.clone(), codex_executable.clone(), codex_home.clone());

    assert_eq!(config.cwd, cwd);
    assert_eq!(config.workspace_key, "/repo/worktrees/task-a");
    assert_eq!(config.codex_executable, codex_executable);
    assert_eq!(config.codex_home, codex_home);
    assert!(!config.mad_max_mode);
    assert!(config.include_hidden_models);
    assert!(!config.browser_tools_enabled);
    assert_eq!(
        config.starting_status_message(),
        "Starting embedded Codex App Server..."
    );
}

#[test]
fn missing_workspace_root_is_skipped_before_starting_codex() {
    let temp = tempdir().expect("temporary directory should be created");
    let missing_workspace = temp.path().join("missing-workspace");
    let missing_codex = temp.path().join("missing-codex");
    let codex_home = temp.path().join("codex-home");

    let catalog =
        load_ai_workspace_thread_catalog(missing_workspace, missing_codex, codex_home.clone())
            .expect("missing workspaces should be skipped");

    assert!(catalog.is_none());
    assert!(!codex_home.exists());
}
