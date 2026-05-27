#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "installer_gate=fail reason=$1" >&2
  exit 1
}

rg -n "CHIMERA_SOCKS_PORT|installer_gate_unify_socks_unit|chimera-socks-tunnel.service" \
  "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_missing_port_unify_logic"

grep -F -- '-D 127.0.0.1:${CHIMERA_SOCKS_PORT:-12080}' \
  "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "legacy_unit_not_parametrized"

rg -n "CHIMERA_SOCKS_PORT" "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_missing_socks_port_env"
rg -n "CHIMERA_SOCKS_PORT" "$ROOT_DIR/scripts/chimera-socks-watchdog.sh" >/dev/null || fail "watchdog_missing_socks_port_env"

echo "installer_gate=pass"
