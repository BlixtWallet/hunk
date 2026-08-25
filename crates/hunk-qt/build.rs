use std::process::Command;

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
}
