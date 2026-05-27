#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
cd "$repo_root"

vps_endpoint="${1:-${CHIMERA_MESH_VPS_ENDPOINT:-}}"

if [[ -z "$vps_endpoint" ]]; then
  echo "mesh launch preflight auto bind: vps endpoint is required via arg or CHIMERA_MESH_VPS_ENDPOINT"
  exit 2
fi

if [[ ! "$vps_endpoint" =~ ^[^:]+:[0-9]+$ ]]; then
  echo "mesh launch preflight auto bind: vps endpoint must be host:port"
  exit 2
fi

have_cmd() {
  command -v "$1" >/dev/null 2>&1
}

probe_endpoint() {
  local endpoint="$1"
  local host port
  host="${endpoint%:*}"
  port="${endpoint##*:}"
  if timeout 2 bash -lc "exec 3<>/dev/tcp/${host}/${port}" >/dev/null 2>&1; then
    return 0
  fi
  return 1
}

pick_public_ip() {
  local ip=""
  if have_cmd curl; then
    ip="$(curl -4 -s --max-time 3 https://api.ipify.org || true)"
    if [[ -z "$ip" ]]; then
      ip="$(curl -4 -s --max-time 3 ifconfig.me || true)"
    fi
  fi
  echo "$ip"
}

declare -a candidates=()

public_ip="$(pick_public_ip)"
if [[ "$public_ip" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  candidates+=("${public_ip}:443")
  candidates+=("${public_ip}:8443")
  candidates+=("${public_ip}:8090")
  candidates+=("${public_ip}:8092")
  candidates+=("${public_ip}:36163")
  candidates+=("${public_ip}:40037")
fi

candidates+=("127.0.0.1:8443")
candidates+=("127.0.0.1:443")

selected=""
for endpoint in "${candidates[@]}"; do
  if probe_endpoint "$endpoint"; then
    selected="$endpoint"
    break
  fi
done

if [[ -z "$selected" ]]; then
  echo "mesh launch preflight auto bind: no reachable laptop endpoint found"
  echo "tried:"
  for endpoint in "${candidates[@]}"; do
    echo "  - $endpoint"
  done
  exit 1
fi

echo "mesh launch preflight auto bind: selected laptop endpoint $selected"
just mesh-launch-preflight-set-real-endpoints "$selected" "$vps_endpoint"
echo "mesh launch preflight auto bind: configured and ready-check passed"
