#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_TRIPLE="${HUNK_WINDOWS_TARGET:-x86_64-pc-windows-msvc}"
PROFILE="release"
STAGE_RUNTIME=1

usage() {
  cat <<'EOF'
Build hunk-desktop for Windows.

Usage:
  ./scripts/build_windows.sh [--target <triple>] [--debug] [--no-stage-runtime]

Options:
  --target <triple>   Override target triple (default: x86_64-pc-windows-msvc)
  --debug             Build debug profile instead of release
  --no-stage-runtime  Skip staging assets/codex-runtime/windows/codex.exe
  -h, --help          Show this help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      TARGET_TRIPLE="${2:-}"
      if [[ -z "$TARGET_TRIPLE" ]]; then
        echo "error: --target requires a value" >&2
        exit 1
      fi
      shift 2
      ;;
    --debug)
      PROFILE="debug"
      shift
      ;;
    --no-stage-runtime)
      STAGE_RUNTIME=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument '$1'" >&2
      usage >&2
      exit 1
      ;;
  esac
done

build_args=(build -p hunk-desktop --target "$TARGET_TRIPLE")
if [[ "$PROFILE" == "release" ]]; then
  build_args+=(--release)
fi

echo "Building hunk-desktop for Windows target '$TARGET_TRIPLE' ($PROFILE profile)..."
(
  cd "$ROOT_DIR"
  cargo "${build_args[@]}"
)

BINARY_PATH="$ROOT_DIR/target/$TARGET_TRIPLE/$PROFILE/hunk-desktop.exe"
echo "Built binary: $BINARY_PATH"

if [[ "$STAGE_RUNTIME" == "1" ]]; then
  SOURCE_RUNTIME="$ROOT_DIR/assets/codex-runtime/windows/codex.exe"
  DEST_RUNTIME="$ROOT_DIR/target/$TARGET_TRIPLE/$PROFILE/codex-runtime/windows/codex.exe"

  if [[ ! -f "$SOURCE_RUNTIME" ]]; then
    echo "warn: windows runtime asset not found at $SOURCE_RUNTIME; skipping runtime staging" >&2
  else
    mkdir -p "$(dirname "$DEST_RUNTIME")"
    cp "$SOURCE_RUNTIME" "$DEST_RUNTIME"
    echo "Staged Windows runtime: $DEST_RUNTIME"
  fi
fi

