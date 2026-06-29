#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

side_a_env="configs/mesh_launch_preflight.side_a.env"
side_b_env="configs/mesh_launch_preflight.side_b.env"

if [[ ! -f "$side_a_env" || ! -f "$side_b_env" ]]; then
  echo "mesh launch preflight ready hint: missing env files"
  echo "required: $side_a_env and $side_b_env"
  exit 1
fi

run_check() {
  local label="$1"
  shift
  if "$@" >/tmp/chimera_${label}.log 2>&1; then
    echo "[ok] $label"
    return 0
  fi
  echo "[fail] $label"
  sed -n '1,3p' /tmp/chimera_${label}.log
  return 1
}

status=0
run_check env_guard_side_a bash scripts/mesh_launch_preflight_env_guard.sh "$side_a_env" || status=1
run_check env_guard_side_b bash scripts/mesh_launch_preflight_env_guard.sh "$side_b_env" || status=1
run_check env_pair_guard bash scripts/mesh_launch_preflight_env_pair_guard.sh "$side_a_env" "$side_b_env" || status=1
run_check endpoint_probe_side_a bash scripts/mesh_launch_preflight_endpoint_probe.sh "$side_a_env" || status=1
run_check endpoint_probe_side_b bash scripts/mesh_launch_preflight_endpoint_probe.sh "$side_b_env" || status=1

if (( status == 0 )); then
  echo "mesh launch preflight ready hint: READY"
  echo "next: just mesh-launch-preflight-autopilot"
  exit 0
fi

echo "hint: run just mesh-launch-preflight-autopilot"
echo "hint: auto-bind resolves current endpoints from local signed discovery snapshot, published runtime state, then inventory/runtime state"
echo "hint: manual <host:port> helpers are fallback only after automatic sources are unavailable"
echo "hint: if that still fails, check CHIMERA_MESH_DISCOVERY_SNAPSHOT, CHIMERA_MESH_NODES_CONFIG and the side_a/side_b env files"

echo "mesh launch preflight ready hint: NOT READY"
exit 1
