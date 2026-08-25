#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  cat <<'EOF'
Download/export and validate the CEF runtime used by Hunk's embedded browser.

Usage:
  ./scripts/prepare_browser_cef_runtime.sh <target-triple> [runtime-dir]

Environment:
  HUNK_CEF_RS_REPO       cef-rs repository URL.
  HUNK_CEF_RS_REV        cef-rs commit/rev to use.
  HUNK_CEF_RS_DIR        checkout directory for cef-rs.
  HUNK_CEF_FORCE_EXPORT  set to 1 to re-export even if runtime validation passes.
EOF
}

if [[ $# -lt 1 || $# -gt 2 ]]; then
  usage >&2
  exit 1
fi

TARGET_TRIPLE="$1"
case "$TARGET_TRIPLE" in
  *-apple-darwin)
    PLATFORM="macos"
    DEFAULT_RUNTIME_DIR="$ROOT_DIR/assets/browser-runtime/cef/macos/runtime"
    VALIDATOR=("$ROOT_DIR/scripts/validate_browser_cef_macos.sh")
    ;;
  *-unknown-linux-gnu)
    PLATFORM="linux"
    DEFAULT_RUNTIME_DIR="$ROOT_DIR/assets/browser-runtime/cef/linux/runtime"
    VALIDATOR=("$ROOT_DIR/scripts/validate_browser_cef_linux.sh")
    ;;
  *)
    echo "error: unsupported CEF target for this script: $TARGET_TRIPLE" >&2
    usage >&2
    exit 1
    ;;
esac

RUNTIME_DIR="${2:-$DEFAULT_RUNTIME_DIR}"
if [[ "$RUNTIME_DIR" != /* ]]; then
  RUNTIME_DIR="$ROOT_DIR/$RUNTIME_DIR"
fi
CEF_RS_REPO="${HUNK_CEF_RS_REPO:-https://github.com/tauri-apps/cef-rs.git}"
CEF_RS_REV="${HUNK_CEF_RS_REV:-a2e15ae659c4b3957883e34de879bd8b38360ce5}"
DEFAULT_CEF_RS_DIR="${TMPDIR:-/tmp}/hunk-cef-rs"
if [[ -n "${XDG_CACHE_HOME:-}" ]]; then
  DEFAULT_CEF_RS_DIR="$XDG_CACHE_HOME/hunk/cef-rs"
fi
CEF_RS_DIR="${HUNK_CEF_RS_DIR:-$DEFAULT_CEF_RS_DIR}"
FORCE_EXPORT="${HUNK_CEF_FORCE_EXPORT:-0}"

validate_runtime() {
  "${VALIDATOR[@]}" "$RUNTIME_DIR" >/dev/null
}

validate_runtime_quietly() {
  "${VALIDATOR[@]}" "$RUNTIME_DIR" >/dev/null 2>&1
}

if [[ "$FORCE_EXPORT" != "1" ]] && validate_runtime_quietly; then
  echo "Using existing $PLATFORM CEF runtime at $RUNTIME_DIR" >&2
  exit 0
fi

if ! command -v git >/dev/null 2>&1; then
  echo "error: git is required to fetch cef-rs" >&2
  exit 1
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo is required to run cef-rs export-cef-dir" >&2
  exit 1
fi

if [[ -e "$CEF_RS_DIR" && ! -d "$CEF_RS_DIR/.git" ]]; then
  echo "error: refusing to replace non-checkout CEF path: $CEF_RS_DIR" >&2
  exit 1
fi
if [[ ! -d "$CEF_RS_DIR/.git" ]]; then
  mkdir -p "$(dirname "$CEF_RS_DIR")"
  git clone --depth=1 "$CEF_RS_REPO" "$CEF_RS_DIR"
fi

git -C "$CEF_RS_DIR" fetch --depth=1 origin "$CEF_RS_REV"
git -C "$CEF_RS_DIR" checkout --detach "$CEF_RS_REV" >/dev/null

mkdir -p "$(dirname "$RUNTIME_DIR")"
echo "Exporting $PLATFORM CEF runtime for $TARGET_TRIPLE to $RUNTIME_DIR" >&2
(
  cd "$CEF_RS_DIR"
  cargo run -p export-cef-dir -- --force --target "$TARGET_TRIPLE" "$RUNTIME_DIR"
)

validate_runtime
echo "CEF runtime ready at $RUNTIME_DIR" >&2
