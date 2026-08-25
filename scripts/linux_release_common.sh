#!/usr/bin/env bash

if [[ -n "${HUNK_LINUX_RELEASE_COMMON_SOURCED:-}" ]]; then
  return 0
fi
HUNK_LINUX_RELEASE_COMMON_SOURCED=1

ROOT_DIR="${ROOT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
# shellcheck disable=SC1091
source "$ROOT_DIR/scripts/qt/qt_env.sh"
TARGET_TRIPLE="${HUNK_LINUX_TARGET:-x86_64-unknown-linux-gnu}"
TARGET_DIR="${TARGET_DIR:-$ROOT_DIR/target}"
VERSION_LABEL="${HUNK_RELEASE_VERSION:-$("$ROOT_DIR/scripts/resolve_hunk_version.sh")}"
PRODUCT_NAME="${HUNK_LINUX_PRODUCT_NAME:-Hunk}"
PACKAGE_NAME="${HUNK_LINUX_PACKAGE_NAME:-hunk-desktop}"
PACKAGE_VENDOR="${HUNK_LINUX_PACKAGE_VENDOR:-Nitesh Balusu}"
PACKAGE_MAINTAINER="${HUNK_LINUX_PACKAGE_MAINTAINER:-Nitesh Balusu <hunk@example.com>}"
PACKAGE_LICENSE="${HUNK_LINUX_PACKAGE_LICENSE:-LicenseRef-Unknown}"
PACKAGE_HOMEPAGE="${HUNK_LINUX_PACKAGE_HOMEPAGE:-https://github.com/BlixtWallet/hunk}"
PACKAGE_SUMMARY="${HUNK_LINUX_PACKAGE_SUMMARY:-Very fast git diff viewer and codex orchestrator.}"
PACKAGE_DESCRIPTION="${HUNK_LINUX_PACKAGE_DESCRIPTION:-A super fast, simple Git diff viewer and Codex orchestrator built with Qt Quick.}"
PACKAGE_SECTION="${HUNK_LINUX_PACKAGE_SECTION:-utils}"
PACKAGE_PRIORITY="${HUNK_LINUX_PACKAGE_PRIORITY:-optional}"
PACKAGE_RELEASE="${HUNK_LINUX_PACKAGE_RELEASE:-1}"
WORK_DIR="$TARGET_DIR/linux-packaging"
DIST_DIR="$TARGET_DIR/dist"
ARCH_LABEL=""
PACKAGE_DIR=""
ARCHIVE_PATH=""
SYSTEM_INSTALL_ROOT=""
SYSTEM_BIN_DIR=""
SYSTEM_LIB_DIR=""
SYSTEM_PRIVATE_LIB_DIR=""
SYSTEM_REAL_BINARY_PATH=""
SYSTEM_LAUNCHER_PATH=""
SYSTEM_RUNTIME_PATH=""
SYSTEM_DESKTOP_ENTRY_PATH=""
SYSTEM_ICON_DIR=""
SYSTEM_ICON_PATH=""
SYSTEM_ICON_ALIAS_PATH=""
SYSTEM_PIXMAP_DIR=""
SYSTEM_PIXMAP_PATH=""
SYSTEM_WRAPPER_PATH=""
SYSTEM_WRAPPER_ALIAS_PATH=""
SYSTEM_QT_CONF_PATH=""
DEB_BUILD_ROOT=""
DEB_ARCH=""
DEB_VERSION=""
DEB_PATH=""
RPM_TOPDIR=""
RPM_ARCH=""
RPM_VERSION=""
RPM_PATH=""
BINARY_SOURCE_PATH=""
REAL_BINARY_NAME="hunk_desktop_bin"
LINUX_ICON_SOURCE_PATH="$ROOT_DIR/assets/icons/hunk_linux_512.png"
PACKAGED_BINARY_PATH=""
PACKAGED_LAUNCHER_PATH=""
PACKAGE_LIB_DIR=""
PACKAGE_QT_DIR=""
PACKAGE_QT_QML_DIR=""
PACKAGE_QT_PLUGIN_DIR=""
PACKAGED_QT_CONF_PATH=""
CODEX_SOURCE_PATH=""
PACKAGED_CODEX_PATH=""
BROWSER_CEF_SOURCE_DIR=""
BROWSER_HELPER_SOURCE_PATH=""
PACKAGED_BROWSER_HELPER_PATH=""

linux_target_arch() {
  printf '%s\n' "${TARGET_TRIPLE%%-*}"
}

linux_dist_arch_label() {
  case "$(linux_target_arch)" in
    x86_64)
      printf '%s\n' "x86_64"
      ;;
    aarch64)
      printf '%s\n' "arm64"
      ;;
    *)
      printf '%s\n' "$(linux_target_arch)"
      ;;
  esac
}

linux_deb_arch() {
  case "$(linux_target_arch)" in
    x86_64)
      printf '%s\n' "amd64"
      ;;
    aarch64)
      printf '%s\n' "arm64"
      ;;
    armv7*)
      printf '%s\n' "armhf"
      ;;
    *)
      echo "error: unsupported Debian architecture for target '$TARGET_TRIPLE'" >&2
      exit 1
      ;;
  esac
}

linux_rpm_arch() {
  case "$(linux_target_arch)" in
    x86_64)
      printf '%s\n' "x86_64"
      ;;
    aarch64)
      printf '%s\n' "aarch64"
      ;;
    *)
      printf '%s\n' "$(linux_target_arch)"
      ;;
  esac
}

linux_deb_version() {
  printf '%s-%s\n' "$VERSION_LABEL" "$PACKAGE_RELEASE"
}

linux_rpm_version() {
  local version="$VERSION_LABEL"
  if [[ "$version" == *-* ]]; then
    local base="${version%%-*}"
    local suffix="${version#*-}"
    suffix="${suffix//-/_}"
    printf '%s~%s\n' "$base" "$suffix"
  else
    printf '%s\n' "$version"
  fi
}

linux_rpm_changelog_date() {
  LC_ALL=C date -u +"%a %b %d %Y"
}

init_linux_release_paths() {
  ARCH_LABEL="$(linux_dist_arch_label)"
  PACKAGE_DIR="$WORK_DIR/tarball/${PRODUCT_NAME}-${VERSION_LABEL}-linux-$ARCH_LABEL"
  ARCHIVE_PATH="$DIST_DIR/${PRODUCT_NAME}-${VERSION_LABEL}-linux-$ARCH_LABEL.tar.gz"
  SYSTEM_INSTALL_ROOT="$WORK_DIR/system-root"
  SYSTEM_BIN_DIR="$SYSTEM_INSTALL_ROOT/usr/bin"
  SYSTEM_LIB_DIR="$SYSTEM_INSTALL_ROOT/usr/lib/$PACKAGE_NAME"
  SYSTEM_PRIVATE_LIB_DIR="$SYSTEM_LIB_DIR/lib"
  SYSTEM_REAL_BINARY_PATH="$SYSTEM_LIB_DIR/$REAL_BINARY_NAME"
  SYSTEM_LAUNCHER_PATH="$SYSTEM_LIB_DIR/$PACKAGE_NAME"
  SYSTEM_RUNTIME_PATH="$SYSTEM_LIB_DIR/codex-runtime/linux/codex"
  SYSTEM_DESKTOP_ENTRY_PATH="$SYSTEM_INSTALL_ROOT/usr/share/applications/$PACKAGE_NAME.desktop"
  SYSTEM_ICON_DIR="$SYSTEM_INSTALL_ROOT/usr/share/icons/hicolor/512x512/apps"
  SYSTEM_ICON_PATH="$SYSTEM_ICON_DIR/$PACKAGE_NAME.png"
  SYSTEM_ICON_ALIAS_PATH="$SYSTEM_ICON_DIR/${PACKAGE_NAME//-/_}.png"
  SYSTEM_PIXMAP_DIR="$SYSTEM_INSTALL_ROOT/usr/share/pixmaps"
  SYSTEM_PIXMAP_PATH="$SYSTEM_PIXMAP_DIR/$PACKAGE_NAME.png"
  SYSTEM_WRAPPER_PATH="$SYSTEM_BIN_DIR/$PACKAGE_NAME"
  SYSTEM_WRAPPER_ALIAS_PATH="$SYSTEM_BIN_DIR/${PACKAGE_NAME//-/_}"
  SYSTEM_QT_CONF_PATH="$SYSTEM_LIB_DIR/qt.conf"
  DEB_BUILD_ROOT="$WORK_DIR/deb-root"
  DEB_ARCH="$(linux_deb_arch)"
  DEB_VERSION="$(linux_deb_version)"
  DEB_PATH="$DIST_DIR/${PACKAGE_NAME}_${DEB_VERSION}_${DEB_ARCH}.deb"
  RPM_TOPDIR="$WORK_DIR/rpmbuild"
  RPM_ARCH="$(linux_rpm_arch)"
  RPM_VERSION="$(linux_rpm_version)"
  RPM_PATH="$DIST_DIR/${PACKAGE_NAME}-${RPM_VERSION}-${PACKAGE_RELEASE}.${RPM_ARCH}.rpm"
  BINARY_SOURCE_PATH="$TARGET_DIR/$TARGET_TRIPLE/release/hunk_desktop"
  PACKAGED_BINARY_PATH="$PACKAGE_DIR/$REAL_BINARY_NAME"
  PACKAGED_LAUNCHER_PATH="$PACKAGE_DIR/$PACKAGE_NAME"
  PACKAGE_LIB_DIR="$PACKAGE_DIR/lib"
  PACKAGE_QT_DIR="$PACKAGE_LIB_DIR/qt6"
  PACKAGE_QT_QML_DIR="$PACKAGE_QT_DIR/qml"
  PACKAGE_QT_PLUGIN_DIR="$PACKAGE_QT_DIR/plugins"
  PACKAGED_QT_CONF_PATH="$PACKAGE_DIR/qt.conf"
  CODEX_SOURCE_PATH="$TARGET_DIR/$TARGET_TRIPLE/release/codex-runtime/linux/codex"
  PACKAGED_CODEX_PATH="$PACKAGE_DIR/codex-runtime/linux/codex"
  BROWSER_CEF_SOURCE_DIR="${HUNK_BROWSER_CEF_LINUX_RUNTIME_DIR:-$ROOT_DIR/assets/browser-runtime/cef/linux/runtime}"
  BROWSER_HELPER_SOURCE_PATH="$TARGET_DIR/$TARGET_TRIPLE/release/hunk-browser-helper"
  PACKAGED_BROWSER_HELPER_PATH="$PACKAGE_DIR/hunk-browser-helper"
}

require_linux_tool() {
  local tool_name="$1"
  if ! command -v "$tool_name" >/dev/null 2>&1; then
    echo "error: required Linux packaging tool '$tool_name' is not installed" >&2
    exit 1
  fi
}

require_linux_qt_sdk() {
  local required_version="6.11.2"

  if [[ -z "${HUNK_QT_ROOT:-}" || ! -x "$HUNK_QT_ROOT/bin/qmake" ]]; then
    echo "error: Qt $required_version SDK is required for Linux packaging" >&2
    echo "hint: install the pinned SDK or set QT_ROOT_DIR before entering the Nix shell" >&2
    exit 1
  fi

  local actual_version
  actual_version="$("$HUNK_QT_ROOT/bin/qmake" -query QT_VERSION)"
  if [[ "$actual_version" != "$required_version" ]]; then
    echo "error: Hunk requires Qt $required_version, found Qt $actual_version at $HUNK_QT_ROOT" >&2
    exit 1
  fi
}

stage_linux_qt_runtime() {
  local plugin_group
  local -a plugin_groups=(
    accessiblebridge
    egldeviceintegrations
    generic
    iconengines
    imageformats
    networkinformation
    platforminputcontexts
    platforms
    platformthemes
    styles
    tls
    wayland-decoration-client
    wayland-graphics-integration-client
    wayland-shell-integration
    xcbglintegrations
  )

  mkdir -p "$PACKAGE_QT_QML_DIR" "$PACKAGE_QT_PLUGIN_DIR"
  cp -LR "$HUNK_QT_ROOT/qml"/. "$PACKAGE_QT_QML_DIR"/

  for plugin_group in "${plugin_groups[@]}"; do
    if [[ -d "$HUNK_QT_ROOT/plugins/$plugin_group" ]]; then
      mkdir -p "$PACKAGE_QT_PLUGIN_DIR/$plugin_group"
      cp -LR "$HUNK_QT_ROOT/plugins/$plugin_group"/. "$PACKAGE_QT_PLUGIN_DIR/$plugin_group"/
    fi
  done

  cat >"$PACKAGED_QT_CONF_PATH" <<'EOF'
[Paths]
Plugins = lib/qt6/plugins
QmlImports = lib/qt6/qml
EOF

  local required_path
  local -a required_paths=(
    "$PACKAGE_QT_QML_DIR/QtQml/qmldir"
    "$PACKAGE_QT_QML_DIR/QtQuick/qmldir"
    "$PACKAGE_QT_QML_DIR/QtQuick/Controls/qmldir"
    "$PACKAGE_QT_PLUGIN_DIR/platforms/libqxcb.so"
    "$PACKAGE_QT_PLUGIN_DIR/platforms/libqwayland-egl.so"
    "$PACKAGE_QT_PLUGIN_DIR/platforms/libqwayland-generic.so"
  )
  for required_path in "${required_paths[@]}"; do
    if [[ ! -f "$required_path" ]]; then
      echo "error: Linux Qt deployment is missing $required_path" >&2
      exit 1
    fi
  done

  echo "Staged Qt QML modules plus X11 and Wayland plugins into the Linux bundle." >&2
}

should_bundle_linux_library() {
  local library_name="$1"

  case "$library_name" in
    linux-vdso.so.*|linux-gate.so.*|ld-linux*.so.*|ld-musl-*.so.*)
      return 1
      ;;
    libc.so.*|libm.so.*|libpthread.so.*|librt.so.*|libdl.so.*|libutil.so.*|libresolv.so.*|libnsl.so.*|libanl.so.*|libBrokenLocale.so.*)
      return 1
      ;;
    *)
      return 0
      ;;
  esac
}

linux_dependency_ld_library_path() {
  local build_dir="$TARGET_DIR/$TARGET_TRIPLE/release/build"
  local extra_path="${HUNK_LINUX_EXTRA_LIBRARY_PATH:-}"

  append_extra_path() {
    local candidate="$1"

    [[ -n "$candidate" && -d "$candidate" ]] || return 0

    case ":$extra_path:" in
      *":$candidate:"*) ;;
      *)
        extra_path="${extra_path:+$extra_path:}$candidate"
        ;;
    esac
  }

  if [[ -n "${HUNK_LINUX_PACKAGING_LIBRARY_PATH:-}" ]]; then
    local -a packaging_paths
    local packaging_path
    IFS=':' read -r -a packaging_paths <<<"$HUNK_LINUX_PACKAGING_LIBRARY_PATH"
    for packaging_path in "${packaging_paths[@]}"; do
      append_extra_path "$packaging_path"
    done
  fi

  if [[ -n "${NIX_LDFLAGS:-}" ]]; then
    local next_is_library_path=0
    local flag
    for flag in $NIX_LDFLAGS; do
      if [[ "$next_is_library_path" == "1" ]]; then
        append_extra_path "$flag"
        next_is_library_path=0
        continue
      fi

      case "$flag" in
        -L)
          next_is_library_path=1
          ;;
        -L*)
          append_extra_path "${flag#-L}"
          ;;
      esac
    done
  fi

  if [[ -d "$build_dir" ]]; then
    local discovered_paths=""
    while IFS= read -r discovered_dir; do
      [[ -n "$discovered_dir" ]] || continue
      discovered_paths="${discovered_paths:+$discovered_paths:}$discovered_dir"
    done < <(find "$build_dir" -type d -path '*/out/ghostty-install/lib' | sort -u)

    if [[ -n "$discovered_paths" ]]; then
      extra_path="${extra_path:+$extra_path:}$discovered_paths"
    fi
  fi

  printf '%s\n' "$extra_path"
}

list_linux_runtime_dependencies() {
  local target_path="$1"
  local ldd_output
  local extra_library_path

  extra_library_path="$(linux_dependency_ld_library_path)"

  if [[ -n "$extra_library_path" ]]; then
    ldd_output="$(env LD_LIBRARY_PATH="${extra_library_path}${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" ldd "$target_path")"
  else
    ldd_output="$(ldd "$target_path")"
  fi
  if grep -Fq "not found" <<<"$ldd_output"; then
    echo "error: unresolved Linux runtime dependencies for $target_path" >&2
    echo "$ldd_output" >&2
    exit 1
  fi

  while IFS= read -r line; do
    line="${line#"${line%%[![:space:]]*}"}"

    if [[ "$line" == *"=>"* ]]; then
      line="${line#*=> }"
    elif [[ "$line" != /* ]]; then
      continue
    fi

    line="${line%% *}"
    if [[ "$line" == /* ]]; then
      printf '%s\n' "$line"
    fi
  done <<<"$ldd_output"
}

bundle_linux_runtime_dependency_roots() {
  local destination_dir="$1"
  shift
  local -A seen_paths=()
  local -A seen_names=()
  local -a queue=("$@")

  while [[ ${#queue[@]} -gt 0 ]]; do
    local current="${queue[0]}"
    queue=("${queue[@]:1}")

    local dependency_output
    if ! dependency_output="$(list_linux_runtime_dependencies "$current")"; then
      exit 1
    fi

    while IFS= read -r dependency_path; do
      [[ -n "$dependency_path" ]] || continue

      local dependency_name
      dependency_name="$(basename "$dependency_path")"
      if ! should_bundle_linux_library "$dependency_name"; then
        continue
      fi

      if [[ -n "${seen_paths[$dependency_path]:-}" ]]; then
        continue
      fi

      if [[ -n "${seen_names[$dependency_name]:-}" && "${seen_names[$dependency_name]}" != "$dependency_path" ]]; then
        echo "error: conflicting Linux dependency paths for $dependency_name:" >&2
        echo "  ${seen_names[$dependency_name]}" >&2
        echo "  $dependency_path" >&2
        exit 1
      fi

      seen_paths["$dependency_path"]=1
      seen_names["$dependency_name"]="$dependency_path"

      echo "Bundling Linux dependency $dependency_name from $dependency_path" >&2
      cp -L "$dependency_path" "$destination_dir/$dependency_name"
      chmod 755 "$destination_dir/$dependency_name"
      queue+=("$dependency_path")
    done <<<"$dependency_output"
  done
}

bundle_linux_runtime_dependencies() {
  local root_binary="$1"
  local destination_dir="$2"
  bundle_linux_runtime_dependency_roots "$destination_dir" "$root_binary"
}

bundle_linux_runtime_tree_dependencies() {
  local runtime_tree="$1"
  local destination_dir="$2"
  local runtime_binary
  local -a runtime_binaries=()

  while IFS= read -r -d '' runtime_binary; do
    if file -b "$runtime_binary" | grep -Fq ELF; then
      runtime_binaries+=("$runtime_binary")
    fi
  done < <(find "$runtime_tree" -type f -name '*.so*' -print0)

  if [[ ${#runtime_binaries[@]} -gt 0 ]]; then
    bundle_linux_runtime_dependency_roots "$destination_dir" "${runtime_binaries[@]}"
  fi
}

find_linux_packaging_library() {
  local library_name="$1"
  local search_path
  local -a search_paths=()

  search_path="$(linux_dependency_ld_library_path)"
  if [[ -n "$search_path" ]]; then
    IFS=':' read -r -a search_paths <<<"$search_path"
  fi

  local search_dir
  for search_dir in "${search_paths[@]}"; do
    local candidate="$search_dir/$library_name"
    if [[ -e "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  return 1
}

bundle_linux_dynamic_library() {
  local library_name="$1"
  local destination_dir="$2"
  local library_path

  if ! library_path="$(find_linux_packaging_library "$library_name")"; then
    echo "error: required Linux dynamic runtime library '$library_name' was not found" >&2
    exit 1
  fi

  echo "Bundling Linux dynamic runtime library $library_name from $library_path" >&2
  cp -L "$library_path" "$destination_dir/$library_name"
  chmod 755 "$destination_dir/$library_name"
  bundle_linux_runtime_dependencies "$library_path" "$destination_dir"
}

bundle_linux_dynamic_runtime_dependencies() {
  local destination_dir="$1"
  local -a dynamic_libraries=(
    libwayland-client.so.0
    libwayland-cursor.so.0
    libwayland-egl.so.1
    libEGL.so.1
    libGL.so.1
    libGLX.so.0
    libGLdispatch.so.0
  )

  local library_name
  for library_name in "${dynamic_libraries[@]}"; do
    bundle_linux_dynamic_library "$library_name" "$destination_dir"
  done
}

patch_linux_runtime_paths() {
  local binary_path="$1"
  local libs_dir="$2"
  local binary_rpath="$3"

  patchelf --set-rpath "$binary_rpath" "$binary_path"

  if [[ -d "$libs_dir" ]]; then
    while IFS= read -r -d '' library_path; do
      patchelf --set-rpath '$ORIGIN' "$library_path"
    done < <(find "$libs_dir" -maxdepth 1 -type f -name '*.so*' -print0)
  fi
}

patch_linux_runtime_tree_paths() {
  local runtime_tree="$1"
  local runtime_binary

  while IFS= read -r -d '' runtime_binary; do
    if file -b "$runtime_binary" | grep -Fq ELF; then
      patchelf --set-rpath '$ORIGIN' "$runtime_binary"
    fi
  done < <(find "$runtime_tree" -type f -name '*.so*' -print0)
}

validate_linux_runtime_bundle() {
  local binary_path="$1"
  local libs_dir="$2"
  local ldd_output

  ldd_output="$(env LD_LIBRARY_PATH="$libs_dir" ldd "$binary_path")"
  if grep -Fq "not found" <<<"$ldd_output"; then
    echo "error: bundled Linux runtime dependencies are incomplete for $binary_path" >&2
    echo "$ldd_output" >&2
    exit 1
  fi
}

validate_linux_runtime_tree() {
  local runtime_tree="$1"
  local libs_dir="$2"
  local runtime_binary

  while IFS= read -r -d '' runtime_binary; do
    if file -b "$runtime_binary" | grep -Fq ELF; then
      validate_linux_runtime_bundle "$runtime_binary" "$libs_dir"
    fi
  done < <(find "$runtime_tree" -type f -name '*.so*' -print0)
}

validate_linux_qt_runtime_layout() {
  local package_root="$1"
  local private_lib_dir="$2"
  local qt_root="$private_lib_dir/qt6"
  local required_path
  local -a required_paths=(
    "$package_root/qt.conf"
    "$private_lib_dir/libQt6Core.so.6"
    "$private_lib_dir/libQt6Gui.so.6"
    "$private_lib_dir/libQt6Qml.so.6"
    "$private_lib_dir/libQt6Quick.so.6"
    "$qt_root/qml/QtQml/qmldir"
    "$qt_root/qml/QtQuick/qmldir"
    "$qt_root/qml/QtQuick/Controls/qmldir"
    "$qt_root/plugins/platforms/libqxcb.so"
    "$qt_root/plugins/platforms/libqwayland-egl.so"
    "$qt_root/plugins/platforms/libqwayland-generic.so"
  )

  for required_path in "${required_paths[@]}"; do
    if [[ ! -f "$required_path" ]]; then
      echo "error: packaged Linux Qt runtime is missing $required_path" >&2
      exit 1
    fi
  done

  if find "$qt_root" -type l -print -quit | grep -q .; then
    echo "error: packaged Linux Qt runtime contains unresolved symbolic links" >&2
    exit 1
  fi

  echo "Verified Linux package contains a self-contained Qt runtime." >&2
}

prepare_linux_release_build_inputs() {
  require_linux_tool patchelf
  require_linux_tool file
  require_linux_qt_sdk

  "$ROOT_DIR/scripts/prepare_browser_cef_runtime.sh" "$TARGET_TRIPLE" "$BROWSER_CEF_SOURCE_DIR" >/dev/null
  export CEF_PATH="$BROWSER_CEF_SOURCE_DIR"
  export LD_LIBRARY_PATH="$BROWSER_CEF_SOURCE_DIR${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  export HUNK_LINUX_EXTRA_LIBRARY_PATH="$HUNK_QT_ROOT/lib:$BROWSER_CEF_SOURCE_DIR${HUNK_LINUX_EXTRA_LIBRARY_PATH:+:$HUNK_LINUX_EXTRA_LIBRARY_PATH}"

  echo "Downloading bundled Codex runtime for Linux..." >&2
  "$ROOT_DIR/scripts/download_codex_runtime_unix.sh" linux >/dev/null
  echo "Validating bundled Codex runtime for Linux..." >&2
  "$ROOT_DIR/scripts/validate_codex_runtime_bundle.sh" --strict --platform linux >/dev/null
  echo "Building Linux CEF helper..." >&2
  (
    cd "$ROOT_DIR"
    cargo build -p hunk-browser-helper --release --locked --target "$TARGET_TRIPLE" --features hunk-browser-helper/cef-subprocess
  )
  echo "Building Linux release binary..." >&2
  (
    cd "$ROOT_DIR"
    "$ROOT_DIR/scripts/build_linux.sh" --target "$TARGET_TRIPLE" --features hunk-desktop/cef-browser
  )
}

write_linux_system_wrapper() {
  local wrapper_path="$1"
  local launcher_path="$2"

  cat >"$wrapper_path" <<EOF
#!/usr/bin/env bash
set -euo pipefail
exec "$launcher_path" "\$@"
EOF
  chmod 755 "$wrapper_path"
}

write_linux_system_desktop_entry() {
  cat >"$SYSTEM_DESKTOP_ENTRY_PATH" <<EOF
[Desktop Entry]
Categories=Development;
Comment=$PACKAGE_SUMMARY
Exec=$PACKAGE_NAME
Icon=/usr/share/pixmaps/$PACKAGE_NAME.png
Name=$PRODUCT_NAME
StartupNotify=true
StartupWMClass=hunk_desktop
Terminal=false
Type=Application
EOF
}

prepare_linux_system_install_root() {
  rm -rf "$SYSTEM_INSTALL_ROOT"
  mkdir -p "$SYSTEM_BIN_DIR" "$SYSTEM_PRIVATE_LIB_DIR" "$(dirname "$SYSTEM_RUNTIME_PATH")" "$SYSTEM_ICON_DIR" "$SYSTEM_PIXMAP_DIR" "$(dirname "$SYSTEM_DESKTOP_ENTRY_PATH")"

  cp "$PACKAGED_BINARY_PATH" "$SYSTEM_REAL_BINARY_PATH"
  cp "$PACKAGED_LAUNCHER_PATH" "$SYSTEM_LAUNCHER_PATH"
  cp "$PACKAGED_BROWSER_HELPER_PATH" "$SYSTEM_LIB_DIR/hunk-browser-helper"
  cp -R "$PACKAGE_LIB_DIR/." "$SYSTEM_PRIVATE_LIB_DIR/"
  cp "$PACKAGED_QT_CONF_PATH" "$SYSTEM_QT_CONF_PATH"
  cp "$PACKAGED_CODEX_PATH" "$SYSTEM_RUNTIME_PATH"
  chmod +x "$SYSTEM_REAL_BINARY_PATH" "$SYSTEM_LAUNCHER_PATH" "$SYSTEM_LIB_DIR/hunk-browser-helper" "$SYSTEM_RUNTIME_PATH"

  patch_linux_runtime_paths "$SYSTEM_REAL_BINARY_PATH" "$SYSTEM_PRIVATE_LIB_DIR" '$ORIGIN/lib'
  patch_linux_runtime_paths "$SYSTEM_LIB_DIR/hunk-browser-helper" "$SYSTEM_PRIVATE_LIB_DIR" '$ORIGIN/lib'
  patch_linux_runtime_tree_paths "$SYSTEM_PRIVATE_LIB_DIR/qt6"
  validate_linux_runtime_bundle "$SYSTEM_REAL_BINARY_PATH" "$SYSTEM_PRIVATE_LIB_DIR"
  validate_linux_runtime_bundle "$SYSTEM_LIB_DIR/hunk-browser-helper" "$SYSTEM_PRIVATE_LIB_DIR"
  validate_linux_runtime_tree "$SYSTEM_PRIVATE_LIB_DIR/qt6" "$SYSTEM_PRIVATE_LIB_DIR"
  validate_linux_qt_runtime_layout "$SYSTEM_LIB_DIR" "$SYSTEM_PRIVATE_LIB_DIR"

  write_linux_system_wrapper "$SYSTEM_WRAPPER_PATH" "/usr/lib/$PACKAGE_NAME/$PACKAGE_NAME"
  write_linux_system_wrapper "$SYSTEM_WRAPPER_ALIAS_PATH" "/usr/lib/$PACKAGE_NAME/$PACKAGE_NAME"
  write_linux_system_desktop_entry

  cp "$LINUX_ICON_SOURCE_PATH" "$SYSTEM_ICON_PATH"
  cp "$LINUX_ICON_SOURCE_PATH" "$SYSTEM_ICON_ALIAS_PATH"
  cp "$LINUX_ICON_SOURCE_PATH" "$SYSTEM_PIXMAP_PATH"

  "$ROOT_DIR/scripts/validate_release_bundle_layout.sh" linux-install-root "$SYSTEM_INSTALL_ROOT"
}

linux_deb_installed_size_kib() {
  du -sk "$DEB_BUILD_ROOT" | awk '{print $1}'
}

write_linux_deb_control_file() {
  local control_path="$1"

  {
    printf 'Package: %s\n' "$PACKAGE_NAME"
    printf 'Version: %s\n' "$DEB_VERSION"
    printf 'Section: %s\n' "$PACKAGE_SECTION"
    printf 'Priority: %s\n' "$PACKAGE_PRIORITY"
    printf 'Architecture: %s\n' "$DEB_ARCH"
    printf 'Maintainer: %s\n' "$PACKAGE_MAINTAINER"
    printf 'Installed-Size: %s\n' "$(linux_deb_installed_size_kib)"
    if [[ -n "${HUNK_LINUX_DEB_DEPENDS:-}" ]]; then
      printf 'Depends: %s\n' "$HUNK_LINUX_DEB_DEPENDS"
    fi
    if [[ -n "$PACKAGE_HOMEPAGE" ]]; then
      printf 'Homepage: %s\n' "$PACKAGE_HOMEPAGE"
    fi
    printf 'Description: %s\n' "$PACKAGE_SUMMARY"
    printf ' %s\n' "$PACKAGE_DESCRIPTION"
  } >"$control_path"
}

build_linux_deb_package() {
  require_linux_tool dpkg-deb

  rm -rf "$DEB_BUILD_ROOT" "$DEB_PATH"
  mkdir -p "$DEB_BUILD_ROOT"
  cp -a "$SYSTEM_INSTALL_ROOT/." "$DEB_BUILD_ROOT/"
  mkdir -p "$DEB_BUILD_ROOT/DEBIAN"
  write_linux_deb_control_file "$DEB_BUILD_ROOT/DEBIAN/control"

  dpkg-deb --root-owner-group --build "$DEB_BUILD_ROOT" "$DEB_PATH" >/dev/null
  echo "Created Linux Debian package at $DEB_PATH" >&2
}

write_linux_rpm_spec() {
  local spec_path="$1"

  {
    printf '%%global _build_id_links none\n'
    # The bundled release binary and vendored private shared libraries are
    # already packaged as final ELF artifacts. Fedora/Red Hat's brp-strip
    # step can fail on patched vendored libraries such as libghostty-vt, so
    # skip that post-processing for this private app bundle.
    printf '%%global __brp_strip %%{nil}\n'
    printf 'Name:           %s\n' "$PACKAGE_NAME"
    printf 'Version:        %s\n' "$RPM_VERSION"
    printf 'Release:        %s\n' "$PACKAGE_RELEASE"
    printf 'Summary:        %s\n' "$PACKAGE_SUMMARY"
    printf 'License:        %s\n' "$PACKAGE_LICENSE"
    printf 'Packager:       %s\n' "$PACKAGE_MAINTAINER"
    if [[ -n "$PACKAGE_HOMEPAGE" ]]; then
      printf 'URL:            %s\n' "$PACKAGE_HOMEPAGE"
    fi
    printf 'BuildArch:      %s\n' "$RPM_ARCH"
    printf '\n'
    printf '%%description\n'
    printf '%s\n' "$PACKAGE_DESCRIPTION"
    printf '\n'
    printf '%%prep\n'
    printf '\n'
    printf '%%build\n'
    printf '\n'
    printf '%%install\n'
    printf 'rm -rf %%{buildroot}\n'
    printf 'mkdir -p %%{buildroot}\n'
    printf 'cp -a %%{_hunk_install_root}/. %%{buildroot}/\n'
    printf '\n'
    printf '%%files\n'
    printf '/usr/bin/%s\n' "$PACKAGE_NAME"
    printf '/usr/bin/%s\n' "${PACKAGE_NAME//-/_}"
    printf '/usr/lib/%s\n' "$PACKAGE_NAME"
    printf '/usr/share/applications/%s.desktop\n' "$PACKAGE_NAME"
    printf '/usr/share/icons/hicolor/512x512/apps/%s.png\n' "$PACKAGE_NAME"
    printf '/usr/share/icons/hicolor/512x512/apps/%s.png\n' "${PACKAGE_NAME//-/_}"
    printf '/usr/share/pixmaps/%s.png\n' "$PACKAGE_NAME"
    printf '\n'
    printf '%%changelog\n'
    printf '* %s %s - %s-%s\n' "$(linux_rpm_changelog_date)" "$PACKAGE_MAINTAINER" "$RPM_VERSION" "$PACKAGE_RELEASE"
    printf '%s\n' '- Package release build.'
  } >"$spec_path"
}

build_linux_rpm_package() {
  require_linux_tool rpmbuild

  rm -rf "$RPM_TOPDIR" "$RPM_PATH"
  mkdir -p "$RPM_TOPDIR/BUILD" "$RPM_TOPDIR/BUILDROOT" "$RPM_TOPDIR/RPMS" "$RPM_TOPDIR/SOURCES" "$RPM_TOPDIR/SPECS" "$RPM_TOPDIR/SRPMS"

  local spec_path="$RPM_TOPDIR/SPECS/$PACKAGE_NAME.spec"
  write_linux_rpm_spec "$spec_path"

  rpmbuild \
    --define "_topdir $RPM_TOPDIR" \
    --define "_hunk_install_root $SYSTEM_INSTALL_ROOT" \
    --nodebuginfo \
    -bb "$spec_path" >/dev/null

  local built_rpm
  built_rpm="$(find "$RPM_TOPDIR/RPMS" -type f -name "*.rpm" | sort | head -n 1)"
  if [[ -z "$built_rpm" ]]; then
    echo "error: rpmbuild did not produce an RPM under $RPM_TOPDIR/RPMS" >&2
    exit 1
  fi

  cp "$built_rpm" "$RPM_PATH"
  echo "Created Linux RPM package at $RPM_PATH" >&2
}
