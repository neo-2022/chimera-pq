#!/usr/bin/env bash
set -euo pipefail

env_file="configs/mesh_launch_preflight.side_b.env"
if [[ ! -f "$env_file" ]]; then
  echo "mesh launch preflight print unblock cmd: missing $env_file"
  exit 1
fi

side_b_local_endpoint="$(awk -F= '$1=="CHIMERA_MESH_LOCAL_ENDPOINT"{print substr($0, index($0,$2)); exit}' "$env_file")"
if [[ -z "${side_b_local_endpoint// }" ]]; then
  echo "mesh launch preflight print unblock cmd: CHIMERA_MESH_LOCAL_ENDPOINT missing in $env_file"
  echo "run auto-bind first so discovery/runtime state can resolve current endpoints"
  echo "automatic command:"
  echo "just mesh-launch-preflight-autopilot"
  exit 1
fi

if [[ "$side_b_local_endpoint" == "__AUTO__" ]]; then
  echo "mesh launch preflight print unblock cmd: CHIMERA_MESH_LOCAL_ENDPOINT is __AUTO__ in $env_file"
  echo "run auto-bind first so discovery/runtime state can replace it automatically"
  echo "automatic command:"
  echo "just mesh-launch-preflight-autopilot"
  echo "manual <host:port> helper is fallback only"
  exit 1
fi

host_part="${side_b_local_endpoint%:*}"
if [[ "$host_part" =~ ^198\.51\.100\. || "$host_part" =~ ^203\.0\.113\. || "$host_part" =~ ^192\.0\.2\. ]]; then
  echo "mesh launch preflight print unblock cmd: CHIMERA_MESH_LOCAL_ENDPOINT is a documentation placeholder: $side_b_local_endpoint"
  echo "run auto-bind first so the placeholder can be replaced automatically"
  echo "automatic command:"
  echo "just mesh-launch-preflight-autopilot"
  echo "manual <host:port> helper is fallback only"
  exit 1
fi

echo "just mesh-launch-preflight-autopilot"
