use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub fn resolve_codex_home_path() -> Option<PathBuf> {
    resolve_codex_home_path_from(
        env::var_os("CODEX_HOME").map(PathBuf::from),
        user_home_dir(),
    )
}

pub fn default_codex_home_path() -> Option<PathBuf> {
    user_home_dir().map(|home_dir| home_dir.join(".codex"))
}

pub fn resolve_ai_chats_root_path() -> Option<PathBuf> {
    hunk_domain::paths::hunk_home_dir()
        .ok()
        .map(|home_dir| home_dir.join("chats"))
}

pub fn ensure_ai_chats_root_path() -> Option<PathBuf> {
    let chats_root = resolve_ai_chats_root_path()?;
    fs::create_dir_all(&chats_root).ok()?;
    Some(chats_root)
}

pub fn is_ai_chats_workspace_path(path: &Path) -> bool {
    resolve_ai_chats_root_path().is_some_and(|chats_root| path == chats_root)
}

pub fn ai_chats_workspace_paths() -> Vec<PathBuf> {
    let Some(chats_root) = ensure_ai_chats_root_path().or_else(resolve_ai_chats_root_path) else {
        return Vec::new();
    };

    vec![chats_root]
}

fn user_home_dir() -> Option<PathBuf> {
    dirs::home_dir()
        .or_else(|| env::var_os("HOME").map(PathBuf::from))
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
}

fn resolve_codex_home_path_from(
    configured_path: Option<PathBuf>,
    home_dir: Option<PathBuf>,
) -> Option<PathBuf> {
    match configured_path {
        Some(path) => expand_home_prefixed_path(path, home_dir.as_deref()),
        None => home_dir.map(|home_dir| home_dir.join(".codex")),
    }
}

fn expand_home_prefixed_path(path: PathBuf, home_dir: Option<&Path>) -> Option<PathBuf> {
    let Some(relative_suffix) = home_relative_suffix(path.as_path()) else {
        return Some(path);
    };

    let mut resolved = home_dir?.to_path_buf();
    if !relative_suffix.as_os_str().is_empty() {
        resolved.push(relative_suffix);
    }
    Some(resolved)
}

fn home_relative_suffix(path: &Path) -> Option<PathBuf> {
    let mut components = path.components();
    match components.next()? {
        Component::Normal(component) if component == OsStr::new("~") => {
            let mut suffix = PathBuf::new();
            for component in components {
                suffix.push(component.as_os_str());
            }
            Some(suffix)
        }
        _ => None,
    }
}
