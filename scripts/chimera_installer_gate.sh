#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "installer_gate=fail reason=$1" >&2
  exit 1
}

rg -n "installer_gate_prepare_upstream_env|transparent runtime|transparent runtime" \
  "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_missing_transparent_bootstrap"

rg -n "proxy-status|transparent_runtime|split-transparent" \
  "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_missing_transparent_runtime"

echo "installer_gate=pass"
