#!/usr/bin/env bash
set -euo pipefail

qt_version="6.11.2"
aqt_version="3.3.0"
cache_root="${HUNK_QT_CACHE_ROOT:-${XDG_CACHE_HOME:-$HOME/.cache}/hunk/qt}"
tool_root="$cache_root/tools/aqt-$aqt_version"
install_wayland="${HUNK_QT_INSTALL_WAYLAND:-0}"

case "$(uname -s)" in
  Darwin)
    host="mac"
    arch="clang_64"
    output_root="$cache_root/$qt_version/macos"
    qt_root="$output_root/$qt_version/macos"
    ;;
  Linux)
    host="linux"
    arch="linux_gcc_64"
    output_root="$cache_root/$qt_version/linux"
    qt_root="$output_root/$qt_version/gcc_64"
    ;;
  *)
    echo "Use the pinned CI installer for Qt $qt_version on Windows." >&2
    exit 1
    ;;
esac

qt_is_ready=0
if [ -x "$qt_root/bin/qmake" ] && [ "$("$qt_root/bin/qmake" -query QT_VERSION)" = "$qt_version" ]; then
  qt_is_ready=1
fi

if [ "$install_wayland" = "1" ] && {
  [ ! -f "$qt_root/plugins/platforms/libqwayland-egl.so" ] \
    || [ ! -f "$qt_root/plugins/platforms/libqwayland-generic.so" ];
}; then
  qt_is_ready=0
fi

if [ "$qt_is_ready" = "1" ]; then
  echo "$qt_root"
  exit 0
fi

mkdir -p "$cache_root" "$cache_root/tmp" "$(dirname "$tool_root")"
export TMPDIR="${HUNK_QT_TMPDIR:-$cache_root/tmp}"

if [ ! -x "$tool_root/bin/python" ]; then
  python3 -m virtualenv "$tool_root"
fi

"$tool_root/bin/python" -m pip install --disable-pip-version-check "aqtinstall==$aqt_version"
cd "$cache_root"
install_args=(install-qt "$host" desktop "$qt_version" "$arch" --outputdir "$output_root")
if [ "$install_wayland" = "1" ]; then
  if [ "$host" != "linux" ]; then
    echo "HUNK_QT_INSTALL_WAYLAND is only supported by the Linux Qt installer." >&2
    exit 1
  fi
  install_args+=(--modules qtwaylandcompositor)
fi
"$tool_root/bin/python" -m aqt "${install_args[@]}"

actual_version="$("$qt_root/bin/qmake" -query QT_VERSION)"
if [ "$actual_version" != "$qt_version" ]; then
  echo "Expected Qt $qt_version after installation, found $actual_version" >&2
  exit 1
fi

if [ "$install_wayland" = "1" ] && {
  [ ! -f "$qt_root/plugins/platforms/libqwayland-egl.so" ] \
    || [ ! -f "$qt_root/plugins/platforms/libqwayland-generic.so" ];
}; then
  echo "Expected Qt $qt_version Wayland platform plugins after installation." >&2
  exit 1
fi

echo "$qt_root"
