#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

APP_PATH="$ROOT_DIR/target/functional/HunkQt.app"
APP_CONTENTS_DIR="$APP_PATH/Contents"
APP_EXECUTABLE_PATH="$APP_CONTENTS_DIR/MacOS/HunkQt"
APP_FRAMEWORKS_DIR="$APP_CONTENTS_DIR/Frameworks"
BROWSER_CEF_RUNTIME_DIR="${HUNK_CEF_RUNTIME_DIR:-$ROOT_DIR/assets/browser-runtime/cef/macos/runtime}"
BROWSER_HELPER_PATH="$ROOT_DIR/target/debug/hunk-browser-helper"
CODEX_RUNTIME_PATH="$ROOT_DIR/assets/codex-runtime/macos/codex"
CEF_ARCHIVE_MARKER="$APP_FRAMEWORKS_DIR/.hunk-cef-archive.json"
SKIP_BUILD=0

usage() {
  cat <<'EOF'
Build and launch an addressable macOS Qt debug app for native UI testing.

Usage:
  ./scripts/run_macos_debug_app.sh [--no-build]

The app lives under target/functional with its own bundle identifier, so it can
run beside /Applications/Hunk.app and be targeted independently by accessibility
and Computer Use tooling. CEF is staged only when its archive metadata changes.
EOF
}

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "error: the macOS debug app is only available on macOS" >&2
  exit 1
fi

if [[ $# -gt 1 ]]; then
  usage >&2
  exit 1
fi
if [[ $# -eq 1 ]]; then
  case "$1" in
    --no-build) SKIP_BUILD=1 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; exit 1 ;;
  esac
fi

if [[ ! -x "$CODEX_RUNTIME_PATH" ]]; then
  echo "error: bundled Codex runtime is missing: $CODEX_RUNTIME_PATH" >&2
  exit 1
fi
if [[ ! -f "$BROWSER_CEF_RUNTIME_DIR/archive.json" ]]; then
  echo "error: staged CEF runtime is missing: $BROWSER_CEF_RUNTIME_DIR" >&2
  exit 1
fi

if [[ $SKIP_BUILD -eq 0 ]]; then
  (
    cd "$ROOT_DIR"
    nix develop --accept-flake-config -c bash -lc '
      set -euo pipefail
      export CEF_PATH="$PWD/assets/browser-runtime/cef/macos/runtime"
      export DYLD_FALLBACK_LIBRARY_PATH="${DYLD_FALLBACK_LIBRARY_PATH:-}:$CEF_PATH:$CEF_PATH/Chromium Embedded Framework.framework/Libraries"
      cargo build -p hunk-desktop --locked --features hunk-desktop/cef-browser
      cargo build -p hunk-browser-helper --locked --features hunk-browser-helper/cef-subprocess
    '
  )
fi

# Load Qt only after the Nix build so host Qt flags do not invalidate Cargo's
# cached Nix build fingerprints.
# shellcheck disable=SC1091
source "$ROOT_DIR/scripts/qt/qt_env.sh"

if [[ -z "${HUNK_QT_ROOT:-}" || ! -d "$HUNK_QT_ROOT/lib/QtCore.framework" ]]; then
  echo "error: the pinned Qt 6.11.2 SDK was not found; run scripts/qt/install_qt.sh" >&2
  exit 1
fi

if [[ ! -x "$ROOT_DIR/target/debug/hunk_desktop" || ! -x "$BROWSER_HELPER_PATH" ]]; then
  echo "error: debug binaries are missing; rerun without --no-build" >&2
  exit 1
fi
if pgrep -f "^$APP_EXECUTABLE_PATH$" >/dev/null 2>&1; then
  echo "error: Hunk Qt is already running; close it before refreshing the debug bundle" >&2
  exit 1
fi

mkdir -p "$APP_CONTENTS_DIR/MacOS" "$APP_FRAMEWORKS_DIR"
cat > "$APP_CONTENTS_DIR/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>English</string>
  <key>CFBundleDisplayName</key>
  <string>Hunk Qt</string>
  <key>CFBundleExecutable</key>
  <string>HunkQt</string>
  <key>CFBundleIdentifier</key>
  <string>com.niteshbalusu.hunk.qt-functional</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>Hunk Qt</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>0.0.11</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.developer-tools</string>
  <key>LSMinimumSystemVersion</key>
  <string>14.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>LSEnvironment</key>
  <dict>
    <key>HUNK_BROWSER_PROFILE_ID</key>
    <string>qt-functional</string>
    <key>HUNK_CODEX_EXECUTABLE</key>
    <string>$CODEX_RUNTIME_PATH</string>
    <key>QML_IMPORT_PATH</key>
    <string>$HUNK_QT_ROOT/qml</string>
    <key>QT_PLUGIN_PATH</key>
    <string>$HUNK_QT_ROOT/plugins</string>
  </dict>
</dict>
</plist>
PLIST

if [[ ! -d "$APP_FRAMEWORKS_DIR/Chromium Embedded Framework.framework" ]] \
  || [[ ! -f "$CEF_ARCHIVE_MARKER" ]] \
  || ! cmp -s "$BROWSER_CEF_RUNTIME_DIR/archive.json" "$CEF_ARCHIVE_MARKER"; then
  "$ROOT_DIR/scripts/package_browser_cef_macos.sh" \
    "$APP_PATH" \
    "$BROWSER_CEF_RUNTIME_DIR" \
    "$BROWSER_HELPER_PATH"
  cp "$BROWSER_CEF_RUNTIME_DIR/archive.json" "$CEF_ARCHIVE_MARKER"
else
  echo "Reusing the staged CEF runtime in $APP_PATH" >&2
fi

if [[ -e "$APP_EXECUTABLE_PATH" ]]; then
  unlink "$APP_EXECUTABLE_PATH"
fi
cp -c "$ROOT_DIR/target/debug/hunk_desktop" "$APP_EXECUTABLE_PATH"
install_name_tool -add_rpath "$HUNK_QT_ROOT/lib" "$APP_EXECUTABLE_PATH"
codesign --force --deep --sign - "$APP_PATH"

open "$APP_PATH"
echo "Launched Hunk Qt from $APP_PATH" >&2
