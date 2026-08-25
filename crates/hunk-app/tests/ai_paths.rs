use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};

use hunk_app::ai::{
    ai_chats_workspace_paths, is_ai_chats_workspace_path, resolve_ai_chats_root_path,
    resolve_codex_home_path,
};
use tempfile::tempdir;

struct EnvVarGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &OsStr) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

fn lock_environment() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(error) => error.into_inner(),
    }
}

#[test]
fn configured_codex_home_path_is_resolved() {
    let _lock = lock_environment();
    let temp = tempdir().expect("temporary directory should be created");
    let _codex_home = EnvVarGuard::set("CODEX_HOME", temp.path().as_os_str());

    assert_eq!(resolve_codex_home_path().as_deref(), Some(temp.path()));
}

#[test]
fn configured_tilde_codex_home_path_expands_from_user_home() {
    let _lock = lock_environment();
    let _codex_home = EnvVarGuard::set("CODEX_HOME", OsStr::new("~/.codex-hunk-test"));
    let expected = dirs::home_dir().map(|home| home.join(".codex-hunk-test"));

    assert_eq!(resolve_codex_home_path(), expected);
}

#[test]
fn chat_workspace_paths_use_only_the_configured_chats_root() {
    let _lock = lock_environment();
    let temp = tempdir().expect("temporary directory should be created");
    let _hunk_home = EnvVarGuard::set(
        hunk_domain::paths::HUNK_HOME_DIR_ENV_VAR,
        temp.path().as_os_str(),
    );
    let chats_root = temp.path().join("chats");

    assert_eq!(resolve_ai_chats_root_path(), Some(chats_root.clone()));
    assert_eq!(ai_chats_workspace_paths(), vec![chats_root.clone()]);
    assert!(is_ai_chats_workspace_path(&chats_root));
    assert!(!is_ai_chats_workspace_path(&chats_root.join("thread-1")));
    assert!(!is_ai_chats_workspace_path(Path::new("/repo")));
}
