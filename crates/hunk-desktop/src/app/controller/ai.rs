#[cfg(test)]
use hunk_app::ai::runtime_path::{
    bundled_codex_executable_candidates, codex_runtime_binary_name,
    codex_runtime_platform_dir, is_command_name_without_path,
    resolve_bundled_codex_executable_from_exe, resolve_workspace_codex_executable_from_exe,
};
#[cfg(all(test, target_os = "windows"))]
use hunk_app::ai::runtime_path::{
    resolve_windows_command_path, resolve_windows_command_path_from_env,
};

include!("ai/core.rs");
include!("ai/workspace_runtime.rs");
include!("ai/workspace_surface.rs");
include!("ai/workspace_surface_helpers.rs");
include!("ai/runtime.rs");
include!("ai/catalog.rs");
include!("ai/helpers.rs");
include!("ai/terminal_protocol.rs");
include!("ai/terminal_cursor.rs");
include!("ai/terminal.rs");
include!("ai/pending_steers.rs");
include!("ai/queued_messages.rs");
include!("ai/timeline_groups.rs");
include!("ai/selection.rs");
include!("ai/visible_threads_tests.rs");
include!("ai/tests.rs");
