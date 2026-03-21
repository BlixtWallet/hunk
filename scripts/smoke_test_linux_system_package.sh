#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="$("$ROOT_DIR/scripts/resolve_cargo_target_dir.sh" "$ROOT_DIR")"
DIST_DIR="$TARGET_DIR/dist"

usage() {
  cat <<'EOF'
Smoke test packaged Linux system installs in a container.

Usage:
  ./scripts/smoke_test_linux_system_package.sh <deb|rpm> [package-path]
EOF
}

container_runner() {
  if command -v docker >/dev/null 2>&1; then
    printf '%s\n' docker
    return 0
  fi

  if command -v podman >/dev/null 2>&1; then
    printf '%s\n' podman
    return 0
  fi

  echo "error: docker or podman is required for Linux package smoke tests" >&2
  exit 1
}

run_deb_smoke_test() {
  local package_path="$1"
  local runner="$2"
  local package_dir
  local package_name

  package_dir="$(cd "$(dirname "$package_path")" && pwd)"
  package_name="$(basename "$package_path")"

  "$runner" run --rm \
    -v "$package_dir:/packages:ro" \
    ubuntu:24.04 \
    bash -lc "
      set -euo pipefail
      export DEBIAN_FRONTEND=noninteractive
      apt-get update
      apt-get install -y /packages/$package_name
      test -x /usr/bin/hunk-desktop
      test -x /usr/lib/hunk-desktop/hunk_desktop_bin
      ldd /usr/lib/hunk-desktop/hunk_desktop_bin | tee /tmp/hunk-ldd.txt
      ! grep -Fq 'not found' /tmp/hunk-ldd.txt
    "
}

run_rpm_smoke_test() {
  local package_path="$1"
  local runner="$2"
  local package_dir
  local package_name

  package_dir="$(cd "$(dirname "$package_path")" && pwd)"
  package_name="$(basename "$package_path")"

  "$runner" run --rm \
    -v "$package_dir:/packages:ro" \
    fedora:latest \
    bash -lc "
      set -euo pipefail
      dnf install -y /packages/$package_name
      test -x /usr/bin/hunk-desktop
      test -x /usr/lib/hunk-desktop/hunk_desktop_bin
      ldd /usr/lib/hunk-desktop/hunk_desktop_bin | tee /tmp/hunk-ldd.txt
      ! grep -Fq 'not found' /tmp/hunk-ldd.txt
    "
}

if [[ $# -lt 1 || $# -gt 2 ]]; then
  usage >&2
  exit 1
fi

package_kind="$1"
package_path="${2:-}"
runner="$(container_runner)"

case "$package_kind" in
  deb)
    if [[ -z "$package_path" ]]; then
      package_path="$(find "$DIST_DIR" -maxdepth 1 -type f -name '*.deb' | sort | head -n 1)"
    fi
    if [[ -z "$package_path" ]]; then
      echo "error: no Debian package found under $DIST_DIR" >&2
      echo "hint: run ./scripts/package_linux_release.sh --formats deb first" >&2
      exit 1
    fi
    run_deb_smoke_test "$package_path" "$runner"
    ;;
  rpm)
    if [[ -z "$package_path" ]]; then
      package_path="$(find "$DIST_DIR" -maxdepth 1 -type f -name '*.rpm' | sort | head -n 1)"
    fi
    if [[ -z "$package_path" ]]; then
      echo "error: no RPM package found under $DIST_DIR" >&2
      echo "hint: run ./scripts/package_linux_release.sh --formats rpm first" >&2
      exit 1
    fi
    run_rpm_smoke_test "$package_path" "$runner"
    ;;
  *)
    echo "error: unsupported package kind '$package_kind'" >&2
    usage >&2
    exit 1
    ;;
esac
