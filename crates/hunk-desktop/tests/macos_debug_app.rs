const DEBUG_APP_SCRIPT: &str = include_str!("../../../scripts/run_macos_debug_app.sh");

#[test]
fn macos_debug_app_has_an_independent_addressable_identity() {
    assert!(DEBUG_APP_SCRIPT.contains("com.niteshbalusu.hunk.qt-functional"));
    assert!(DEBUG_APP_SCRIPT.contains("target/functional/HunkQt.app"));
    assert!(DEBUG_APP_SCRIPT.contains("HUNK_CODEX_EXECUTABLE"));
    assert!(DEBUG_APP_SCRIPT.contains("install_name_tool -add_rpath"));
}

#[test]
fn macos_debug_app_reuses_cached_build_and_cef_inputs() {
    assert!(DEBUG_APP_SCRIPT.contains("nix develop --accept-flake-config"));
    assert!(DEBUG_APP_SCRIPT.contains("--no-build"));
    assert!(DEBUG_APP_SCRIPT.contains("cmp -s"));
    assert!(DEBUG_APP_SCRIPT.contains("Reusing the staged CEF runtime"));
}
