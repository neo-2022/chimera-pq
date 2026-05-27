#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: mesh_launch_preflight_endpoint_probe.sh <env_file>"
  exit 2
fi

env_file="$1"
if [[ ! -f "$env_file" ]]; then
  echo "mesh launch preflight endpoint probe: missing env file: $env_file"
  exit 1
fi

set -a
# shellcheck disable=SC1090
source "$env_file"
set +a

endpoint="${CHIMERA_MESH_REMOTE_ENDPOINT:-}"
allow_remote_missing="${CHIMERA_MESH_ALLOW_REMOTE_MISSING:-0}"
timeout_sec="${CHIMERA_MESH_ENDPOINT_PROBE_TIMEOUT_SEC:-2}"

if [[ -z "${endpoint// }" ]]; then
  echo "mesh launch preflight endpoint probe: missing or blank CHIMERA_MESH_REMOTE_ENDPOINT"
  exit 1
fi

if [[ "$allow_remote_missing" == "1" ]]; then
  echo "mesh launch preflight endpoint probe: skipped (CHIMERA_MESH_ALLOW_REMOTE_MISSING=1)"
  exit 0
fi

if [[ "$endpoint" != *:* ]]; then
  echo "mesh launch preflight endpoint probe: endpoint must be host:port"
  exit 1
fi

host="${endpoint%:*}"
port="${endpoint##*:}"

if ! [[ "$port" =~ ^[0-9]+$ ]]; then
  echo "mesh launch preflight endpoint probe: endpoint port is not numeric"
  exit 1
fi
if (( port < 1 || port > 65535 )); then
  echo "mesh launch preflight endpoint probe: endpoint port out of range"
  exit 1
fi
if ! [[ "$timeout_sec" =~ ^[0-9]+$ ]] || (( timeout_sec < 1 )); then
  echo "mesh launch preflight endpoint probe: CHIMERA_MESH_ENDPOINT_PROBE_TIMEOUT_SEC must be positive integer"
  exit 1
fi

if timeout "$timeout_sec" bash -lc "</dev/tcp/$host/$port" 2>/dev/null; then
  echo "mesh launch preflight endpoint probe: PASS ($host:$port reachable)"
  exit 0
fi

echo "mesh launch preflight endpoint probe: FAIL ($host:$port unreachable within ${timeout_sec}s)"
exit 1
