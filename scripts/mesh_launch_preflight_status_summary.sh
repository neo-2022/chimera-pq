#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

side_a_env="configs/mesh_launch_preflight.side_a.env"
side_b_env="configs/mesh_launch_preflight.side_b.env"

if [[ ! -f "$side_a_env" || ! -f "$side_b_env" ]]; then
  echo "mesh launch preflight status summary: missing env files"
  echo "required: $side_a_env and $side_b_env"
  exit 1
fi

extract_value() {
  local key="$1"
  local file="$2"
  awk -F= -v k="$key" '$1==k{print substr($0, index($0,$2)); exit}' "$file"
}

side_a_endpoint="$(extract_value CHIMERA_MESH_REMOTE_ENDPOINT "$side_a_env")"
side_b_endpoint="$(extract_value CHIMERA_MESH_REMOTE_ENDPOINT "$side_b_env")"

echo "mesh launch preflight status summary"
echo "- side_a remote endpoint: ${side_a_endpoint:-<missing>}"
echo "- side_b remote endpoint: ${side_b_endpoint:-<missing>}"

if [[ -f docs/MESH_LAUNCH_PREFLIGHT_SIDE_A.json ]]; then
  side_a_status="$(jq -r '.status // "unknown"' docs/MESH_LAUNCH_PREFLIGHT_SIDE_A.json)"
  side_a_ready="$(jq -r '.ready_for_real_launch // false' docs/MESH_LAUNCH_PREFLIGHT_SIDE_A.json)"
  echo "- side_a artifact: status=${side_a_status}, ready_for_real_launch=${side_a_ready}"
else
  echo "- side_a artifact: missing"
fi

if [[ -f docs/MESH_LAUNCH_PREFLIGHT_SIDE_B.json ]]; then
  side_b_status="$(jq -r '.status // "unknown"' docs/MESH_LAUNCH_PREFLIGHT_SIDE_B.json)"
  side_b_ready="$(jq -r '.ready_for_real_launch // false' docs/MESH_LAUNCH_PREFLIGHT_SIDE_B.json)"
  echo "- side_b artifact: status=${side_b_status}, ready_for_real_launch=${side_b_ready}"
else
  echo "- side_b artifact: missing"
fi

if [[ -f docs/MESH_LAUNCH_PREFLIGHT_VERIFY.json ]]; then
  verify_status="$(jq -r '.status // "unknown"' docs/MESH_LAUNCH_PREFLIGHT_VERIFY.json)"
  all_ready="$(jq -r '.all_ready // false' docs/MESH_LAUNCH_PREFLIGHT_VERIFY.json)"
  blockers="$(jq -r '(.blockers // []) | join(",")' docs/MESH_LAUNCH_PREFLIGHT_VERIFY.json)"
  if [[ -z "$blockers" ]]; then
    blockers="none"
  fi
  echo "- verify artifact: status=${verify_status}, all_ready=${all_ready}, blockers=${blockers}"
else
  echo "- verify artifact: missing"
fi

if bash scripts/mesh_launch_preflight_ready_hint.sh >/tmp/chimera_ready_hint_summary.log 2>&1; then
  echo "- readiness gate: READY"
else
  echo "- readiness gate: NOT READY"
  sed -n '1,6p' /tmp/chimera_ready_hint_summary.log | sed 's/^/  /'
fi
