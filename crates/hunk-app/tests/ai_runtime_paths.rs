use std::path::{Path, PathBuf};

use hunk_app::ai::runtime_path::{
    codex_runtime_platform_dir, is_command_name_without_path,
    resolve_bundled_codex_executable_from_exe, resolve_codex_executable_from_exe,
    resolve_workspace_codex_executable_from_exe, validate_codex_executable_path,
};
use tempfile::TempDir;

#[cfg(target_os = "windows")]
use hunk_app::ai::runtime_path::{
    resolve_windows_command_path, resolve_windows_command_path_from_env,
};
#[cfg(target_os = "windows")]
use std::ffi::OsString;

fn runtime_entrypoint_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "codex.cmd"
    } else {
        "codex"
    }
}

fn write_fake_codex_launcher(path: &Path) {
    std::fs::create_dir_all(path.parent().expect("launcher parent should exist"))
        .expect("launcher directory should be created");

    #[cfg(target_os = "windows")]
    std::fs::write(path, "@echo off\r\nexit /b 0\r\n").expect("fake launcher should be written");

    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;

        std::fs::write(path, "#!/bin/sh\nexit 0\n").expect("fake launcher should be written");
        let mut permissions = std::fs::metadata(path)
            .expect("launcher metadata should be readable")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("launcher should be executable");
    }
}

fn bundled_runtime_path(exe_dir: &Path) -> PathBuf {
    exe_dir
        .join("codex-runtime")
        .join(codex_runtime_platform_dir())
        .join(runtime_entrypoint_name())
}

#[test]
fn command_name_detection_distinguishes_path_fallbacks() {
    assert!(is_command_name_without_path(Path::new("codex")));
    assert!(!is_command_name_without_path(Path::new("./codex")));
    assert!(!is_command_name_without_path(Path::new("/usr/bin/codex")));
}

#[test]
fn resolver_finds_an_adjacent_bundled_runtime() {
    let root = TempDir::new().expect("temp dir should be created");
    let exe_dir = root.path().join("bin");
    std::fs::create_dir_all(&exe_dir).expect("exe dir should be created");
    let exe_path = exe_dir.join("hunk");
    std::fs::write(&exe_path, "").expect("fake executable should be written");
    let runtime_path = bundled_runtime_path(exe_dir.as_path());
    write_fake_codex_launcher(runtime_path.as_path());

    assert_eq!(
        resolve_bundled_codex_executable_from_exe(exe_path.as_path()),
        Some(runtime_path.clone())
    );
    assert_eq!(
        resolve_codex_executable_from_exe(exe_path.as_path()),
        Some(runtime_path)
    );
}

#[test]
fn resolver_prefers_the_workspace_runtime_for_cargo_targets() {
    let root = TempDir::new().expect("temp dir should be created");
    let exe_path = root.path().join("target").join("debug").join("hunk_qt");
    std::fs::create_dir_all(exe_path.parent().expect("exe parent should exist"))
        .expect("exe dir should be created");
    std::fs::write(&exe_path, "").expect("fake executable should be written");
    let runtime_path = root
        .path()
        .join("assets")
        .join("codex-runtime")
        .join(codex_runtime_platform_dir())
        .join(runtime_entrypoint_name());
    write_fake_codex_launcher(runtime_path.as_path());

    assert_eq!(
        resolve_workspace_codex_executable_from_exe(exe_path.as_path()),
        Some(runtime_path.clone())
    );
    assert_eq!(
        resolve_codex_executable_from_exe(exe_path.as_path()),
        Some(runtime_path)
    );
}

#[test]
fn workspace_lookup_does_not_escape_a_packaged_target_layout() {
    let root = TempDir::new().expect("temp dir should be created");
    let exe_path = root
        .path()
        .join("target")
        .join("packager")
        .join("Hunk.app")
        .join("Contents")
        .join("MacOS")
        .join("Hunk");
    std::fs::create_dir_all(exe_path.parent().expect("exe parent should exist"))
        .expect("exe dir should be created");
    std::fs::write(&exe_path, "").expect("fake executable should be written");
    let workspace_runtime = root
        .path()
        .join("assets")
        .join("codex-runtime")
        .join(codex_runtime_platform_dir())
        .join(runtime_entrypoint_name());
    write_fake_codex_launcher(workspace_runtime.as_path());

    assert_eq!(
        resolve_workspace_codex_executable_from_exe(exe_path.as_path()),
        None
    );
}

#[cfg(target_os = "macos")]
#[test]
fn resolver_finds_the_macos_bundle_resources_runtime() {
    let root = TempDir::new().expect("temp dir should be created");
    let contents = root.path().join("Hunk.app").join("Contents");
    let exe_path = contents.join("MacOS").join("Hunk");
    std::fs::create_dir_all(exe_path.parent().expect("exe parent should exist"))
        .expect("exe dir should be created");
    std::fs::write(&exe_path, "").expect("fake executable should be written");
    let runtime_path = contents
        .join("Resources")
        .join("codex-runtime")
        .join(codex_runtime_platform_dir())
        .join(runtime_entrypoint_name());
    write_fake_codex_launcher(runtime_path.as_path());

    assert_eq!(
        resolve_bundled_codex_executable_from_exe(exe_path.as_path()),
        Some(runtime_path)
    );
}

#[cfg(not(target_os = "macos"))]
#[test]
fn resolver_finds_the_resources_runtime_next_to_the_binary() {
    let root = TempDir::new().expect("temp dir should be created");
    let exe_path = root.path().join("hunk");
    std::fs::write(&exe_path, "").expect("fake executable should be written");
    let runtime_path = root
        .path()
        .join("Resources")
        .join("codex-runtime")
        .join(codex_runtime_platform_dir())
        .join(runtime_entrypoint_name());
    write_fake_codex_launcher(runtime_path.as_path());

    assert_eq!(
        resolve_bundled_codex_executable_from_exe(exe_path.as_path()),
        Some(runtime_path)
    );
}

#[cfg(target_os = "linux")]
#[test]
fn resolver_includes_the_qt_linux_packager_directory() {
    let root = TempDir::new().expect("temp dir should be created");
    let exe_path = root.path().join("usr").join("bin").join("hunk_qt_bin");
    std::fs::create_dir_all(exe_path.parent().expect("exe parent should exist"))
        .expect("exe dir should be created");
    std::fs::write(&exe_path, "").expect("fake executable should be written");
    let runtime_path = root
        .path()
        .join("usr")
        .join("lib")
        .join("hunk_qt")
        .join("codex-runtime")
        .join(codex_runtime_platform_dir())
        .join(runtime_entrypoint_name());
    write_fake_codex_launcher(runtime_path.as_path());

    assert_eq!(
        resolve_bundled_codex_executable_from_exe(exe_path.as_path()),
        Some(runtime_path)
    );
}

#[test]
fn validation_rejects_missing_explicit_paths() {
    let root = TempDir::new().expect("temp dir should be created");
    let missing = root.path().join("missing-codex");

    assert!(
        validate_codex_executable_path(missing.as_path())
            .expect_err("missing path should fail")
            .contains("not found")
    );
}

#[cfg(unix)]
#[test]
fn validation_requires_an_executable_unix_file() {
    use std::os::unix::fs::PermissionsExt;

    let root = TempDir::new().expect("temp dir should be created");
    let launcher = root.path().join("codex-runtime");
    std::fs::write(&launcher, "#!/bin/sh\nexit 0\n").expect("launcher should be written");

    assert!(
        validate_codex_executable_path(launcher.as_path())
            .expect_err("non-executable file should fail")
            .contains("not marked executable")
    );

    let mut permissions = std::fs::metadata(&launcher)
        .expect("launcher metadata should be readable")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&launcher, permissions).expect("launcher should be executable");
    validate_codex_executable_path(launcher.as_path()).expect("executable file should validate");
}

#[cfg(target_os = "windows")]
#[test]
fn windows_command_resolution_prefers_a_spawnable_launcher() {
    let root = TempDir::new().expect("temp dir should be created");
    let bin_dir = root.path().join("bin");
    std::fs::create_dir_all(&bin_dir).expect("bin dir should be created");
    std::fs::write(bin_dir.join("codex"), "#!/bin/sh\n").expect("unix shim should be written");
    let launcher_path = bin_dir.join("codex.cmd");
    write_fake_codex_launcher(launcher_path.as_path());

    let resolved = resolve_windows_command_path_from_env(
        Path::new("codex"),
        Some(std::env::join_paths([bin_dir.as_path()]).expect("path should join")),
        Some(OsString::from(".COM;.EXE;.BAT;.CMD")),
    );
    assert_eq!(resolved, Some(launcher_path.clone()));
    assert_eq!(
        resolve_windows_command_path(bin_dir.join("codex").as_path()),
        Some(launcher_path)
    );
}
