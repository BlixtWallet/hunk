pub(crate) use hunk_app::ai::{
    ai_chats_workspace_paths, is_ai_chats_workspace_path, resolve_ai_chats_root_path,
    resolve_codex_home_path,
};

#[cfg(test)]
pub(crate) fn lock_hunk_home_test_env() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};

    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(error) => error.into_inner(),
    }
}
