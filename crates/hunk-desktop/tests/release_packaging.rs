const RELEASE_WORKFLOWS: [(&str, &str); 3] = [
    (
        "release.yml",
        include_str!("../../../.github/workflows/release.yml"),
    ),
    (
        "release-dispatch.yml",
        include_str!("../../../.github/workflows/release-dispatch.yml"),
    ),
    (
        "adhoc-release-assets.yml",
        include_str!("../../../.github/workflows/adhoc-release-assets.yml"),
    ),
];

#[test]
fn every_release_entry_point_installs_the_exact_qt_sdk() {
    for (name, workflow) in RELEASE_WORKFLOWS {
        assert!(
            workflow.contains("HUNK_QT_VERSION: \"6.11.2\""),
            "{name} must pin the supported Qt version"
        );
        assert_eq!(
            workflow
                .matches("uses: jurplel/install-qt-action@v4")
                .count(),
            3,
            "{name} must provision Qt for macOS, Linux, and Windows"
        );
        assert!(
            workflow.contains("modules: qtwaylandcompositor"),
            "{name} must install native Wayland support"
        );
        assert!(
            workflow.contains("aqtinstall.git@8c3695d4a4e1ceabf6a74dc6c79681656dc6b74b"),
            "{name} must use the pinned Qt 6.11 Windows repository fix"
        );
        assert!(!workflow.contains("hunk-qt"));
        assert!(!workflow.to_ascii_lowercase().contains("gpui"));
    }
}

#[test]
fn linux_pr_checks_use_the_self_hosted_qt_runner() {
    let workflow = include_str!("../../../.github/workflows/pr-build.yml");
    assert!(workflow.contains("runs-on: ubuntu-self-hosted"));
    assert!(!workflow.contains("runs-on: ubuntu-24.04"));
    assert!(workflow.contains("version: ${{ env.HUNK_QT_VERSION }}"));
    assert!(workflow.contains("cargo build -p hunk-desktop --locked --profile ci"));
    assert!(!workflow.contains("cargo build -p hunk-qt"));
    assert!(
        !workflow
            .to_ascii_lowercase()
            .contains("cargo build -p gpui")
    );
}

#[test]
fn platform_packagers_require_deployed_qt_runtimes() {
    let macos = include_str!("../../../scripts/package_macos_release.sh");
    assert!(macos.contains("/bin/macdeployqt"));
    assert!(macos.contains("-qmldir=$QML_SOURCE_DIR"));
    assert!(macos.contains("QtCore.framework/QtCore"));
    assert!(macos.contains("PlugIns/platforms/libqcocoa.dylib"));
    assert!(macos.contains("Resources/qml/QtQuick/Controls/qmldir"));

    let windows = include_str!("../../../scripts/package_windows_release.ps1");
    assert!(windows.contains("bin/windeployqt.exe"));
    assert!(windows.contains("--qmldir $QmlSourceDir"));
    assert!(windows.contains("Qt6Core.dll"));
    assert!(windows.contains("platforms/qwindows.dll"));
    assert!(windows.contains("qml/QtQuick/Controls/qmldir"));

    let linux = include_str!("../../../scripts/linux_release_common.sh");
    assert!(linux.contains("Plugins = lib/qt6/plugins"));
    assert!(linux.contains("QmlImports = lib/qt6/qml"));
    assert!(linux.contains("libQt6Core.so.6"));
    assert!(linux.contains("plugins/platforms/libqxcb.so"));
    assert!(linux.contains("plugins/platforms/libqwayland-generic.so"));
    assert!(linux.contains("validate_linux_runtime_tree"));
}
