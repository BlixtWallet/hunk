use std::path::PathBuf;
use std::process::Command;

use qtbridge_build_utils::qt_build::QtInstallation;

const REQUIRED_QT_VERSION: &str = "6.11.2";

fn main() {
    println!("cargo:rerun-if-env-changed=PATH");
    println!("cargo:rerun-if-env-changed=QMAKE");

    let qmake = std::env::var_os("QMAKE").unwrap_or_else(|| "qmake".into());
    let output = Command::new(&qmake)
        .args(["-query", "QT_VERSION"])
        .output()
        .unwrap_or_else(|error| {
            panic!(
                "failed to run {:?}: {error}; install Qt {REQUIRED_QT_VERSION} with scripts/qt/install_qt.sh",
                qmake
            )
        });

    assert!(
        output.status.success(),
        "qmake failed while checking the Qt version: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );

    let actual = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    assert_eq!(
        actual, REQUIRED_QT_VERSION,
        "Hunk requires Qt {REQUIRED_QT_VERSION}, found Qt {actual}"
    );

    build_browser_frame_item();
}

fn build_browser_frame_item() {
    const QT_MODULES: [&str; 4] = ["Core", "Gui", "Qml", "Quick"];
    let qt = QtInstallation::default();
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR is not set"));
    let header = PathBuf::from("src/native/browser_frame_item.h");
    let moc_output = out_dir.join("moc_browser_frame_item.cpp");
    qt.run_moc(&header, &moc_output);

    let mut builder = cc::Build::new();
    builder
        .cpp(true)
        .std("c++17")
        .flag_if_supported("/Zc:__cplusplus")
        .flag_if_supported("/permissive-")
        .include("src/native")
        .file("src/native/browser_frame_item.cpp")
        .file(moc_output);
    qt.configure_builder(&mut builder);
    for include_dir in qt.include_dirs(QT_MODULES, false) {
        builder.include(include_dir);
    }
    for include_dir in qt.include_dirs(["Gui"], true) {
        builder.include(include_dir);
    }
    builder.compile("hunk_desktop_browser_frame");
    qt.link_modules(QT_MODULES);

    println!("cargo:rerun-if-changed=src/native/browser_frame_item.cpp");
    println!("cargo:rerun-if-changed=src/native/browser_frame_item.h");
}
