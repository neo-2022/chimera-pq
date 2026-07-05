#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
cd "$repo_root"

side_a_endpoint="${1:-${CHIMERA_MESH_SIDE_A_ENDPOINT:-}}"
side_a_env_file="${CHIMERA_MESH_SIDE_A_ENV_FILE:-configs/mesh_launch_preflight.side_a.env}"
side_b_endpoint="${CHIMERA_MESH_SIDE_B_ENDPOINT:-${CHIMERA_MESH_REMOTE_ENDPOINT:-}}"
side_b_env_file="${CHIMERA_MESH_SIDE_B_ENV_FILE:-configs/mesh_launch_preflight.side_b.env}"
mesh_nodes_config="${CHIMERA_MESH_NODES_CONFIG:-${CHIMERA_MESH_CONFIG_FILE:-configs/mesh_nodes.example.conf}}"
endpoint_resolve_timeout_ms="${CHIMERA_MESH_ENDPOINT_RESOLVE_TIMEOUT_MS:-1200}"
json_max_bytes="${CHIMERA_MESH_AUTOBIND_JSON_MAX_BYTES:-1048576}"

trim_ascii() {
  local value="${1:-}"
  value="${value#"${value%%[![:space:]]*}"}"
  value="${value%"${value##*[![:space:]]}"}"
  printf '%s' "$value"
}

is_placeholder_endpoint() {
  local endpoint="${1:-}"
  [[ "$endpoint" =~ ^(198\.51\.100\.|203\.0\.113\.|192\.0\.2\.)[^:]+:[0-9]+$ ]]
}

is_auto_value() {
  local value
  value="$(trim_ascii "${1:-}")"
  [[ -z "$value" || "$value" == "__AUTO__" ]] || is_placeholder_endpoint "$value"
}

is_endpoint() {
  local endpoint host port
  endpoint="$(trim_ascii "${1:-}")"
  [[ "$endpoint" =~ ^[^[:space:]:]+:[0-9]+$ ]] || return 1
  host="${endpoint%:*}"
  port="${endpoint##*:}"
  [[ -n "$host" && "$port" =~ ^[0-9]+$ ]] || return 1
  (( port >= 1 && port <= 65535 )) || return 1
  ! is_placeholder_endpoint "$endpoint"
}

read_env_value_from_file() {
  local key="${1:?key_required}"
  local file="${2:?file_required}"
  [[ -f "$file" ]] || return 0
  awk -F= -v key="$key" '$1 == key { print substr($0, index($0, $2)); exit }' "$file" 2>/dev/null || true
}

run_mesh_cli() {
  if [[ -n "${CHIMERA_RUNNER:-}" && -x "${CHIMERA_RUNNER:-}" ]]; then
    "$CHIMERA_RUNNER" cli "$@"
    return $?
  fi
  if [[ -x "$repo_root/scripts/chimera-runner.sh" ]]; then
    "$repo_root/scripts/chimera-runner.sh" cli "$@"
    return $?
  fi
  if [[ -x "$repo_root/bin/chimera-cli" ]]; then
    "$repo_root/bin/chimera-cli" "$@"
    return $?
  fi
  echo "error: shipped chimera-cli binary is missing" >&2
  return 1
}

read_endpoint_from_json_file() {
  local path="${1:?path_required}"
  local node_id="${2:-}"
  [[ -f "$path" ]] || return 1
  command -v jq >/dev/null 2>&1 || return 1
  local size
  size="$(wc -c <"$path" 2>/dev/null || echo 0)"
  [[ "$size" =~ ^[0-9]+$ ]] || return 1
  (( size > 0 && size <= json_max_bytes )) || return 1
  local candidates candidate
  candidates="$(jq -er --arg node_id "$node_id" '
    def clean: strings | gsub("^\\s+|\\s+$"; "");
    def endpoint:
      (.endpoint // .current_endpoint // .published_endpoint // .resolved_peer_listen // .listen // .peer_listen // empty)
      | clean;
    if type == "object" and has("nodes") and has("signature") and has("key_id") then
      .nodes[]
      | select(($node_id == "") or (.node_id == $node_id))
      | endpoint
    elif type == "object" then
      select(($node_id == "") or (.node_id? == $node_id) or (.self_node_id? == $node_id))
      | endpoint
    else
      empty
    end
  ' "$path" 2>/dev/null || true)"
  [[ -n "$candidates" ]] || return 1
  while IFS= read -r candidate; do
    candidate="$(trim_ascii "$candidate")"
    if is_endpoint "$candidate"; then
      printf '%s' "$candidate"
      return 0
    fi
  done <<<"$candidates"
  return 1
}

candidate_discovery_snapshot_paths() {
  local side_label="${1:?side_label_required}"
  local side_specific_var="CHIMERA_MESH_${side_label^^}_DISCOVERY_SNAPSHOT"
  local side_specific="${!side_specific_var:-}"
  local xdg_cache="${XDG_CACHE_HOME:-${HOME:-}/.cache}"
  printf '%s\n' \
    "$side_specific" \
    "${CHIMERA_MESH_DISCOVERY_SNAPSHOT:-}" \
    "${CHIMERA_MESH_NODES_DISCOVERY_SNAPSHOT:-}" \
    "${MESH_DISCOVERY_OUT_FILE:-}" \
    "mesh_nodes.discovery.json" \
    "${xdg_cache%/}/chimera/mesh_nodes.discovery.json"
}

resolve_endpoint_from_discovery_snapshot() {
  local node_id="${1:-}"
  local side_label="${2:?side_label_required}"
  [[ -n "$(trim_ascii "$node_id")" ]] || return 1
  local path endpoint
  while IFS= read -r path; do
    path="$(trim_ascii "$path")"
    [[ -n "$path" ]] || continue
    endpoint="$(read_endpoint_from_json_file "$path" "$node_id" || true)"
    if is_endpoint "$endpoint"; then
      printf '%s' "$endpoint"
      return 0
    fi
  done < <(candidate_discovery_snapshot_paths "$side_label")
  return 1
}

candidate_published_runtime_state_paths() {
  local side_label="${1:?side_label_required}"
  local side_specific_var="CHIMERA_MESH_${side_label^^}_PUBLISHED_RUNTIME_STATE"
  local side_specific="${!side_specific_var:-}"
  printf '%s\n' \
    "$side_specific" \
    "${CHIMERA_MESH_PUBLISHED_RUNTIME_STATE:-}" \
    "${CHIMERA_MESH_PEER_EGRESS_STATE_PATH:-}"
}

resolve_endpoint_from_published_runtime_state() {
  local node_id="${1:-}"
  local side_label="${2:?side_label_required}"
  local path endpoint
  while IFS= read -r path; do
    path="$(trim_ascii "$path")"
    [[ -n "$path" ]] || continue
    endpoint="$(read_endpoint_from_json_file "$path" "$node_id" || true)"
    if is_endpoint "$endpoint"; then
      printf '%s' "$endpoint"
      return 0
    fi
  done < <(candidate_published_runtime_state_paths "$side_label")
  return 1
}

resolve_endpoint_from_inventory() {
  local node_id="${1:-}"
  local label="${2:-node}"
  [[ -n "$(trim_ascii "$node_id")" ]] || return 1
  if [[ ! -f "$mesh_nodes_config" ]]; then
    return 1
  fi
  local runtime_state
  runtime_state="$(mktemp "${TMPDIR:-/tmp}/chimera-mesh-auto-bind-${label}.XXXXXX.json")"
  local endpoint=""
  if run_mesh_cli mesh nodes select --id "$node_id" --config "$mesh_nodes_config" --runtime-state "$runtime_state" --probe-timeout-ms "$endpoint_resolve_timeout_ms" >/dev/null 2>&1; then
    endpoint="$(
      run_mesh_cli mesh nodes selected-endpoint --config "$mesh_nodes_config" --runtime-state "$runtime_state" 2>/dev/null | head -n1 | tr -d '[:space:]'
    )"
  fi
  rm -f "$runtime_state"
  if is_endpoint "$endpoint"; then
    printf '%s' "$endpoint"
    return 0
  fi
  return 1
}

resolve_side_endpoint() {
  local side_label="${1:?side_label_required}"
  local node_key="${2:?node_key_required}"
  local env_file="${3:?env_file_required}"
  local explicit_endpoint="${4:-}"
  local endpoint="$explicit_endpoint"
  local node_id
  local env_endpoint=""

  node_id="$(trim_ascii "$(read_env_value_from_file "$node_key" "$env_file")")"
  env_endpoint="$(trim_ascii "$(read_env_value_from_file CHIMERA_MESH_LOCAL_ENDPOINT "$env_file")")"

  if endpoint="$(resolve_endpoint_from_published_runtime_state "$node_id" "$side_label" || true)" && is_endpoint "$endpoint"; then
    printf '%s' "$endpoint"
    return 0
  fi

  if endpoint="$(resolve_endpoint_from_discovery_snapshot "$node_id" "$side_label" || true)" && is_endpoint "$endpoint"; then
    printf '%s' "$endpoint"
    return 0
  fi

  if [[ -n "$node_id" ]]; then
    endpoint="$(resolve_endpoint_from_inventory "$node_id" "$side_label" || true)"
    if is_endpoint "$endpoint"; then
      printf '%s' "$endpoint"
      return 0
    fi
  fi

  if ! is_auto_value "$explicit_endpoint" && is_endpoint "$explicit_endpoint"; then
    printf '%s' "$(trim_ascii "$explicit_endpoint")"
    return 0
  fi

  if ! is_auto_value "$env_endpoint" && is_endpoint "$env_endpoint"; then
    printf '%s' "$env_endpoint"
    return 0
  fi

  return 1
}

side_a_node_key="CHIMERA_MESH_LOCAL_NODE"
side_b_node_key="CHIMERA_MESH_LOCAL_NODE"

if side_a_endpoint="$(resolve_side_endpoint side_a "$side_a_node_key" "$side_a_env_file" "$side_a_endpoint" 2>/dev/null)"; then
  :
else
  echo "mesh launch preflight auto bind: side_a endpoint could not be resolved automatically"
  echo "  config: $mesh_nodes_config"
  echo "  env file: $side_a_env_file"
  exit 2
fi

if side_b_endpoint="$(resolve_side_endpoint side_b "$side_b_node_key" "$side_b_env_file" "$side_b_endpoint" 2>/dev/null)"; then
  :
else
  echo "mesh launch preflight auto bind: side_b endpoint could not be resolved automatically"
  echo "  config: $mesh_nodes_config"
  echo "  env file: $side_b_env_file"
  exit 2
fi

if [[ -z "$side_b_endpoint" ]] || ! is_endpoint "$side_b_endpoint"; then
  echo "mesh launch preflight auto bind: side_b endpoint must be host:port"
  exit 2
fi

echo "mesh launch preflight auto bind: selected side_b endpoint $side_b_endpoint"
just mesh-launch-preflight-set-real-endpoints "$side_b_endpoint" "$side_a_endpoint"
echo "mesh launch preflight auto bind: configured and ready-check passed"
