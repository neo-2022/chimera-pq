#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "chimera_update_contract_smoke: $1" >&2
  exit 1
}

source "$ROOT_DIR/scripts/chimera-sh"

case_update_sources_unreachable_continues() (
  read_local_runtime_version() { printf '%s\n' 0.1.0; }
  read_local_runtime_bundle_sha() { printf '%s\n' deadbeef; }
  load_update_peer_bootstrap_urls() { return 0; }
  try_update_from_bootstrap_source() { return 2; }

  local output rc
  output="$(auto_update_if_needed -start 2>&1)"
  rc=$?
  [[ "$rc" -eq 0 ]] || fail "expected rc=0 for update-source outage, got $rc"
  [[ "$output" == *"chimera_update=unavailable"* ]] || fail "missing unavailable diagnostic"
)

case_update_required_install_failure_blocks() (
  read_local_runtime_version() { printf '%s\n' 0.1.0; }
  read_local_runtime_bundle_sha() { printf '%s\n' deadbeef; }
  load_update_peer_bootstrap_urls() { return 0; }
  try_update_from_bootstrap_source() { return 3; }

  local output rc
  output="$(auto_update_if_needed -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  [[ "$rc" -ne 0 ]] || fail "expected non-zero rc when update is required but install fails"
  [[ "$output" != *"chimera_update=unavailable"* ]] || fail "unexpected outage diagnostic for real update failure"
)

case_update_sources_unreachable_continues
case_update_required_install_failure_blocks

echo "chimera_update_contract_smoke=pass"
