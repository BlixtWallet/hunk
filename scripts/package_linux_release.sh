#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_TRIPLE="${HUNK_LINUX_TARGET:-x86_64-unknown-linux-gnu}"
TARGET_DIR="$("$ROOT_DIR/scripts/resolve_cargo_target_dir.sh" "$ROOT_DIR")"
VERSION_LABEL="${HUNK_RELEASE_VERSION:-$("$ROOT_DIR/scripts/resolve_hunk_version.sh")}"
DIST_DIR="$TARGET_DIR/dist"
PACKAGE_DIR="$DIST_DIR/Hunk-$VERSION_LABEL-linux-x86_64"
ARCHIVE_PATH="$DIST_DIR/Hunk-$VERSION_LABEL-linux-x86_64.tar.gz"
APPIMAGE_WORK_DIR="$TARGET_DIR/$TARGET_TRIPLE/release/appimage"
APPDIR_PATH="$APPIMAGE_WORK_DIR/Hunk.AppDir"
APPIMAGE_PATH="$DIST_DIR/Hunk-$VERSION_LABEL-linux-x86_64.AppImage"
SQUASHFS_PATH="$APPIMAGE_WORK_DIR/Hunk-$VERSION_LABEL-linux-x86_64.squashfs"
APPIMAGE_RUNTIME_PATH="$APPIMAGE_WORK_DIR/runtime-x86_64"
APPIMAGE_RUNTIME_URL="https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64"
DESKTOP_ENTRY_PATH="$APPDIR_PATH/hunk.desktop"
APPDIR_BIN_DIR="$APPDIR_PATH/usr/bin"
APPDIR_ICON_PATH="$APPDIR_PATH/hunk.png"
APPDIR_ICON_THEME_PATH="$APPDIR_PATH/usr/share/icons/hicolor/512x512/apps/hunk.png"
APPDIR_APPLICATIONS_DESKTOP_PATH="$APPDIR_PATH/usr/share/applications/hunk.desktop"
BINARY_SOURCE_PATH="$TARGET_DIR/$TARGET_TRIPLE/release/hunk_desktop"
PACKAGED_BINARY_PATH="$PACKAGE_DIR/hunk-desktop"
APPDIR_BINARY_PATH="$APPDIR_BIN_DIR/hunk-desktop"
CODEX_SOURCE_PATH="$TARGET_DIR/$TARGET_TRIPLE/release/codex-runtime/linux/codex"
PACKAGED_CODEX_PATH="$PACKAGE_DIR/codex-runtime/linux/codex"
APPDIR_CODEX_PATH="$APPDIR_BIN_DIR/codex-runtime/linux/codex"
ICON_SOURCE_PATH="$ROOT_DIR/assets/icons/hunk-icon-default.png"

create_linux_desktop_entry() {
  local output_path="$1"
  cat >"$output_path" <<'EOF'
[Desktop Entry]
Type=Application
Name=Hunk
Comment=Fast Git diff viewer.
Exec=hunk-desktop
Icon=hunk
Categories=Development;
Terminal=false
EOF
}

echo "Downloading bundled Codex runtime for Linux..." >&2
"$ROOT_DIR/scripts/download_codex_runtime_unix.sh" linux >/dev/null
echo "Validating bundled Codex runtime for Linux..." >&2
"$ROOT_DIR/scripts/validate_codex_runtime_bundle.sh" --strict --platform linux >/dev/null
echo "Building Linux release binary..." >&2
"$ROOT_DIR/scripts/build_linux.sh" --target "$TARGET_TRIPLE"

if ! command -v mksquashfs >/dev/null 2>&1; then
  echo "error: mksquashfs is required to build the Linux AppImage (install squashfs-tools)" >&2
  exit 1
fi

if [[ ! -f "$ICON_SOURCE_PATH" ]]; then
  echo "error: expected Linux icon asset at $ICON_SOURCE_PATH" >&2
  exit 1
fi

rm -rf "$PACKAGE_DIR" "$APPDIR_PATH" "$ARCHIVE_PATH" "$APPIMAGE_PATH" "$SQUASHFS_PATH"
mkdir -p "$PACKAGE_DIR/codex-runtime/linux"
mkdir -p "$APPDIR_BIN_DIR/codex-runtime/linux"
mkdir -p "$(dirname "$APPDIR_ICON_THEME_PATH")" "$(dirname "$APPDIR_APPLICATIONS_DESKTOP_PATH")"

cp "$BINARY_SOURCE_PATH" "$PACKAGED_BINARY_PATH"
cp "$CODEX_SOURCE_PATH" "$PACKAGED_CODEX_PATH"
chmod +x "$PACKAGED_BINARY_PATH" "$PACKAGED_CODEX_PATH"

cp "$BINARY_SOURCE_PATH" "$APPDIR_BINARY_PATH"
cp "$CODEX_SOURCE_PATH" "$APPDIR_CODEX_PATH"
cp "$ICON_SOURCE_PATH" "$APPDIR_ICON_PATH"
cp "$ICON_SOURCE_PATH" "$APPDIR_ICON_THEME_PATH"
chmod +x "$APPDIR_BINARY_PATH" "$APPDIR_CODEX_PATH"

create_linux_desktop_entry "$DESKTOP_ENTRY_PATH"
cp "$DESKTOP_ENTRY_PATH" "$APPDIR_APPLICATIONS_DESKTOP_PATH"
ln -s usr/bin/hunk-desktop "$APPDIR_PATH/AppRun"
ln -s hunk.png "$APPDIR_PATH/.DirIcon"

mkdir -p "$DIST_DIR"
tar -C "$DIST_DIR" -czf "$ARCHIVE_PATH" "$(basename "$PACKAGE_DIR")"

mkdir -p "$APPIMAGE_WORK_DIR"
if [[ ! -s "$APPIMAGE_RUNTIME_PATH" ]]; then
  echo "Downloading AppImage runtime from $APPIMAGE_RUNTIME_URL..." >&2
  curl --fail --location --silent --show-error "$APPIMAGE_RUNTIME_URL" --output "$APPIMAGE_RUNTIME_PATH"
fi

echo "Packing Linux AppDir into squashfs..." >&2
mksquashfs "$APPDIR_PATH" "$SQUASHFS_PATH" -root-owned -noappend -quiet
cat "$APPIMAGE_RUNTIME_PATH" "$SQUASHFS_PATH" > "$APPIMAGE_PATH"
chmod +x "$APPIMAGE_PATH"

echo "Created Linux AppImage artifact at $APPIMAGE_PATH" >&2
echo "Created Linux release artifact at $ARCHIVE_PATH" >&2

printf '%s\n' "$APPIMAGE_PATH"
