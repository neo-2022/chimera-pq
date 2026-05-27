#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MAX_AGE_SEC="${CHIMERA_MESH_PREFLIGHT_MAX_AGE_SEC:-1800}"
if ! [[ "$MAX_AGE_SEC" =~ ^[0-9]+$ ]] || (( MAX_AGE_SEC < 1 )); then
  echo "mesh launch preflight freshness guard: CHIMERA_MESH_PREFLIGHT_MAX_AGE_SEC must be positive integer"
  exit 1
fi

vps_json="${CHIMERA_MESH_PREFLIGHT_VPS_JSON:-docs/MESH_LAUNCH_PREFLIGHT_VPS.json}"
laptop_json="${CHIMERA_MESH_PREFLIGHT_LAPTOP_JSON:-docs/MESH_LAUNCH_PREFLIGHT_LAPTOP.json}"
verify_json="${CHIMERA_MESH_PREFLIGHT_VERIFY_JSON:-docs/MESH_LAUNCH_PREFLIGHT_VERIFY.json}"

for p in "$vps_json" "$laptop_json" "$verify_json"; do
  if [[ ! -f "$p" ]]; then
    echo "mesh launch preflight freshness guard: missing artifact: $p"
    exit 1
  fi
done

now_epoch="$(date +%s)"
vps_mtime="$(stat -c %Y "$vps_json")"
laptop_mtime="$(stat -c %Y "$laptop_json")"
verify_mtime="$(stat -c %Y "$verify_json")"

for pair in "$vps_json:$vps_mtime" "$laptop_json:$laptop_mtime" "$verify_json:$verify_mtime"; do
  file="${pair%%:*}"
  mtime="${pair##*:}"
  age=$(( now_epoch - mtime ))
  if (( age < 0 )); then
    echo "mesh launch preflight freshness guard: artifact mtime is in the future: $file"
    exit 1
  fi
  if (( age > MAX_AGE_SEC )); then
    echo "mesh launch preflight freshness guard: stale artifact ($age sec): $file"
    exit 1
  fi
done

if (( verify_mtime < vps_mtime || verify_mtime < laptop_mtime )); then
  echo "mesh launch preflight freshness guard: verify artifact older than peer artifacts"
  exit 1
fi

echo "mesh launch preflight freshness guard: PASS"
