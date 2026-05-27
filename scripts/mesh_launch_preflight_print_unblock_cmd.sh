#!/usr/bin/env bash
set -euo pipefail

env_file="configs/mesh_launch_preflight.laptop.env"
if [[ ! -f "$env_file" ]]; then
  echo "mesh launch preflight print unblock cmd: missing $env_file"
  exit 1
fi

laptop_local_endpoint="$(awk -F= '$1=="CHIMERA_MESH_LOCAL_ENDPOINT"{print substr($0, index($0,$2)); exit}' "$env_file")"
if [[ -z "${laptop_local_endpoint// }" ]]; then
  echo "mesh launch preflight print unblock cmd: CHIMERA_MESH_LOCAL_ENDPOINT missing in $env_file"
  echo "set CHIMERA_MESH_LOCAL_ENDPOINT=<real_laptop_host:port> and rerun"
  echo "fallback command template:"
  echo "just mesh-launch-preflight-unblock-and-run <laptop_host:port>"
  exit 1
fi

host_part="${laptop_local_endpoint%:*}"
if [[ "$host_part" =~ ^198\.51\.100\. || "$host_part" =~ ^203\.0\.113\. || "$host_part" =~ ^192\.0\.2\. ]]; then
  echo "mesh launch preflight print unblock cmd: CHIMERA_MESH_LOCAL_ENDPOINT is a documentation placeholder: $laptop_local_endpoint"
  echo "replace with real laptop host:port, then rerun"
  echo "fallback command template:"
  echo "just mesh-launch-preflight-unblock-and-run <laptop_host:port>"
  exit 1
fi

echo "just mesh-launch-preflight-unblock-and-run $laptop_local_endpoint"
