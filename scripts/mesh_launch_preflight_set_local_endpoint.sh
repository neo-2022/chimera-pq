#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: mesh_launch_preflight_set_local_endpoint.sh <side_a|side_b> <host:port>"
  exit 2
fi

side="$1"
endpoint="$2"

case "$side" in
  side_a) env_file="configs/mesh_launch_preflight.vps.env" ;;
  side_b) env_file="configs/mesh_launch_preflight.laptop.env" ;;
  *)
    echo "mesh launch preflight set local endpoint: side must be side_a or side_b"
    exit 1
    ;;
esac

if [[ ! -f "$env_file" ]]; then
  echo "mesh launch preflight set local endpoint: missing env file: $env_file"
  exit 1
fi

if [[ "$endpoint" != *:* ]]; then
  echo "mesh launch preflight set local endpoint: endpoint must be host:port"
  exit 1
fi

host_part="${endpoint%:*}"
port_part="${endpoint##*:}"

if [[ -z "$host_part" ]]; then
  echo "mesh launch preflight set local endpoint: host must be non-empty"
  exit 1
fi
if ! [[ "$port_part" =~ ^[0-9]+$ ]]; then
  echo "mesh launch preflight set local endpoint: port is not numeric"
  exit 1
fi
if (( port_part < 1 || port_part > 65535 )); then
  echo "mesh launch preflight set local endpoint: port out of range"
  exit 1
fi

if grep -q '^CHIMERA_MESH_LOCAL_ENDPOINT=' "$env_file"; then
  perl -0pi -e 's/^CHIMERA_MESH_LOCAL_ENDPOINT=.*$/CHIMERA_MESH_LOCAL_ENDPOINT='"$endpoint"'/m' "$env_file"
else
  printf '\nCHIMERA_MESH_LOCAL_ENDPOINT=%s\n' "$endpoint" >> "$env_file"
fi

echo "mesh launch preflight set local endpoint: updated $env_file"
echo "CHIMERA_MESH_LOCAL_ENDPOINT=$endpoint"
