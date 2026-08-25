#!/usr/bin/env bash

# Source this file to expose Hunk's exact Qt SDK to Cargo and the QML tools.
# It intentionally does not download anything; use install_qt.sh once when the
# shared cache does not already contain the pinned SDK.

hunk_qt_version="6.11.2"
hunk_qt_cache_root="${HUNK_QT_CACHE_ROOT:-}"

if [ -z "$hunk_qt_cache_root" ] && [ -d "/Volumes/hulk/dev/cache/qt" ]; then
  hunk_qt_cache_root="/Volumes/hulk/dev/cache/qt"
fi

if [ -z "$hunk_qt_cache_root" ] && [ -n "${XDG_CACHE_HOME:-}" ]; then
  hunk_qt_cache_root="$XDG_CACHE_HOME/hunk/qt"
fi

if [ -n "${HUNK_QT_ROOT:-}" ]; then
  hunk_qt_root="$HUNK_QT_ROOT"
elif [ -n "${QT_ROOT_DIR:-}" ]; then
  hunk_qt_root="$QT_ROOT_DIR"
elif [ "$(uname -s)" = "Darwin" ] && [ -n "$hunk_qt_cache_root" ]; then
  hunk_qt_root="$hunk_qt_cache_root/$hunk_qt_version/macos/$hunk_qt_version/macos"
elif [ "$(uname -s)" = "Linux" ] && [ -n "$hunk_qt_cache_root" ]; then
  hunk_qt_root="$hunk_qt_cache_root/$hunk_qt_version/linux/$hunk_qt_version/gcc_64"
else
  hunk_qt_root=""
fi

if [ -x "$hunk_qt_root/bin/qmake" ]; then
  hunk_qt_actual_version="$("$hunk_qt_root/bin/qmake" -query QT_VERSION)"
  if [ "$hunk_qt_actual_version" != "$hunk_qt_version" ]; then
    echo "Hunk requires Qt $hunk_qt_version, found $hunk_qt_actual_version at $hunk_qt_root" >&2
    return 1 2>/dev/null || exit 1
  fi

  export HUNK_QT_ROOT="$hunk_qt_root"
  export QMAKE="$hunk_qt_root/bin/qmake"
  export PATH="$hunk_qt_root/bin:$PATH"
  export CMAKE_PREFIX_PATH="$hunk_qt_root${CMAKE_PREFIX_PATH:+:$CMAKE_PREFIX_PATH}"
  export QML_IMPORT_PATH="$hunk_qt_root/qml${QML_IMPORT_PATH:+:$QML_IMPORT_PATH}"
  export QT_PLUGIN_PATH="$hunk_qt_root/plugins${QT_PLUGIN_PATH:+:$QT_PLUGIN_PATH}"

  if [ "$(uname -s)" = "Darwin" ]; then
    export CXXFLAGS="-F$hunk_qt_root/lib${CXXFLAGS:+ $CXXFLAGS}"
    export LDFLAGS="-F$hunk_qt_root/lib${LDFLAGS:+ $LDFLAGS}"
    export DYLD_FRAMEWORK_PATH="$hunk_qt_root/lib${DYLD_FRAMEWORK_PATH:+:$DYLD_FRAMEWORK_PATH}"
  elif [ "$(uname -s)" = "Linux" ]; then
    export LD_LIBRARY_PATH="$hunk_qt_root/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  fi
fi

unset hunk_qt_actual_version hunk_qt_cache_root hunk_qt_root hunk_qt_version
