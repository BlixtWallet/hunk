#!/usr/bin/env bash
set -euo pipefail

required_version="6.11.2"
qmake_command="${QMAKE:-qmake}"

if ! command -v "$qmake_command" >/dev/null 2>&1; then
  echo "qmake was not found; install the pinned Qt SDK with scripts/qt/install_qt.sh" >&2
  exit 1
fi

actual_version="$("$qmake_command" -query QT_VERSION)"
if [ "$actual_version" != "$required_version" ]; then
  echo "Hunk requires Qt $required_version, found $actual_version" >&2
  exit 1
fi

echo "Qt $actual_version: $(command -v "$qmake_command")"
