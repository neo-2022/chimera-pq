#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

profiles=(
  high_speed_anonymous
  privacy_first
  speed_first
  low_latency_private
)

VPS_ENV="configs/mesh_launch_preflight.vps.env.example"
LAPTOP_ENV="configs/mesh_launch_preflight.laptop.env.example"

if [[ ! -f "$VPS_ENV" ]]; then
  echo "mesh launch preflight profile smoke: missing $VPS_ENV"
  exit 1
fi
if [[ ! -f "$LAPTOP_ENV" ]]; then
  echo "mesh launch preflight profile smoke: missing $LAPTOP_ENV"
  exit 1
fi

cleanup_profile_artifacts() {
  local profile="$1"
  rm -f \
    "/tmp/chimera_mesh_launch_preflight_${profile}_side_a_local.json" \
    "/tmp/chimera_mesh_launch_preflight_${profile}_side_a_remote.json" \
    "/tmp/chimera_mesh_launch_preflight_${profile}_side_b_local.json" \
    "/tmp/chimera_mesh_launch_preflight_${profile}_side_b_remote.json" \
    "/tmp/chimera_mesh_launch_preflight_${profile}_verify.json"
}

for profile in "${profiles[@]}"; do
  cleanup_profile_artifacts "$profile"

  set -a
  # shellcheck disable=SC1090
  source "$VPS_ENV"
  CHIMERA_MESH_LOCAL_OUT="/tmp/chimera_mesh_launch_preflight_${profile}_side_a_local.json"
  CHIMERA_MESH_REMOTE_OUT="/tmp/chimera_mesh_launch_preflight_${profile}_side_a_remote.json"
  CHIMERA_MESH_VERIFY_OUT="/tmp/chimera_mesh_launch_preflight_${profile}_verify.json"
  CHIMERA_MESH_ALLOW_REMOTE_MISSING=1
  CHIMERA_MESH_TRAFFIC_PROFILE="$profile"
  unset CHIMERA_MESH_POLICY_PAYLOAD
  set +a
  side_a_rc=0
  if ! bash scripts/mesh_launch_preflight_pair.sh; then
    side_a_rc=$?
  fi
  if [[ "$side_a_rc" != "0" && "$side_a_rc" != "1" ]]; then
    echo "mesh launch preflight profile smoke: unexpected side_a rc=$side_a_rc for $profile"
    exit 1
  fi

  set -a
  # shellcheck disable=SC1090
  source "$LAPTOP_ENV"
  CHIMERA_MESH_LOCAL_OUT="/tmp/chimera_mesh_launch_preflight_${profile}_side_b_local.json"
  CHIMERA_MESH_REMOTE_OUT="/tmp/chimera_mesh_launch_preflight_${profile}_side_b_remote.json"
  CHIMERA_MESH_VERIFY_OUT="/tmp/chimera_mesh_launch_preflight_${profile}_verify.json"
  CHIMERA_MESH_ALLOW_REMOTE_MISSING=1
  CHIMERA_MESH_TRAFFIC_PROFILE="$profile"
  unset CHIMERA_MESH_POLICY_PAYLOAD
  set +a
  side_b_rc=0
  if ! bash scripts/mesh_launch_preflight_pair.sh; then
    side_b_rc=$?
  fi
  if [[ "$side_b_rc" != "0" && "$side_b_rc" != "1" ]]; then
    echo "mesh launch preflight profile smoke: unexpected side_b rc=$side_b_rc for $profile"
    exit 1
  fi

  if [[ ! -f "/tmp/chimera_mesh_launch_preflight_${profile}_side_a_local.json" ]]; then
    echo "mesh launch preflight profile smoke: missing side_a artifact for $profile"
    exit 1
  fi
  if [[ ! -f "/tmp/chimera_mesh_launch_preflight_${profile}_side_b_local.json" ]]; then
    echo "mesh launch preflight profile smoke: missing side_b artifact for $profile"
    exit 1
  fi

  cleanup_profile_artifacts "$profile"
done

echo "mesh launch preflight profile smoke: PASS"
