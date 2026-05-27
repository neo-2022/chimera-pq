#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: mesh_launch_preflight_env_guard.sh <env_file>"
  exit 2
fi

ENV_FILE="$1"
if [[ ! -f "$ENV_FILE" ]]; then
  echo "mesh launch preflight env guard: missing env file: $ENV_FILE"
  exit 1
fi

set -a
# shellcheck disable=SC1090
source "$ENV_FILE"
set +a

required_vars=(
  CHIMERA_MESH_LOCAL_ROLE
  CHIMERA_MESH_NAMESPACE
  CHIMERA_MESH_LOCAL_NODE
  CHIMERA_MESH_REMOTE_NODE
  CHIMERA_MESH_REMOTE_ENDPOINT
  CHIMERA_MESH_LOCAL_OUT
  CHIMERA_MESH_REMOTE_OUT
)

for key in "${required_vars[@]}"; do
  value="${!key-}"
  if [[ -z "${value// }" ]]; then
    echo "mesh launch preflight env guard: missing or blank: $key"
    exit 1
  fi
done

if [[ "$CHIMERA_MESH_LOCAL_ROLE" != "side_a" && "$CHIMERA_MESH_LOCAL_ROLE" != "side_b" ]]; then
  echo "mesh launch preflight env guard: CHIMERA_MESH_LOCAL_ROLE must be side_a or side_b"
  exit 1
fi

allow_remote_missing="${CHIMERA_MESH_ALLOW_REMOTE_MISSING:-0}"
if [[ "$allow_remote_missing" != "0" && "$allow_remote_missing" != "1" ]]; then
  echo "mesh launch preflight env guard: CHIMERA_MESH_ALLOW_REMOTE_MISSING must be 0 or 1"
  exit 1
fi

if [[ "$CHIMERA_MESH_LOCAL_NODE" == "$CHIMERA_MESH_REMOTE_NODE" ]]; then
  echo "mesh launch preflight env guard: local and remote node must differ"
  exit 1
fi

if [[ "$CHIMERA_MESH_LOCAL_OUT" == "$CHIMERA_MESH_REMOTE_OUT" ]]; then
  echo "mesh launch preflight env guard: local and remote out paths must differ"
  exit 1
fi

local_out_base="$(basename -- "$CHIMERA_MESH_LOCAL_OUT")"
remote_out_base="$(basename -- "$CHIMERA_MESH_REMOTE_OUT")"
if [[ "$local_out_base" == "$remote_out_base" ]]; then
  echo "mesh launch preflight env guard: local and remote out filenames must differ"
  exit 1
fi

if [[ "$CHIMERA_MESH_REMOTE_ENDPOINT" != *:* ]]; then
  echo "mesh launch preflight env guard: remote endpoint must be host:port"
  exit 1
fi

port_part="${CHIMERA_MESH_REMOTE_ENDPOINT##*:}"
if ! [[ "$port_part" =~ ^[0-9]+$ ]]; then
  echo "mesh launch preflight env guard: remote endpoint port is not numeric"
  exit 1
fi
if (( port_part < 1 || port_part > 65535 )); then
  echo "mesh launch preflight env guard: remote endpoint port out of range"
  exit 1
fi

host_part="${CHIMERA_MESH_REMOTE_ENDPOINT%:*}"
is_placeholder_host=0
if [[ "$host_part" =~ ^198\.51\.100\. ]]; then
  is_placeholder_host=1
elif [[ "$host_part" =~ ^203\.0\.113\. ]]; then
  is_placeholder_host=1
elif [[ "$host_part" =~ ^192\.0\.2\. ]]; then
  is_placeholder_host=1
fi

if (( is_placeholder_host == 1 )) && [[ "$allow_remote_missing" != "1" ]]; then
  echo "mesh launch preflight env guard: remote endpoint uses test placeholder range; set real host:port or use CHIMERA_MESH_ALLOW_REMOTE_MISSING=1 for local-only staged checks"
  exit 1
fi

local_endpoint="${CHIMERA_MESH_LOCAL_ENDPOINT:-}"
if [[ -n "${local_endpoint// }" ]]; then
  if [[ "$local_endpoint" != *:* ]]; then
    echo "mesh launch preflight env guard: local endpoint must be host:port"
    exit 1
  fi
  local_port_part="${local_endpoint##*:}"
  if ! [[ "$local_port_part" =~ ^[0-9]+$ ]]; then
    echo "mesh launch preflight env guard: local endpoint port is not numeric"
    exit 1
  fi
  if (( local_port_part < 1 || local_port_part > 65535 )); then
    echo "mesh launch preflight env guard: local endpoint port out of range"
    exit 1
  fi

  local_host_part="${local_endpoint%:*}"
  local_is_placeholder_host=0
  if [[ "$local_host_part" =~ ^198\.51\.100\. ]]; then
    local_is_placeholder_host=1
  elif [[ "$local_host_part" =~ ^203\.0\.113\. ]]; then
    local_is_placeholder_host=1
  elif [[ "$local_host_part" =~ ^192\.0\.2\. ]]; then
    local_is_placeholder_host=1
  fi
  if (( local_is_placeholder_host == 1 )) && [[ "$allow_remote_missing" != "1" ]]; then
    echo "mesh launch preflight env guard: local endpoint uses test placeholder range; set real host:port or use CHIMERA_MESH_ALLOW_REMOTE_MISSING=1 for local-only staged checks"
    exit 1
  fi
fi

policy_payload="${CHIMERA_MESH_POLICY_PAYLOAD:-}"
traffic_profile="${CHIMERA_MESH_TRAFFIC_PROFILE:-}"
if [[ -n "$policy_payload" && -n "$traffic_profile" ]]; then
  echo "mesh launch preflight env guard: set either CHIMERA_MESH_POLICY_PAYLOAD or CHIMERA_MESH_TRAFFIC_PROFILE, not both"
  exit 1
fi

if [[ -n "$traffic_profile" ]]; then
  case "$traffic_profile" in
    high_speed_anonymous|privacy_first|speed_first|low_latency_private) ;;
    *)
      echo "mesh launch preflight env guard: invalid CHIMERA_MESH_TRAFFIC_PROFILE"
      exit 1
      ;;
  esac
fi

echo "mesh launch preflight env guard: PASS"
