#!/usr/bin/env bash
set -euo pipefail

OUT_FILE="${1:-${CHIMERA_MESH_CONTROL_PLANE_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/mesh-control-plane.env}}"

fail() {
  echo "mesh control-plane env: $1" >&2
  exit 1
}

trim_ascii() {
  local value="${1:-}"
  value="${value#"${value%%[![:space:]]*}"}"
  value="${value%"${value##*[![:space:]]}"}"
  printf '%s' "$value"
}

shell_quote_env_value() {
  local key="${1:?key_required}"
  local value="${2:-}"
  if printf '%s' "$value" | LC_ALL=C grep -q '[[:cntrl:]]'; then
    fail "invalid control character in env value: $key"
  fi
  printf '%q' "$value"
}

write_env_kv() {
  local key="${1:?key_required}"
  local value="${2:-}"
  [[ "$key" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || fail "invalid env key: $key"
  printf '%s=%s\n' "$key" "$(shell_quote_env_value "$key" "$value")"
}

validate_score() {
  local key="${1:?key_required}"
  local value="${2:?value_required}"
  if ! [[ "$value" =~ ^[0-9]+$ ]]; then
    fail "$key must be an integer"
  fi
  if (( value < 0 || value > 100 )); then
    fail "$key must be between 0 and 100"
  fi
}

validate_endpoint() {
  local key="${1:?key_required}"
  local value="${2:?value_required}"
  if [[ "$value" != *:* ]]; then
    fail "$key must be host:port"
  fi
  local port="${value##*:}"
  if ! [[ "$port" =~ ^[0-9]+$ ]]; then
    fail "$key port must be numeric"
  fi
  if (( port < 1 || port > 65535 )); then
    fail "$key port out of range"
  fi
}

validate_peer_spec() {
  local value="${1:?peer_spec_required}"
  if ! [[ "$value" =~ ^[^@[:space:]]+@[^@[:space:]]+:[0-9]+@[^@[:space:]]+@[0-9]+@[0-9]+$ ]]; then
    fail "CHIMERA_MESH_REMOTE_PEER_SPEC must be node@endpoint:port#region@load@reliability"
  fi
}

policy_payload_has_route_binding() {
  local payload="${1:-}"
  [[ "$payload" =~ (^|[;\ ]+)[Mm][Ee][Ss][Hh]_[Rr][Oo][Uu][Tt][Ee]_[Bb][Ii][Nn][Dd][Ii][Nn][Gg]_[Ii][Dd]=[1-9][0-9]*($|[;\ ]+) ]]
}

namespace="$(trim_ascii "${CHIMERA_MESH_NAMESPACE:-}")"
local_node="$(trim_ascii "${CHIMERA_MESH_LOCAL_NODE:-}")"
policy_payload="$(trim_ascii "${CHIMERA_MESH_POLICY_PAYLOAD:-}")"
traffic_profile="$(trim_ascii "${CHIMERA_MESH_TRAFFIC_PROFILE:-}")"
remote_peer_spec="$(trim_ascii "${CHIMERA_MESH_REMOTE_PEER_SPEC:-}")"
extra_peers="$(trim_ascii "${CHIMERA_MESH_EXTRA_PEERS:-}")"

[[ -n "$namespace" ]] || fail "missing CHIMERA_MESH_NAMESPACE"
[[ -n "$local_node" ]] || fail "missing CHIMERA_MESH_LOCAL_NODE"

if [[ -n "$policy_payload" && -n "$traffic_profile" ]]; then
  fail "set either CHIMERA_MESH_POLICY_PAYLOAD or CHIMERA_MESH_TRAFFIC_PROFILE, not both"
fi
case "$traffic_profile" in
  ""|high_speed_anonymous|privacy_first|speed_first|low_latency_private) ;;
  *) fail "invalid CHIMERA_MESH_TRAFFIC_PROFILE" ;;
esac

if [[ -z "$policy_payload" && -z "$traffic_profile" ]]; then
  echo "mesh_control_plane_env=skipped reason=missing_authoritative_policy"
  exit 0
fi

if [[ -n "$policy_payload" ]] && ! policy_payload_has_route_binding "$policy_payload"; then
  echo "mesh_control_plane_env=skipped reason=missing_route_binding_id"
  exit 0
fi

if [[ -z "$remote_peer_spec" ]]; then
  remote_node="$(trim_ascii "${CHIMERA_MESH_REMOTE_NODE:-}")"
  remote_endpoint="$(trim_ascii "${CHIMERA_MESH_REMOTE_ENDPOINT:-}")"
  remote_region="$(trim_ascii "${CHIMERA_MESH_REMOTE_REGION:-}")"
  remote_load="$(trim_ascii "${CHIMERA_MESH_REMOTE_LOAD_SCORE:-}")"
  remote_reliability="$(trim_ascii "${CHIMERA_MESH_REMOTE_RELIABILITY_SCORE:-}")"
  [[ -n "$remote_node" ]] || fail "missing CHIMERA_MESH_REMOTE_NODE"
  [[ -n "$remote_endpoint" ]] || fail "missing CHIMERA_MESH_REMOTE_ENDPOINT"
  [[ -n "$remote_region" ]] || fail "missing CHIMERA_MESH_REMOTE_REGION"
  [[ -n "$remote_load" ]] || fail "missing CHIMERA_MESH_REMOTE_LOAD_SCORE"
  [[ -n "$remote_reliability" ]] || fail "missing CHIMERA_MESH_REMOTE_RELIABILITY_SCORE"
  validate_endpoint "CHIMERA_MESH_REMOTE_ENDPOINT" "$remote_endpoint"
  validate_score "CHIMERA_MESH_REMOTE_LOAD_SCORE" "$remote_load"
  validate_score "CHIMERA_MESH_REMOTE_RELIABILITY_SCORE" "$remote_reliability"
  remote_peer_spec="${remote_node}@${remote_endpoint}@${remote_region}@${remote_load}@${remote_reliability}"
fi
validate_peer_spec "$remote_peer_spec"

mkdir -p "$(dirname "$OUT_FILE")"
tmp_file="$(mktemp)"
{
  write_env_kv "CHIMERA_MESH_NAMESPACE" "$namespace"
  write_env_kv "CHIMERA_MESH_LOCAL_NODE" "$local_node"
  if [[ -n "$policy_payload" ]]; then
    write_env_kv "CHIMERA_MESH_POLICY_PAYLOAD" "$policy_payload"
  else
    write_env_kv "CHIMERA_MESH_TRAFFIC_PROFILE" "$traffic_profile"
  fi
  write_env_kv "CHIMERA_MESH_REMOTE_PEER_SPEC" "$remote_peer_spec"
  if [[ -n "$extra_peers" ]]; then
    write_env_kv "CHIMERA_MESH_EXTRA_PEERS" "$extra_peers"
  fi
} >"$tmp_file"
cat "$tmp_file" >"$OUT_FILE"
rm -f "$tmp_file"
chmod 600 "$OUT_FILE"
echo "mesh_control_plane_env=ok"
echo "mesh_control_plane_env_file=$OUT_FILE"
