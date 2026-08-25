#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"

"$script_dir/verify_qt.sh"

export QT_QPA_PLATFORM="offscreen"
mkdir -p "$repo_root/target"
cd "$repo_root"

qml_module_dir="$repo_root/crates/hunk-qt/src/qml/Hunk"
qml_sources=()
for qml_source in "$qml_module_dir"/*.qml; do
  if [[ "$qml_source" != "$qml_module_dir/Main.qml" ]]; then
    qml_sources+=("$qml_source")
  fi
done
qmllint -I "$repo_root/crates/hunk-qt/src/qml" "${qml_sources[@]}"

qmltestrunner \
  -input "$repo_root/crates/hunk-qt/tests/qml" \
  -import "$repo_root/crates/hunk-qt/src/qml"
