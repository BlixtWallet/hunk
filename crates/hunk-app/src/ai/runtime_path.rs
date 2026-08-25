use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use std::ffi::{OsStr, OsString};

#[cfg(target_os = "linux")]
const LEGACY_DESKTOP_PACKAGE_NAMES: &[&str] = &["hunk-desktop", "hunk_desktop"];
#[cfg(target_os = "linux")]
const QT_DESKTOP_PACKAGE_NAMES: &[&str] = &["hunk-qt", "hunk_qt"];

#[cfg(target_os = "windows")]
const BUNDLED_CODEX_ENTRYPOINT_FILE_NAMES: &[&str] = &["codex.exe", "codex.cmd"];
#[cfg(not(target_os = "windows"))]
const BUNDLED_CODEX_ENTRYPOINT_FILE_NAMES: &[&str] = &["codex"];

pub fn resolve_codex_executable_path() -> PathBuf {
    std::env::var_os("HUNK_CODEX_EXECUTABLE")
        .map(PathBuf::from)
        .map(normalize_codex_executable_override)
        .or_else(|| {
            let current_exe = std::env::current_exe().ok()?;
            resolve_codex_executable_from_exe(current_exe.as_path()).or_else(|| {
                running_from_packaged_bundle()
                    .then(|| expected_bundled_codex_executable_from_exe(current_exe.as_path()))?
            })
        })
        .or({
            #[cfg(target_os = "windows")]
            {
                resolve_windows_command_path(Path::new("codex"))
            }
            #[cfg(not(target_os = "windows"))]
            {
                None
            }
        })
        .unwrap_or_else(|| PathBuf::from("codex"))
}

pub fn resolve_codex_executable_from_exe(current_exe: &Path) -> Option<PathBuf> {
    resolve_workspace_codex_executable_from_exe(current_exe)
        .or_else(|| resolve_bundled_codex_executable_from_exe(current_exe))
}

pub fn validate_codex_executable_path(path: &Path) -> Result<(), String> {
    if is_command_name_without_path(path) {
        #[cfg(target_os = "windows")]
        {
            return Err(format!(
                "Unable to find a spawnable Codex executable for '{}'. Install Codex so that 'codex.cmd' or 'codex.exe' is on PATH, or set HUNK_CODEX_EXECUTABLE to the full launcher path.",
                path.display()
            ));
        }
        #[cfg(not(target_os = "windows"))]
        {
            if running_from_packaged_bundle() {
                return Err(format!(
                    "Bundled Codex executable was not found for this packaged build; refusing to fall back to PATH for '{}'.",
                    path.display()
                ));
            }
            return Ok(());
        }
    }
    if !path.exists() {
        return Err(format!(
            "Bundled Codex executable not found at {}",
            path.display()
        ));
    }
    if !path.is_file() {
        return Err(format!(
            "Bundled Codex executable path is not a file: {}",
            path.display()
        ));
    }
    #[cfg(target_os = "windows")]
    {
        if !windows_path_is_spawnable(path) {
            return Err(format!(
                "Codex executable is not spawnable on Windows: {}. Point HUNK_CODEX_EXECUTABLE at a real '.cmd' or '.exe' launcher, not the Unix shim.",
                path.display()
            ));
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(path)
            .map_err(|error| format!("Unable to inspect Codex executable: {error}"))?;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(format!(
                "Bundled Codex executable is not marked executable: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn normalize_codex_executable_override(path: PathBuf) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        resolve_windows_command_path(path.as_path()).unwrap_or(path)
    }
    #[cfg(not(target_os = "windows"))]
    {
        path
    }
}

#[doc(hidden)]
pub fn resolve_workspace_codex_executable_from_exe(current_exe: &Path) -> Option<PathBuf> {
    cargo_target_root_candidates(current_exe)
        .into_iter()
        .flat_map(|target_root| workspace_codex_executable_candidates(target_root.as_path()))
        .find(|candidate| candidate_is_spawnable(candidate))
}

#[doc(hidden)]
pub fn resolve_bundled_codex_executable_from_exe(current_exe: &Path) -> Option<PathBuf> {
    bundled_codex_executable_candidates(current_exe)
        .into_iter()
        .find(|candidate| candidate_is_spawnable(candidate))
}

fn candidate_is_spawnable(candidate: &Path) -> bool {
    #[cfg(target_os = "windows")]
    {
        windows_path_is_spawnable(candidate)
    }
    #[cfg(not(target_os = "windows"))]
    {
        candidate.is_file()
    }
}

fn expected_bundled_codex_executable_from_exe(current_exe: &Path) -> Option<PathBuf> {
    bundled_codex_executable_candidates(current_exe)
        .into_iter()
        .next()
}

#[cfg(target_os = "windows")]
#[doc(hidden)]
pub fn resolve_windows_command_path(command_name: &Path) -> Option<PathBuf> {
    if is_command_name_without_path(command_name) {
        return resolve_windows_command_path_from_env(
            command_name,
            std::env::var_os("PATH"),
            std::env::var_os("PATHEXT"),
        );
    }

    resolve_windows_explicit_command_path(command_name, std::env::var_os("PATHEXT"))
}

#[cfg(target_os = "windows")]
#[doc(hidden)]
pub fn resolve_windows_command_path_from_env(
    command_name: &Path,
    path_var: Option<OsString>,
    pathext_var: Option<OsString>,
) -> Option<PathBuf> {
    let command_name = command_name.as_os_str();
    let path_var = path_var?;
    let candidate_names = windows_command_candidate_names(command_name, pathext_var.as_deref());
    std::env::split_paths(&path_var).find_map(|directory| {
        candidate_names
            .iter()
            .map(|candidate| directory.join(candidate))
            .find(|candidate| windows_path_is_spawnable(candidate))
    })
}

#[cfg(target_os = "windows")]
fn resolve_windows_explicit_command_path(
    command_path: &Path,
    pathext_var: Option<OsString>,
) -> Option<PathBuf> {
    if windows_path_is_spawnable(command_path) {
        return Some(command_path.to_path_buf());
    }

    let parent = command_path.parent()?;
    let file_name = command_path.file_name()?;
    let candidate_names = windows_command_candidate_names(file_name, pathext_var.as_deref());
    candidate_names
        .iter()
        .map(|candidate| parent.join(candidate))
        .find(|candidate| windows_path_is_spawnable(candidate))
}

#[cfg(target_os = "windows")]
fn windows_command_candidate_names(
    command_name: &OsStr,
    pathext_var: Option<&OsStr>,
) -> Vec<OsString> {
    let command_path = Path::new(command_name);
    if command_path.extension().is_some() {
        return vec![command_name.to_os_string()];
    }

    let mut candidates = Vec::new();
    let pathext_var = pathext_var
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| OsStr::new(".COM;.EXE;.BAT;.CMD"));
    for extension in pathext_var
        .to_string_lossy()
        .split(';')
        .map(str::trim)
        .filter(|extension| !extension.is_empty())
    {
        let normalized = if extension.starts_with('.') {
            extension.to_string()
        } else {
            format!(".{extension}")
        };
        candidates.push(OsString::from(format!(
            "{}{}",
            command_name.to_string_lossy(),
            normalized
        )));
    }
    candidates.push(command_name.to_os_string());
    candidates
}

#[cfg(target_os = "windows")]
fn windows_path_is_spawnable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());
    match extension.as_deref() {
        Some("cmd" | "bat" | "com") => true,
        Some("exe") => windows_file_has_mz_header(path),
        Some(_) => false,
        None => windows_file_has_mz_header(path),
    }
}

#[cfg(target_os = "windows")]
fn windows_file_has_mz_header(path: &Path) -> bool {
    use std::io::Read as _;

    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut header = [0_u8; 2];
    file.read_exact(&mut header).is_ok() && header == *b"MZ"
}

#[doc(hidden)]
pub fn bundled_codex_executable_candidates(current_exe: &Path) -> Vec<PathBuf> {
    let Some(exe_dir) = current_exe.parent() else {
        return Vec::new();
    };

    let platform_dir = codex_runtime_platform_dir();
    let mut candidates = Vec::new();
    let push_candidates = |base_dir: &Path, candidates: &mut Vec<PathBuf>| {
        for entrypoint in bundled_codex_entrypoint_file_names() {
            candidates.push(base_dir.join(entrypoint));
        }
    };
    push_candidates(
        exe_dir.join("codex-runtime").join(platform_dir).as_path(),
        &mut candidates,
    );

    if cfg!(target_os = "macos")
        && let Some(contents_dir) = exe_dir.parent()
    {
        push_candidates(
            contents_dir
                .join("Resources")
                .join("codex-runtime")
                .join(platform_dir)
                .as_path(),
            &mut candidates,
        );
    } else {
        push_candidates(
            exe_dir
                .join("Resources")
                .join("codex-runtime")
                .join(platform_dir)
                .as_path(),
            &mut candidates,
        );
    }

    #[cfg(target_os = "linux")]
    {
        let push_linux_packager_candidates =
            |root_dir: &Path,
             binary_names: &[std::ffi::OsString],
             candidates: &mut Vec<PathBuf>| {
                for binary_name in binary_names {
                    push_candidates(
                        root_dir
                            .join("usr")
                            .join("lib")
                            .join(binary_name)
                            .join("codex-runtime")
                            .join(platform_dir)
                            .as_path(),
                        candidates,
                    );
                }
            };
        let binary_names = packaged_linux_binary_dir_names(current_exe);

        if let Some(root_dir) = linux_packager_root_from_current_exe(current_exe) {
            push_linux_packager_candidates(root_dir.as_path(), &binary_names, &mut candidates);
        }
    }

    push_candidates(exe_dir, &mut candidates);
    candidates
}

fn cargo_target_root_candidates(current_exe: &Path) -> Vec<PathBuf> {
    let current_exe_parent = current_exe.parent();
    current_exe
        .ancestors()
        .filter_map(
            |ancestor| match ancestor.file_name().and_then(|name| name.to_str()) {
                Some("target")
                    if current_exe_parent
                        .and_then(|parent| parent.file_name())
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| matches!(name, "debug" | "release")) =>
                {
                    ancestor.parent().map(Path::to_path_buf)
                }
                _ => None,
            },
        )
        .collect()
}

fn workspace_codex_executable_candidates(workspace_root: &Path) -> Vec<PathBuf> {
    let base_dir = workspace_root
        .join("assets")
        .join("codex-runtime")
        .join(codex_runtime_platform_dir());
    bundled_codex_entrypoint_file_names()
        .iter()
        .map(|entrypoint| base_dir.join(entrypoint))
        .collect()
}

#[cfg(target_os = "linux")]
fn linux_packager_root_from_current_exe(current_exe: &Path) -> Option<PathBuf> {
    let exe_dir = current_exe.parent()?;
    if exe_dir.file_name()? != std::ffi::OsStr::new("bin") {
        return None;
    }
    let usr_dir = exe_dir.parent()?;
    if usr_dir.file_name()? != std::ffi::OsStr::new("usr") {
        return None;
    }
    usr_dir.parent().map(Path::to_path_buf)
}

#[cfg(target_os = "linux")]
fn packaged_linux_binary_dir_names(current_exe: &Path) -> Vec<std::ffi::OsString> {
    use std::ffi::OsString;

    let mut names = Vec::new();
    if let Some(file_name) = current_exe.file_name() {
        names.push(file_name.to_os_string());
    }

    for candidate in LEGACY_DESKTOP_PACKAGE_NAMES
        .iter()
        .chain(QT_DESKTOP_PACKAGE_NAMES)
        .map(|name| OsString::from(*name))
    {
        if !names.iter().any(|existing| existing == &candidate) {
            names.push(candidate);
        }
    }
    names
}

pub fn codex_runtime_platform_dir() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}

#[doc(hidden)]
pub fn codex_runtime_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "codex.exe"
    } else {
        "codex"
    }
}

fn bundled_codex_entrypoint_file_names() -> &'static [&'static str] {
    BUNDLED_CODEX_ENTRYPOINT_FILE_NAMES
}

#[doc(hidden)]
pub fn is_command_name_without_path(path: &Path) -> bool {
    if path.is_absolute() {
        return false;
    }
    let text = path.to_string_lossy();
    !text.contains(std::path::MAIN_SEPARATOR) && !text.contains('/')
}

fn running_from_packaged_bundle() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("APPDIR").is_some() || std::env::var_os("APPIMAGE").is_some()
    }

    #[cfg(target_os = "macos")]
    {
        let Ok(current_exe) = std::env::current_exe() else {
            return false;
        };
        current_exe
            .components()
            .any(|component| component.as_os_str() == std::ffi::OsStr::new("Contents"))
    }

    #[cfg(target_os = "windows")]
    {
        false
    }
}
