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
            2,
            "{name} must use the hosted Qt action only for macOS and Windows"
        );
        assert!(
            workflow.contains("nix develop --accept-flake-config -c ./scripts/qt/install_qt.sh"),
            "{name} must provision Linux Qt through Nix"
        );
        assert!(
            workflow.contains("HUNK_QT_INSTALL_WAYLAND: \"1\""),
            "{name} must install native Wayland support through the pinned installer"
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
    assert!(workflow.contains("nix develop --accept-flake-config -c ./scripts/qt/install_qt.sh"));
    assert_eq!(
        workflow
            .matches("uses: jurplel/install-qt-action@v4")
            .count(),
        1,
        "only the Windows PR job should use install-qt-action"
    );
    assert!(workflow.contains("$HUNK_QT_ROOT/lib:$HUNK_LINUX_PACKAGING_LIBRARY_PATH"));
    assert!(workflow.contains("cargo build -p hunk-desktop --locked --profile ci"));
    assert!(!workflow.contains("cargo build -p hunk-qt"));
    assert!(
        !workflow
            .to_ascii_lowercase()
            .contains("cargo build -p gpui")
    );
}

#[test]
fn self_hosted_qt_installer_is_nix_owned_and_wayland_aware() {
    let installer = include_str!("../../../scripts/qt/install_qt.sh");
    assert!(installer.contains("qt_version=\"6.11.2\""));
    assert!(installer.contains("aqt_version=\"3.3.0\""));
    assert!(installer.contains("python3 -m virtualenv"));
    assert!(installer.contains("--modules qtwaylandcompositor"));
    assert!(installer.contains("libqwayland-egl.so"));
    assert!(installer.contains("libqwayland-generic.so"));

    let flake = include_str!("../../../flake.nix");
    assert!(flake.contains("pythonPackages.pip"));
    assert!(flake.contains("pythonPackages.virtualenv"));
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
    assert!(linux.contains("export HUNK_UPDATE_EXPLANATION="));
    assert!(linux.contains("validate_linux_runtime_tree"));
}
