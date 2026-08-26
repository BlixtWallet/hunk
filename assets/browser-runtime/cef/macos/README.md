# Hunk CEF Runtime: macOS

This folder is reserved for the bundled CEF runtime used by Hunk's embedded AI browser.

Pinned runtime target:

- OS/architecture: `aarch64-apple-darwin`
- Rust binding: `tauri-apps/cef-rs`
- Binding release: `cef-v151.8.0+151.3.24`
- Binding commit: `a2e15ae659c4b3957883e34de879bd8b38360ce5`
- CEF version: `151.3.24+g2384915+chromium-151.0.7922.174`
- Download source used by cef-rs: `https://cef-builds.spotifycdn.com`
- Current archive: `cef_binary_151.3.24+g2384915+chromium-151.0.7922.174_macosarm64_minimal.tar.bz2`
- Current archive SHA-1: `82af2c0cadaafc4ad057f54c14cb3791cc139852`

The exported runtime is generated under `assets/browser-runtime/cef/macos/runtime` and is intentionally ignored by git. Recreate it with:

```sh
nix develop -c ./scripts/prepare_browser_cef_runtime.sh \
  aarch64-apple-darwin \
  assets/browser-runtime/cef/macos/runtime
```

Refresh the pinned runtime metadata by updating:

- `HUNK_CEF_RS_REV` in `scripts/prepare_browser_cef_runtime.sh` and `scripts/prepare_browser_cef_runtime_windows.ps1` when moving to a newer cef-rs commit.
- The candidate binding and CEF version lines in this README.
- The archive name and SHA-1 from `assets/browser-runtime/cef/macos/runtime/archive.json` after export.
- The notes in `docs/AI_BROWSER_CEF_TODO.md`.

Then rerun:

```sh
HUNK_CEF_FORCE_EXPORT=1 nix develop -c ./scripts/prepare_browser_cef_runtime.sh \
  aarch64-apple-darwin \
  assets/browser-runtime/cef/macos/runtime
```

Validate an existing staged runtime with:

```sh
nix develop -c ./scripts/validate_browser_cef_macos.sh
```

Validate both a staged runtime and an app bundle with:

```sh
nix develop -c ./scripts/validate_browser_cef_macos.sh \
  assets/browser-runtime/cef/macos/runtime \
  target/packager/macos/Hunk.app
```

Package the staged runtime into an existing macOS app bundle with:

```sh
nix develop -c cargo build -p hunk-browser-helper --release --target aarch64-apple-darwin
nix develop -c ./scripts/package_browser_cef_macos.sh \
  target/packager/macos/Hunk.app \
  assets/browser-runtime/cef/macos/runtime \
  target/aarch64-apple-darwin/release/hunk-browser-helper
```

Expected staged files:

- `Chromium Embedded Framework.framework`
- CEF resources and locales
- CEF snapshot/blob assets required by the selected CEF build
- `hunk-browser-helper` subprocess binary or helper app
- `archive.json` or an equivalent pinned manifest with source URL, version, size, and checksum

Release and PR packaging now use `scripts/prepare_browser_cef_runtime.sh` to
download/export the pinned CEF runtime through cef-rs before validating and
building browser-enabled Hunk binaries.

Checksum process:

1. Download/export the pinned CEF runtime with the selected cef-rs tooling.
2. Record the source URL, archive file name, byte size, and SHA-256 in a manifest.
3. Validate the staged runtime before packaging Hunk.
4. Fail packaging if the staged files do not match the manifest.
