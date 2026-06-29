#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MODE="${1:-staged}"
PROFILE_SET="${2:-core}"
SIDE_A_ENDPOINT="${3:-${CHIMERA_MESH_SIDE_A_ENDPOINT:-}}"

case "$MODE" in
  staged|full) ;;
  *)
    echo "mesh launch preflight autopilot: mode must be staged or full"
    echo "usage: mesh_launch_preflight_autopilot.sh [staged|full] [core|all] [side_a_host:port]"
    exit 2
    ;;
esac

case "$PROFILE_SET" in
  core|all) ;;
  *)
    echo "mesh launch preflight autopilot: profile set must be core or all"
    echo "usage: mesh_launch_preflight_autopilot.sh [staged|full] [core|all] [side_a_host:port]"
    exit 2
    ;;
esac

profiles=("high_speed_anonymous" "privacy_first")
if [[ "$PROFILE_SET" == "all" ]]; then
  profiles+=("speed_first" "low_latency_private")
fi

if [[ -n "$SIDE_A_ENDPOINT" ]]; then
  echo "mesh launch preflight autopilot: mode=$MODE profile_set=$PROFILE_SET manual_side_a_endpoint=fallback"
else
  echo "mesh launch preflight autopilot: mode=$MODE profile_set=$PROFILE_SET auto_bind=discovery_runtime_inventory"
fi

if [[ -n "$SIDE_A_ENDPOINT" ]]; then
  just mesh-launch-preflight-auto-bind "$SIDE_A_ENDPOINT"
else
  just mesh-launch-preflight-auto-bind
fi
just mesh-launch-preflight-ready-check

if [[ "$MODE" == "staged" ]]; then
  for profile in "${profiles[@]}"; do
    echo "mesh launch preflight autopilot: staged profile=$profile side_a"
    just mesh-launch-preflight-side-a-profile-staged "$profile"
    echo "mesh launch preflight autopilot: staged profile=$profile side_b"
    just mesh-launch-preflight-side-b-profile-staged "$profile"
  done
else
  for profile in "${profiles[@]}"; do
    echo "mesh launch preflight autopilot: full profile=$profile side_a"
    just mesh-launch-preflight-side-a-profile "$profile"
    echo "mesh launch preflight autopilot: full profile=$profile side_b"
    just mesh-launch-preflight-side-b-profile "$profile"
  done
  just mesh-launch-preflight-evidence-guard
fi

just mesh-launch-preflight-status-summary
echo "mesh launch preflight autopilot: PASS"
