#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

vps_env="configs/mesh_launch_preflight.vps.env"
laptop_env="configs/mesh_launch_preflight.laptop.env"

if [[ ! -f "$vps_env" || ! -f "$laptop_env" ]]; then
  echo "mesh launch preflight ready hint: missing env files"
  echo "required: $vps_env and $laptop_env"
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
run_check env_guard_side_a bash scripts/mesh_launch_preflight_env_guard.sh "$vps_env" || status=1
run_check env_guard_side_b bash scripts/mesh_launch_preflight_env_guard.sh "$laptop_env" || status=1
run_check env_pair_guard bash scripts/mesh_launch_preflight_env_pair_guard.sh "$vps_env" "$laptop_env" || status=1
run_check endpoint_probe_side_a bash scripts/mesh_launch_preflight_endpoint_probe.sh "$vps_env" || status=1
run_check endpoint_probe_side_b bash scripts/mesh_launch_preflight_endpoint_probe.sh "$laptop_env" || status=1

if (( status == 0 )); then
  echo "mesh launch preflight ready hint: READY"
  echo "next: just mesh-launch-preflight-side-a && just mesh-launch-preflight-side-b && just mesh-launch-preflight-evidence-guard"
  exit 0
fi

# Context hint for the most common blocker.
set -a
# shellcheck disable=SC1090
source "$vps_env"
set +a
vps_endpoint="${CHIMERA_MESH_REMOTE_ENDPOINT:-}"

set -a
# shellcheck disable=SC1090
source "$laptop_env"
set +a
laptop_endpoint="${CHIMERA_MESH_REMOTE_ENDPOINT:-}"

if [[ "$vps_endpoint" =~ ^198\.51\.100\. || "$vps_endpoint" =~ ^203\.0\.113\. || "$vps_endpoint" =~ ^192\.0\.2\. ]]; then
  echo "hint: side_a uses a documentation placeholder endpoint: $vps_endpoint"
  echo "hint: replace CHIMERA_MESH_REMOTE_ENDPOINT in $vps_env with real laptop host:port"
fi
if [[ "$laptop_endpoint" =~ ^198\.51\.100\. || "$laptop_endpoint" =~ ^203\.0\.113\. || "$laptop_endpoint" =~ ^192\.0\.2\. ]]; then
  echo "hint: side_b uses a documentation placeholder endpoint: $laptop_endpoint"
  echo "hint: replace CHIMERA_MESH_REMOTE_ENDPOINT in $laptop_env with real vps host:port"
fi

echo "mesh launch preflight ready hint: NOT READY"
exit 1
