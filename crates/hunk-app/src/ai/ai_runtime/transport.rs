#[derive(Debug, Clone)]
pub struct AiWorkerStartConfig {
    pub cwd: std::path::PathBuf,
    pub workspace_key: String,
    pub codex_executable: std::path::PathBuf,
    pub codex_home: std::path::PathBuf,
    pub request_timeout: std::time::Duration,
    pub mad_max_mode: bool,
    pub include_hidden_models: bool,
    pub browser_tools_enabled: bool,
}

impl AiWorkerStartConfig {
    pub fn new(
        cwd: std::path::PathBuf,
        codex_executable: std::path::PathBuf,
        codex_home: std::path::PathBuf,
    ) -> Self {
        let workspace_key = cwd.to_string_lossy().to_string();
        Self {
            cwd,
            workspace_key,
            codex_executable,
            codex_home,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            mad_max_mode: false,
            include_hidden_models: true,
            browser_tools_enabled: false,
        }
    }

    pub fn starting_status_message(&self) -> String {
        "Starting embedded Codex App Server...".to_string()
    }
}
