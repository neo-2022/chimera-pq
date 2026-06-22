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
  echo "next: just mesh-launch-preflight-side-a && just mesh-launch-preflight-side-b && just mesh-launch-preflight-evidence-guard"
  exit 0
fi

# Context hint for the most common blocker.
set -a
# shellcheck disable=SC1090
source "$side_a_env"
set +a
side_a_endpoint="${CHIMERA_MESH_REMOTE_ENDPOINT:-}"

set -a
# shellcheck disable=SC1090
source "$side_b_env"
set +a
side_b_endpoint="${CHIMERA_MESH_REMOTE_ENDPOINT:-}"

if [[ "$side_a_endpoint" =~ ^198\.51\.100\. || "$side_a_endpoint" =~ ^203\.0\.113\. || "$side_a_endpoint" =~ ^192\.0\.2\. ]]; then
  echo "hint: side_a uses a documentation placeholder endpoint: $side_a_endpoint"
  echo "hint: replace CHIMERA_MESH_REMOTE_ENDPOINT in $side_a_env with real side_b host:port"
fi
if [[ "$side_b_endpoint" =~ ^198\.51\.100\. || "$side_b_endpoint" =~ ^203\.0\.113\. || "$side_b_endpoint" =~ ^192\.0\.2\. ]]; then
  echo "hint: side_b uses a documentation placeholder endpoint: $side_b_endpoint"
  echo "hint: replace CHIMERA_MESH_REMOTE_ENDPOINT in $side_b_env with real side_a host:port"
fi

echo "mesh launch preflight ready hint: NOT READY"
exit 1
