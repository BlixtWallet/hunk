#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_ID="io.github.BlixtWallet.Hunk"
TARGET_DIR="$("$ROOT_DIR/scripts/resolve_cargo_target_dir.sh" "$ROOT_DIR")"
REPO_DIR="$TARGET_DIR/flatpak/repo"
REMOTE_NAME="hunk-local"

"$ROOT_DIR/scripts/build_flatpak.sh"

flatpak --user remote-add --if-not-exists --no-gpg-verify "$REMOTE_NAME" "$REPO_DIR" >/dev/null
flatpak --user install -y "$REMOTE_NAME" "$APP_ID" >/dev/null
flatpak run "$APP_ID"
