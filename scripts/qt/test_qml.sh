#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"

"$script_dir/verify_qt.sh"

export QT_QPA_PLATFORM="offscreen"
mkdir -p "$repo_root/target"
cd "$repo_root"
qmltestrunner \
  -input "$repo_root/crates/hunk-qt/tests/qml" \
  -import "$repo_root/crates/hunk-qt/src/qml"
