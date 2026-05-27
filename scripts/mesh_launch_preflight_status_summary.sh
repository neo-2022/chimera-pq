#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

vps_env="configs/mesh_launch_preflight.vps.env"
laptop_env="configs/mesh_launch_preflight.laptop.env"

if [[ ! -f "$vps_env" || ! -f "$laptop_env" ]]; then
  echo "mesh launch preflight status summary: missing env files"
  echo "required: $vps_env and $laptop_env"
  exit 1
fi

extract_value() {
  local key="$1"
  local file="$2"
  awk -F= -v k="$key" '$1==k{print substr($0, index($0,$2)); exit}' "$file"
}

vps_endpoint="$(extract_value CHIMERA_MESH_REMOTE_ENDPOINT "$vps_env")"
laptop_endpoint="$(extract_value CHIMERA_MESH_REMOTE_ENDPOINT "$laptop_env")"

echo "mesh launch preflight status summary"
echo "- side_a remote endpoint: ${vps_endpoint:-<missing>}"
echo "- side_b remote endpoint: ${laptop_endpoint:-<missing>}"

if [[ -f docs/MESH_LAUNCH_PREFLIGHT_VPS.json ]]; then
  vps_status="$(jq -r '.status // "unknown"' docs/MESH_LAUNCH_PREFLIGHT_VPS.json)"
  vps_ready="$(jq -r '.ready_for_real_launch // false' docs/MESH_LAUNCH_PREFLIGHT_VPS.json)"
  echo "- vps artifact: status=${vps_status}, ready_for_real_launch=${vps_ready}"
else
  echo "- vps artifact: missing"
fi

if [[ -f docs/MESH_LAUNCH_PREFLIGHT_LAPTOP.json ]]; then
  laptop_status="$(jq -r '.status // "unknown"' docs/MESH_LAUNCH_PREFLIGHT_LAPTOP.json)"
  laptop_ready="$(jq -r '.ready_for_real_launch // false' docs/MESH_LAUNCH_PREFLIGHT_LAPTOP.json)"
  echo "- laptop artifact: status=${laptop_status}, ready_for_real_launch=${laptop_ready}"
else
  echo "- laptop artifact: missing"
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
