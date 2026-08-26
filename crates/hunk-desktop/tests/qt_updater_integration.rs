const DESKTOP_LIB: &str = include_str!("../src/lib.rs");
const UPDATE_BRIDGE: &str = include_str!("../src/updater.rs");
const UPDATE_HELPER: &str = include_str!("../src/updater_helper.rs");
const MAIN_QML: &str = include_str!("../src/qml/Hunk/Main.qml");
const SHELL_QML: &str = include_str!("../src/qml/Hunk/Shell.qml");
const RELEASE_WORKFLOWS: [&str; 2] = [
    include_str!("../../../.github/workflows/release.yml"),
    include_str!("../../../.github/workflows/release-dispatch.yml"),
];

#[test]
fn updater_helper_mode_runs_before_qt_initialization() {
    let helper = DESKTOP_LIB
        .find("maybe_handle_updater_helper_mode")
        .expect("desktop startup must dispatch updater helper mode");
    let qt_app = DESKTOP_LIB
        .find("QApp::new")
        .expect("desktop startup must initialize Qt");
    assert!(helper < qt_app);
    assert!(DESKTOP_LIB.contains(".register::<UpdateBridge>()"));
}

#[test]
fn update_network_and_download_work_stay_off_the_qt_thread() {
    assert!(UPDATE_BRIDGE.contains("Builder::new()"));
    assert!(UPDATE_BRIDGE.contains(".name(\"hunk-updater-check\""));
    assert!(UPDATE_BRIDGE.contains("hunk_updater::check_for_updates"));
    assert!(UPDATE_BRIDGE.contains("hunk_updater::stage_available_update"));
    assert!(UPDATE_BRIDGE.contains("invoke_method!(invoker, \"complete_update_check\""));
    assert!(!UPDATE_BRIDGE.contains("std::process::exit"));
}

#[test]
fn qt_shell_exposes_check_download_and_restart_lifecycle() {
    assert!(MAIN_QML.contains("backend.updates.bootstrap()"));
    assert!(MAIN_QML.contains("function onQuitRequested() { Qt.quit() }"));
    assert!(SHELL_QML.contains("UpdateControl"));
    assert!(SHELL_QML.contains("root.backend.updates.poll()"));
    assert!(SHELL_QML.contains("root.backend.updates.restart_to_update()"));
}

#[test]
fn qt_window_shuts_services_down_before_backend_destruction() {
    assert!(MAIN_QML.contains("onClosing:"));
    assert!(MAIN_QML.contains("backend.browser.shutdown()"));
    assert!(MAIN_QML.contains("backend.updates.shutdown()"));
    assert!(!MAIN_QML.contains("Component.onDestruction"));
}

#[test]
fn staged_updates_use_post_exit_helpers_on_every_platform() {
    assert!(UPDATE_HELPER.contains("--apply-staged-update"));
    assert!(UPDATE_HELPER.contains("wait_for_process_to_exit"));
    assert!(UPDATE_HELPER.contains("UpdateInstallTarget::MacOsApp"));
    assert!(UPDATE_HELPER.contains("UpdateInstallTarget::LinuxBundle"));
    assert!(UPDATE_HELPER.contains("UpdateInstallTarget::WindowsMsi"));
    assert!(UPDATE_HELPER.contains("msiexec.exe"));
}

#[test]
fn release_manifests_keep_the_packaged_qt_asset_contract() {
    for workflow in RELEASE_WORKFLOWS {
        assert!(workflow.contains("Hunk-${HUNK_RELEASE_VERSION}-macos-arm64.app.tar.gz"));
        assert!(workflow.contains("Hunk-${HUNK_RELEASE_VERSION}-linux-x86_64.tar.gz"));
        assert!(workflow.contains("--asset \"macos-aarch64:app:$macos_ota_path\""));
        assert!(workflow.contains("--asset \"windows-x86_64:wix:$windows_msi_path\""));
        assert!(workflow.contains("--asset \"linux-x86_64:tarball:$linux_tarball_path\""));
    }
}
