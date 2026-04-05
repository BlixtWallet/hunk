use std::collections::BTreeMap;

use hunk_updater::{
    AssetFormat, ReleaseAsset, ReleaseManifest, UpdateCheckResult, current_update_target,
    detect_install_source, evaluate_manifest, install_source_from_explanation,
};

fn sample_manifest(version: &str) -> ReleaseManifest {
    let mut platforms = BTreeMap::new();
    platforms.insert(
        "macos-aarch64".to_string(),
        ReleaseAsset {
            url: "https://example.com/Hunk.app.tar.gz".to_string(),
            signature: "sig-macos".to_string(),
            format: AssetFormat::App,
        },
    );
    platforms.insert(
        "windows-x86_64".to_string(),
        ReleaseAsset {
            url: "https://example.com/Hunk.msi".to_string(),
            signature: "sig-windows".to_string(),
            format: AssetFormat::Wix,
        },
    );
    platforms.insert(
        "linux-x86_64".to_string(),
        ReleaseAsset {
            url: "https://example.com/Hunk.tar.gz".to_string(),
            signature: "sig-linux".to_string(),
            format: AssetFormat::Tarball,
        },
    );

    ReleaseManifest {
        version: version.to_string(),
        pub_date: Some("2026-04-05T20:00:00Z".to_string()),
        notes: Some("Notes".to_string()),
        platforms,
    }
}

#[test]
fn manifest_update_result_uses_target_asset() {
    let result = evaluate_manifest(
        "https://updates.hunk.dev/stable.json",
        "0.0.1",
        "linux-x86_64",
        sample_manifest("0.0.2"),
    )
    .expect("manifest should evaluate");

    match result {
        UpdateCheckResult::UpdateAvailable(update) => {
            assert_eq!(update.version, "0.0.2");
            assert_eq!(update.target, "linux-x86_64");
            assert_eq!(update.asset.signature, "sig-linux");
            assert_eq!(update.asset.format, AssetFormat::Tarball);
        }
        other => panic!("expected update available, got {other:?}"),
    }
}

#[test]
fn manifest_up_to_date_when_remote_is_not_newer() {
    let result = evaluate_manifest(
        "https://updates.hunk.dev/stable.json",
        "0.0.2",
        "windows-x86_64",
        sample_manifest("0.0.2"),
    )
    .expect("manifest should evaluate");

    assert_eq!(
        result,
        UpdateCheckResult::UpToDate {
            version: "0.0.2".to_string()
        }
    );
}

#[test]
fn prerelease_manifest_versions_are_rejected() {
    let error = evaluate_manifest(
        "https://updates.hunk.dev/stable.json",
        "0.0.1",
        "macos-aarch64",
        sample_manifest("0.0.2-alpha.1"),
    )
    .expect_err("prerelease manifest version should fail");

    assert!(
        error
            .to_string()
            .contains("invalid update manifest version"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn install_source_uses_package_manager_explanation_when_present() {
    let source = install_source_from_explanation(Some(
        "This Hunk install is managed by apt. Update it with apt upgrade.",
    ));

    assert_eq!(
        source.explanation(),
        Some("This Hunk install is managed by apt. Update it with apt upgrade.")
    );
}

#[test]
fn install_source_defaults_to_self_managed_when_explanation_is_missing() {
    assert!(matches!(
        install_source_from_explanation(None),
        hunk_updater::InstallSource::SelfManaged
    ));
}

#[test]
fn install_source_ignores_blank_explanations() {
    assert!(matches!(
        install_source_from_explanation(Some("   ")),
        hunk_updater::InstallSource::SelfManaged
    ));
}

#[test]
fn supported_targets_include_the_current_platform() {
    let target = current_update_target().expect("current platform should be supported");

    assert!(!target.is_empty());
}

#[test]
fn detect_install_source_reads_environment_override() {
    unsafe {
        std::env::set_var(
            hunk_updater::UPDATE_EXPLANATION_ENV_VAR,
            "Managed by package manager",
        );
    }
    let source = detect_install_source();
    unsafe {
        std::env::remove_var(hunk_updater::UPDATE_EXPLANATION_ENV_VAR);
    }

    assert_eq!(source.explanation(), Some("Managed by package manager"));
}
