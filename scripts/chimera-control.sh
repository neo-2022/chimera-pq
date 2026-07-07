#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
STATE_FILE="${STATE_FILE:-$ROOT_DIR/docs/runtime_state_latest.json}"
INSTALL_LOCAL_BIN_FILE="${INSTALL_LOCAL_BIN_FILE:-$ROOT_DIR/.chimera_install_local_bin}"
SYSTEMD_USER_DIR="${SYSTEMD_USER_DIR:-${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user}"
SYSTEMD_USER_DEFAULT_TARGET_WANTS_DIR="${SYSTEMD_USER_DEFAULT_TARGET_WANTS_DIR:-$SYSTEMD_USER_DIR/default.target.wants}"
APPLICATIONS_DIR="${APPLICATIONS_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/applications}"
CHIMERA_CONFIG_DIR="${CHIMERA_CONFIG_DIR:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera}"
CHIMERA_CACHE_DIR="${CHIMERA_CACHE_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera}"
DEFAULT_LOCAL_BIN_DIR="${HOME}/.local/bin"
if [[ -f "$INSTALL_LOCAL_BIN_FILE" ]]; then
  stored_local_bin_dir="$(tr -d '\r\n' <"$INSTALL_LOCAL_BIN_FILE" 2>/dev/null || true)"
  if [[ -n "${stored_local_bin_dir:-}" ]]; then
    DEFAULT_LOCAL_BIN_DIR="$stored_local_bin_dir"
  fi
fi
LOCAL_BIN_DIR="${LOCAL_BIN_DIR:-${CHIMERA_LOCAL_BIN:-$DEFAULT_LOCAL_BIN_DIR}}"
RUNTIME_SERVICE_UNIT="${CHIMERA_RUNTIME_SERVICE_UNIT:-chimera-runtime.service}"
NODE_SERVICE_UNIT="${CHIMERA_NODE_SERVICE_UNIT:-chimera-node.service}"
DATAPATH_SERVICE_UNIT="${CHIMERA_DATAPATH_SERVICE_UNIT:-chimera-datapath.service}"
SITE_AUTOWATCH_SERVICE_UNIT="${CHIMERA_SITE_AUTOWATCH_SERVICE_UNIT:-chimera-site-watch.service}"
LEGACY_NODE_COMPAT_SERVICE_UNIT="${LEGACY_NODE_COMPAT_SERVICE_UNIT:-${CHIMERA_LEGACY_NODE_SERVICE_UNIT:-chimera-gateway.service}}"
LEGACY_DATAPATH_COMPAT_SERVICE_UNIT="${LEGACY_DATAPATH_COMPAT_SERVICE_UNIT:-${CHIMERA_LEGACY_DATAPATH_SERVICE_UNIT:-chimera-client.service}}"
NODE_LOG="${NODE_LOG:-${GATEWAY_LOG:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/chimera_node.service.log}}"
DATAPATH_LOG="${DATAPATH_LOG:-${CLIENT_LOG:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/chimera_datapath.service.log}}"
UI_MODE_FILE="${UI_MODE_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/ui_mode}"
BOOTSTRAP_ENV_FILE="${CHIMERA_BOOTSTRAP_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/mesh_bootstrap.env}"
LEGACY_UPSTREAM_ENV_FILE="${LEGACY_UPSTREAM_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/legacy_upstream_probe.env}"
LEGACY_UPSTREAM_ENV_COMPAT_FILE="${LEGACY_UPSTREAM_ENV_COMPAT_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/upstream_proxy.env}"
UPSTREAM_ENV_FILE="${UPSTREAM_ENV_FILE:-$BOOTSTRAP_ENV_FILE}"
CHIMERA_PROTECTED_PORTS_CSV="${CHIMERA_PROTECTED_PORTS_CSV:-11080,22180}"
CHIMERA_SAFE_HOST_LOCK="${CHIMERA_SAFE_HOST_LOCK:-1}"
CHIMERA_ALLOW_LOCAL_NETWORK_MUTATION="${CHIMERA_ALLOW_LOCAL_NETWORK_MUTATION:-0}"
POLICY_FILE="${POLICY_FILE:-$ROOT_DIR/configs/policy.runtime.conf}"
MANUAL_TRANSIT_DOMAINS_FILE="${MANUAL_TRANSIT_DOMAINS_FILE:-$ROOT_DIR/configs/manual_transit_domains.txt}"
LEGACY_MANUAL_COMPAT_DOMAINS_FILE="${LEGACY_MANUAL_COMPAT_DOMAINS_FILE:-${MANUAL_GATEWAY_DOMAINS_FILE:-$ROOT_DIR/configs/manual_gateway_domains.txt}}"
ADAPTIVE_DOMAINS_FILE="${ADAPTIVE_DOMAINS_FILE:-$ROOT_DIR/configs/adaptive_domains.txt}"
APP_ROUTES_FILE="${APP_ROUTES_FILE:-$ROOT_DIR/configs/chimera-app-routes.conf}"
SERVICE_ROUTE_OVERRIDES_FILE="${SERVICE_ROUTE_OVERRIDES_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/service_route_overrides.conf}"
ROUTE_MODE_FILE="${ROUTE_MODE_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/route_mode}"
SPLIT_LIST_MODE_FILE="${SPLIT_LIST_MODE_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/split_list_mode}"
AUTOFIX_SCRIPT="$ROOT_DIR/scripts/chimera-autofix.sh"
AUTOFIX_TIMEOUT="${CHIMERA_AUTOFIX_MAX_TIME:-25}"
CHIMERA_ALLOW_PGREP_KILL="${CHIMERA_ALLOW_PGREP_KILL:-0}"
AUTO_RESTART_CHROMIUM="${CHIMERA_AUTO_RESTART_CHROMIUM:-0}"
CHIMERA_SYSTEM_INTEGRATION="${CHIMERA_SYSTEM_INTEGRATION:-0}"
UPSTREAM_STRATEGY="${CHIMERA_UPSTREAM_STRATEGY:-balanced}"
UPSTREAM_STICKY_SEC="${CHIMERA_UPSTREAM_STICKY_SEC:-120}"
LAST_ENDPOINT_FILE="${XDG_CACHE_HOME:-$HOME/.cache}/chimera/last_upstream_endpoint"
UPSTREAM_HEALTH_STATE_FILE="${XDG_CACHE_HOME:-$HOME/.cache}/chimera/upstream_health_state"
SITE_ADAPTIVE_DB_FILE="${SITE_ADAPTIVE_DB_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/site_adaptive_routes.db}"
SITE_AUTO_SEEDS_FILE="${SITE_AUTO_SEEDS_FILE:-$ROOT_DIR/configs/auto_failover_seeds.txt}"
SITE_AUTOWATCH_PID_FILE="${SITE_AUTOWATCH_PID_FILE:-${XDG_RUNTIME_DIR:-/tmp}/chimera-site-autowatch.pid}"
SITE_AUTOWATCH_INTERVAL_SEC="${SITE_AUTOWATCH_INTERVAL_SEC:-60}"
SITE_AUTOWATCH_FAILURE_BUDGET="${SITE_AUTOWATCH_FAILURE_BUDGET:-3}"
SITE_AUTOWATCH_ENABLED="${SITE_AUTOWATCH_ENABLED:-1}"
SITE_AUTO_DISCOVERY_ENABLED="${SITE_AUTO_DISCOVERY_ENABLED:-1}"
SITE_AUTO_DISCOVERY_LOOKBACK_SEC="${SITE_AUTO_DISCOVERY_LOOKBACK_SEC:-120}"
SITE_DISCOVERY_DOMAINS_FILE="${SITE_DISCOVERY_DOMAINS_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/discovered_domains.txt}"
SITE_FAILOVER_DATAPATH_THRESHOLD="${SITE_FAILOVER_DATAPATH_THRESHOLD:-1}"
SITE_FAILBACK_DIRECT_THRESHOLD="${SITE_FAILBACK_DIRECT_THRESHOLD:-3}"
SITE_ADAPTIVE_ENTRY_TTL_SEC="${SITE_ADAPTIVE_ENTRY_TTL_SEC:-86400}"
AUTOFIX_LOG_FILE="${AUTOFIX_LOG_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/autofix.log}"
CHIMERA_CLI_BIN="${CHIMERA_CLI_BIN:-$ROOT_DIR/bin/chimera-cli}"
CHIMERA_NODE_BIN="${CHIMERA_NODE_BIN:-${CHIMERA_GATEWAY_BIN:-$ROOT_DIR/bin/chimera-node}}"
CHIMERA_BOOTSTRAP_BIN="${CHIMERA_BOOTSTRAP_BIN:-$ROOT_DIR/bin/chimera-bootstrap}"
CHIMERA_RUNNER="${CHIMERA_RUNNER:-$ROOT_DIR/scripts/chimera-runner.sh}"
NODE_CONFIG_FILE="${NODE_CONFIG_FILE:-$ROOT_DIR/configs/mesh-node.conf}"
PEER_EGRESS_ENV_FILE="${PEER_EGRESS_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/peer-egress.env}"
PEER_EGRESS_STATE_FILE="${PEER_EGRESS_STATE_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/peer-egress.state}"
PEER_UPDATE_STATE_FILE="${PEER_UPDATE_STATE_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/peer-update.state.json}"
PEER_UPDATE_ENV_FILE="${PEER_UPDATE_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/peer-update.env}"
PEER_UPDATE_PID_FILE="${PEER_UPDATE_PID_FILE:-${XDG_RUNTIME_DIR:-/tmp}/chimera-peer-update.pid}"
PEER_UPDATE_LOG="${PEER_UPDATE_LOG:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/chimera_peer_update.service.log}"
MESH_CONTROL_PLANE_ENV_FILE="${MESH_CONTROL_PLANE_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/mesh-control-plane.env}"
MESH_DISCOVERY_OUT_FILE="${MESH_DISCOVERY_OUT_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/mesh_nodes.discovery.json}"
MESH_DISCOVERY_PUBKEY_OUT_FILE="${MESH_DISCOVERY_PUBKEY_OUT_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/mesh_nodes.discovery.pubkey}"
MESH_DISCOVERY_URLS_FILE="${MESH_DISCOVERY_URLS_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/mesh_nodes_discovery_urls.list}"
RUNTIME_LISTENER_OVERRIDE_FILE="${RUNTIME_LISTENER_OVERRIDE_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/runtime_listener_overrides.env}"
TRANSPARENT_RUNTIME_ENV_FILE="${TRANSPARENT_RUNTIME_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/transparent-runtime.env}"
SPLIT_TRANSPARENT_ENABLED="${SPLIT_TRANSPARENT_ENABLED:-1}"
SPLIT_TRANSPARENT_TUN_NAME="${SPLIT_TRANSPARENT_TUN_NAME:-chimera-tun}"
SPLIT_TRANSPARENT_TUN_ADDR="${SPLIT_TRANSPARENT_TUN_ADDR:-172.19.0.1/30}"
SPLIT_TRANSPARENT_TUN_ADDR6="${SPLIT_TRANSPARENT_TUN_ADDR6:-fd5a:7c0a:1::1/126}"
SPLIT_TRANSPARENT_AUTO_REDIRECT="${SPLIT_TRANSPARENT_AUTO_REDIRECT:-1}"
SPLIT_TRANSPARENT_CONFIG_FILE="${SPLIT_TRANSPARENT_CONFIG_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/transparent-runtime.json}"
SPLIT_TRANSPARENT_PID_FILE="${SPLIT_TRANSPARENT_PID_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/chimera-transparent-runtime.pid}"
SPLIT_TRANSPARENT_LOG_FILE="${SPLIT_TRANSPARENT_LOG_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/transparent-runtime.log}"
SPLIT_TRANSPARENT_LOG_LEVEL="${SPLIT_TRANSPARENT_LOG_LEVEL:-warn}"
SPLIT_TRANSPARENT_DNS_STRATEGY="${SPLIT_TRANSPARENT_DNS_STRATEGY:-prefer_ipv4}"
SPLIT_TRANSPARENT_WATCHDOG_PID_FILE="${SPLIT_TRANSPARENT_WATCHDOG_PID_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/chimera-split-watchdog.pid}"
CHIMERA_ALLOW_WEAVE_COEXIST_MUTATION="${CHIMERA_ALLOW_WEAVE_COEXIST_MUTATION:-0}"
CHIMERA_COEXIST_TRANSPARENT_CAPTURE="${CHIMERA_COEXIST_TRANSPARENT_CAPTURE:-1}"

# Inherit split-tunnel/full-tunnel apply flags from the current process
# environment when they are present and the caller has not overridden them.
# This honours `Environment=CHIMERA_APPLY_*=false` in the installed
# chimera-runtime.service unit and keeps an ordinary `chimera-sh -start` shell
# invocation safe-by-default unless the operator explicitly opts in.
chimera_apply_flag_default() {
  local key="${1:?key_required}"
  local value="${!key:-}"
  if [[ -z "$value" ]] && command -v systemctl >/dev/null 2>&1 && systemctl --user show-environment >/dev/null 2>&1; then
    value="$(systemctl --user show-environment 2>/dev/null | awk -F= -v k="$key" '$1 == k { sub(/^[^=]+=/, ""); print; exit }')"
  fi
  printf '%s' "${value:-false}"
}

CHIMERA_APPLY_DNS="${CHIMERA_APPLY_DNS:-$(chimera_apply_flag_default CHIMERA_APPLY_DNS)}"
CHIMERA_APPLY_TUN="${CHIMERA_APPLY_TUN:-$(chimera_apply_flag_default CHIMERA_APPLY_TUN)}"
CHIMERA_APPLY_ROUTE="${CHIMERA_APPLY_ROUTE:-$(chimera_apply_flag_default CHIMERA_APPLY_ROUTE)}"
CHIMERA_SERVICE_FWMARK="${CHIMERA_SERVICE_FWMARK:-0x5244}"
CHIMERA_ROUTE_FWMARK="${CHIMERA_ROUTE_FWMARK:-0x5244}"
CHIMERA_ROUTE_CIDR="${CHIMERA_ROUTE_CIDR:-0.0.0.0/1,128.0.0.0/1}"
CHIMERA_REQUIRE_UPSTREAM_FOR_FAILOVER="${CHIMERA_REQUIRE_UPSTREAM_FOR_FAILOVER:-1}"
CHIMERA_STRICT_FAILOVER_GATE="${CHIMERA_STRICT_FAILOVER_GATE:-1}"
CHIMERA_FLOW_PROOF_MAX_AGE_SEC="${CHIMERA_FLOW_PROOF_MAX_AGE_SEC:-300}"
NFT_BIN="${NFT_BIN:-}"

find_matching_tunnel_port() {
  local host="${1:-}"
  local user="${2:-}"
  if [[ -z "$host" || -z "$user" ]]; then
    return 1
  fi
  ps -eo args= 2>/dev/null | awk -v h="$host" -v u="$user" '
    /ssh/ && / -N / && / -D / {
      if (index($0, u "@" h) == 0) {
        next
      }
      for (i = 1; i <= NF; i++) {
        if ($i == "-D" && i + 1 <= NF) {
          split($(i+1), hp, ":")
          p = hp[length(hp)]
          if (p ~ /^[0-9]+$/) {
            print p
            exit
          }
        }
      }
    }'
}

upsert_env_kv() {
  local file="${1:?file_required}"
  local key="${2:?key_required}"
  local value="${3:-}"
  local quoted_value tmp_file replaced=0 line
  [[ "$key" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || {
    echo "error: invalid env key: $key" >&2
    exit 2
  }
  quoted_value="$(shell_quote_env_value "$key" "$value")"
  mkdir -p "$(dirname "$file")"
  touch "$file"
  tmp_file="$(mktemp)"
  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ "$line" == "$key="* ]]; then
      if [[ "$replaced" -eq 0 ]]; then
        printf '%s=%s\n' "$key" "$quoted_value"
        replaced=1
      fi
      continue
    fi
    printf '%s\n' "$line"
  done <"$file" >"$tmp_file"
  if [[ "$replaced" -eq 0 ]]; then
    printf '%s=%s\n' "$key" "$quoted_value" >>"$tmp_file"
  fi
  cat "$tmp_file" >"$file"
  /bin/rm -f -- "$tmp_file"
}

shell_quote_env_value() {
  local key="${1:?key_required}"
  local value="${2:-}"
  if printf '%s' "$value" | LC_ALL=C grep -q '[[:cntrl:]]'; then
    echo "error: invalid control character in env value: $key" >&2
    exit 2
  fi
  printf '%q' "$value"
}

write_env_kv() {
  local key="${1:?key_required}"
  local value="${2:-}"
  printf '%s=%s\n' "$key" "$(shell_quote_env_value "$key" "$value")"
}

bootstrap_env_key_allowed() {
  case "${1:-}" in
    CHIMERA_MESH_NODES_DISCOVERY_URL|CHIMERA_MESH_NODES_DISCOVERY_URLS|CHIMERA_MESH_NODES_DISCOVERY_PUBKEY|CHIMERA_MESH_NODES_DISCOVERY_KEYRING|CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS|CHIMERA_MESH_NAMESPACE|CHIMERA_MESH_LOCAL_NODE|CHIMERA_MESH_POLICY_PAYLOAD|CHIMERA_MESH_TRAFFIC_PROFILE|CHIMERA_MESH_REMOTE_PEER_SPEC|CHIMERA_MESH_EXTRA_PEERS|CHIMERA_MESH_REMOTE_NODE|CHIMERA_MESH_REMOTE_ENDPOINT|CHIMERA_MESH_REMOTE_REGION|CHIMERA_MESH_REMOTE_LOAD_SCORE|CHIMERA_MESH_REMOTE_RELIABILITY_SCORE|CHIMERA_PEER_UPDATE_BASE_URL|CHIMERA_PEER_UPDATE_LISTEN)
      return 0
      ;;
  esac
  return 1
}

bootstrap_env_rhs_is_safe_data() {
  local raw="${1:-}" char
  if printf '%s' "$raw" | LC_ALL=C grep -q '[[:cntrl:]]'; then
    return 1
  fi
  while [[ -n "$raw" ]]; do
    char="${raw:0:1}"
    raw="${raw:1}"
    if [[ "$char" == "\\" ]]; then
      [[ -n "$raw" ]] || return 1
      raw="${raw:1}"
      continue
    fi
    case "$char" in
      '$'|'`'|'|'|'&'|'('|')'|'<'|'>'|'{'|'}'|';')
        return 1
        ;;
    esac
  done
  return 0
}

validate_safe_env_file_for_source() {
  local file="${1:?file_required}"
  local line key rhs
  declare -A seen=()
  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ -z "$(trim_ascii "$line")" || "$line" == \#* ]] && continue
    [[ "$line" == *=* ]] || return 1
    key="${line%%=*}"
    rhs="${line#*=}"
    [[ "$key" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || return 1
    [[ -z "${seen[$key]+x}" ]] || return 1
    seen["$key"]=1
    bootstrap_env_rhs_is_safe_data "$rhs" || return 1
  done <"$file"
}

seed_bootstrap_env_value_if_absent() {
  local key="${1:?key_required}"
  local value="${2:-}"
  local existing=""
  [[ -n "$value" ]] || return 0
  existing="$(read_existing_env_kv_from_file "$BOOTSTRAP_ENV_FILE" "$key")"
  if grep -q "^${key}=" "$BOOTSTRAP_ENV_FILE" 2>/dev/null; then
    return 0
  fi
  upsert_env_kv "$BOOTSTRAP_ENV_FILE" "$key" "$value"
}

validate_bootstrap_env_file_for_load() {
  local file="${1:?file_required}"
  local line key rhs
  declare -A seen=()
  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ -z "$(trim_ascii "$line")" || "$line" == \#* ]] && continue
    [[ "$line" == *=* ]] || return 1
    key="${line%%=*}"
    rhs="${line#*=}"
    bootstrap_env_key_allowed "$key" || return 1
    [[ -z "${seen[$key]+x}" ]] || return 1
    seen["$key"]=1
    bootstrap_env_rhs_is_safe_data "$rhs" || return 1
  done <"$file"
}

load_bootstrap_env_if_present() {
  [[ -f "$BOOTSTRAP_ENV_FILE" ]] || return 0
  if ! validate_bootstrap_env_file_for_load "$BOOTSTRAP_ENV_FILE"; then
    echo "error: invalid bootstrap env: $BOOTSTRAP_ENV_FILE" >&2
    return 2
  fi
  local key value
  while IFS= read -r key || [[ -n "$key" ]]; do
    value="$(read_existing_env_kv_from_file "$BOOTSTRAP_ENV_FILE" "$key")"
    if grep -q "^${key}=" "$BOOTSTRAP_ENV_FILE" 2>/dev/null; then
      printf -v "$key" '%s' "$value"
      export "$key"
    fi
  done <<'EOF'
CHIMERA_MESH_NODES_DISCOVERY_URL
CHIMERA_MESH_NODES_DISCOVERY_URLS
CHIMERA_MESH_NODES_DISCOVERY_PUBKEY
CHIMERA_MESH_NODES_DISCOVERY_KEYRING
CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS
CHIMERA_MESH_NAMESPACE
CHIMERA_MESH_LOCAL_NODE
CHIMERA_MESH_POLICY_PAYLOAD
CHIMERA_MESH_TRAFFIC_PROFILE
CHIMERA_MESH_REMOTE_PEER_SPEC
CHIMERA_MESH_EXTRA_PEERS
CHIMERA_MESH_REMOTE_NODE
CHIMERA_MESH_REMOTE_ENDPOINT
CHIMERA_MESH_REMOTE_REGION
CHIMERA_MESH_REMOTE_LOAD_SCORE
CHIMERA_MESH_REMOTE_RELIABILITY_SCORE
CHIMERA_PEER_UPDATE_BASE_URL
CHIMERA_PEER_UPDATE_LISTEN
EOF
}

decode_existing_env_rhs() {
  local key="${1:?key_required}"
  local raw="${2:-}"
  [[ -n "$raw" ]] || return 0
  if printf '%s' "$raw" | LC_ALL=C grep -q '[[:cntrl:]]'; then
    echo "error: invalid control character in env value: $key" >&2
    exit 2
  fi
  local out="" char rest
  while [[ -n "$raw" ]]; do
    char="${raw:0:1}"
    raw="${raw:1}"
    if [[ "$char" == "\\" ]]; then
      [[ -n "$raw" ]] || {
        echo "error: dangling escape in env value: $key" >&2
        exit 2
      }
      rest="${raw:0:1}"
      raw="${raw:1}"
      out+="$rest"
    else
      case "$char" in
        '$'|'`'|'|'|'&'|'('|')'|'<'|'>'|'{'|'}')
          echo "error: unsupported shell syntax in env value: $key" >&2
          exit 2
          ;;
      esac
      out+="$char"
    fi
  done
  printf '%s' "$out"
}

pid_cmdline_contains() {
  local pid="${1:-}"
  local needle="${2:-}"
  [[ -n "$pid" && -n "$needle" ]] || return 1
  [[ -r "/proc/$pid/cmdline" ]] || return 1
  tr '\0' ' ' <"/proc/$pid/cmdline" | grep -Fq -- "$needle"
}

runner_cmdline_needle_for_target() {
  local target="${1:?target_required}"
  printf '%s %s' "$(basename "${CHIMERA_RUNNER:-$ROOT_DIR/scripts/chimera-runner.sh}")" "$target"
}

pidfile_running() {
  local pidfile="${1:?pidfile_required}"
  local pid=""
  [[ -f "$pidfile" ]] || return 1
  pid="$(tr -d '[:space:]' <"$pidfile" 2>/dev/null || true)"
  [[ -n "$pid" ]] || return 1
  kill -0 "$pid" >/dev/null 2>&1 || return 1
  case "$pidfile" in
    *chimera-peer-egress.pid)
      pid_cmdline_contains "$pid" "$(runner_cmdline_needle_for_target peer-egress)"
      ;;
    *chimera-transparent-runtime.pid)
      pid_cmdline_contains "$pid" "$(runner_cmdline_needle_for_target transparent-runtime)"
      ;;
    *chimera-peer-update.pid)
      pid_cmdline_contains "$pid" "$(runner_cmdline_needle_for_target peer-update)"
      ;;
    *)
      return 0
      ;;
  esac
}

runner_started() {
  local pidfile="${1:?pidfile_required}"
  local attempts="${2:-10}"
  local i=0
  while (( i < attempts )); do
    if pidfile_running "$pidfile"; then
      return 0
    fi
    sleep 0.1
    i=$((i + 1))
  done
  return 1
}

peer_egress_pid_path() {
  printf '%s' "${PEER_EGRESS_PID_FILE:-${XDG_RUNTIME_DIR:-/tmp}/chimera-peer-egress.pid}"
}

transparent_runtime_pid_path() {
  printf '%s' "${TRANSPARENT_RUNTIME_PID_FILE:-${XDG_RUNTIME_DIR:-/tmp}/chimera-transparent-runtime.pid}"
}

peer_update_pid_path() {
  printf '%s' "${PEER_UPDATE_PID_FILE:-${XDG_RUNTIME_DIR:-/tmp}/chimera-peer-update.pid}"
}

peer_egress_state_path() {
  printf '%s' "${PEER_EGRESS_STATE_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/peer-egress.state}"
}

mesh_discovery_out_path() {
  printf '%s' "${MESH_DISCOVERY_OUT_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/mesh_nodes.discovery.json}"
}

mesh_discovery_pubkey_out_path() {
  printf '%s' "${MESH_DISCOVERY_PUBKEY_OUT_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/mesh_nodes.discovery.pubkey}"
}

mesh_discovery_source_present() {
  [[ -n "$(trim_ascii "${CHIMERA_MESH_NODES_DISCOVERY_URL:-}")" ]] && return 0
  [[ -n "$(trim_ascii "${CHIMERA_MESH_NODES_DISCOVERY_URLS:-}")" ]] && return 0
  [[ -s "${MESH_DISCOVERY_URLS_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/mesh_nodes_discovery_urls.list}" ]] && return 0
  return 1
}

clear_mesh_discovery_snapshot_runtime_state() {
  rm -f "$(mesh_discovery_out_path)" "$(mesh_discovery_pubkey_out_path)" >/dev/null 2>&1 || true
}

mesh_discovery_snapshot_publish_skip() {
  local strict="${1:?strict_required}"
  local reason="${2:?reason_required}"
  local exit_code="${3:-1}"
  clear_mesh_discovery_snapshot_runtime_state
  printf '%s\n' "discovery_snapshot_publish=skipped reason=${reason}" >&2
  if [[ "$strict" == "1" ]]; then
    return "$exit_code"
  fi
  return 0
}

publish_mesh_discovery_snapshot() {
  local strict_publish=0
  case "${1:-best-effort}" in
    best-effort|--best-effort|"") strict_publish=0 ;;
    strict|--strict) strict_publish=1 ;;
    *)
      echo "error: discovery publish mode must be strict|best-effort" >&2
      return 2
      ;;
  esac
  local state_path
  if [[ -f "$BOOTSTRAP_ENV_FILE" ]]; then
    load_bootstrap_env_if_present || return $?
  fi
  state_path="$(peer_egress_state_path)"
  [[ -f "$state_path" ]] || {
    mesh_discovery_snapshot_publish_skip "$strict_publish" "peer_state_missing"
    return $?
  }
  local discovery_out
  discovery_out="$(mesh_discovery_out_path)"
  local pubkey_out
  pubkey_out="$(mesh_discovery_pubkey_out_path)"
  local advertise_invite_token=""
  local self_node_id=""
  if [[ -f "$STATE_FILE" ]]; then
    self_node_id="$(awk -F= '/^mesh_node[[:space:]]*=/{print $2; exit}' "$STATE_FILE" 2>/dev/null || true)"
    if [[ -z "$self_node_id" ]]; then
      self_node_id="$(awk -F= '/^selected_node[[:space:]]*=/{print $2; exit}' "$STATE_FILE" 2>/dev/null || true)"
    fi
  fi
  if [[ -z "$self_node_id" && -f "$UPSTREAM_ENV_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$UPSTREAM_ENV_FILE"
    self_node_id="${CHIMERA_MESH_SELF_NODE_ID:-}"
  fi
  self_node_id="${self_node_id:-$(hostname -s 2>/dev/null || hostname 2>/dev/null || echo chimera-node)}"
  advertise_invite_token="$(trim_ascii_line "$(read_peer_egress_env_kv CHIMERA_PEER_EGRESS_TOKEN)")"
  if ! wait_for_file "$state_path" 5; then
    mesh_discovery_snapshot_publish_skip "$strict_publish" "peer_state_unready"
    return $?
  fi
  local advertise_args=(
    mesh nodes advertise
    --skip-discovery
    --state-file "$state_path"
    --out "$discovery_out"
    --pubkey-out "$pubkey_out"
  )
  if [[ -n "${CHIMERA_MESH_LOCAL_NODE_REGION:-}" ]]; then
    advertise_args+=(--region "${CHIMERA_MESH_LOCAL_NODE_REGION}")
  fi
  if [[ -f "$PEER_UPDATE_STATE_FILE" ]]; then
    advertise_args+=(--update-state-file "$PEER_UPDATE_STATE_FILE")
  fi
  CHIMERA_MESH_PEER_EGRESS_STATE_PATH="$state_path" \
  CHIMERA_MESH_ADVERTISE_INVITE_TOKEN="$advertise_invite_token" \
  CHIMERA_MESH_NODES_DISCOVERY_KEYRING="${CHIMERA_MESH_NODES_DISCOVERY_KEYRING:-}" \
  CHIMERA_MESH_NODES_DISCOVERY_PUBKEY="${CHIMERA_MESH_NODES_DISCOVERY_PUBKEY:-}" \
  CHIMERA_PEER_UPDATE_STATE_FILE="$PEER_UPDATE_STATE_FILE" \
  CHIMERA_MESH_SELF_NODE_ID="$self_node_id" \
  "$CHIMERA_RUNNER" cli "${advertise_args[@]}" >/dev/null 2>&1 || {
    clear_mesh_discovery_snapshot_runtime_state
    if ! mesh_discovery_source_present; then
      mesh_discovery_snapshot_publish_skip "$strict_publish" "discovery_source_not_configured" 0
      return 0
    fi
    return 1
  }
  if [[ ! -s "$discovery_out" || ! -s "$pubkey_out" ]]; then
    mesh_discovery_snapshot_publish_skip "$strict_publish" "generated_file_missing"
    return $?
  fi
  echo "discovery_snapshot_out=$discovery_out"
  echo "discovery_snapshot_pubkey=$pubkey_out"
}

clear_peer_egress_transit_lane_bindings_runtime_state() {
  local lane_file="${1:-}"
  if [[ -z "$lane_file" ]]; then
    lane_file="$(peer_egress_transit_lane_bindings_file)"
  fi
  [[ -n "$lane_file" ]] && rm -f "$lane_file" >/dev/null 2>&1 || true
  [[ -f "$PEER_EGRESS_ENV_FILE" ]] || return 0
  remove_env_kv_from_file "$PEER_EGRESS_ENV_FILE" "CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE"
}

peer_egress_transit_lane_bindings_publish_skip() {
  local strict="${1:?strict_required}"
  local reason="${2:?reason_required}"
  local exit_code="${3:-1}"
  local suffix="${4:-}"
  local cleanup_path="${5:-}"
  clear_peer_egress_transit_lane_bindings_runtime_state "$cleanup_path"
  printf '%s\n' "peer_egress_transit_lane_bindings_publish=skipped reason=${reason}${suffix}" >&2
  if [[ "$strict" == "1" ]]; then
    return "$exit_code"
  fi
  return 0
}

validate_mesh_control_plane_env_file_for_source() {
  local file="${1:?file_required}"
  local line key rhs
  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ -z "$(trim_ascii "$line")" || "$line" == \#* ]] && continue
    [[ "$line" == *=* ]] || return 1
    key="${line%%=*}"
    rhs="${line#*=}"
    case "$key" in
      CHIMERA_MESH_NAMESPACE|CHIMERA_MESH_LOCAL_NODE|CHIMERA_MESH_POLICY_PAYLOAD|CHIMERA_MESH_TRAFFIC_PROFILE|CHIMERA_MESH_REMOTE_PEER_SPEC|CHIMERA_MESH_EXTRA_PEERS) ;;
      *) return 1 ;;
    esac
    if printf '%s' "$rhs" | LC_ALL=C grep -q '[[:cntrl:]]'; then
      return 1
    fi
    case "$rhs" in
      *'$('*|*'${'*|*'`'*) return 1 ;;
    esac
    if printf '%s' "$rhs" | grep -Eq '(^|[^\\])[;|&<>]'; then
      return 1
    fi
  done <"$file"
}

publish_peer_egress_transit_lane_bindings_from_control_plane() {
  local strict_publish=0
  case "${1:-best-effort}" in
    best-effort|--best-effort|"") strict_publish=0 ;;
    strict|--strict) strict_publish=1 ;;
    *)
      echo "error: publish transit lane bindings mode must be strict|best-effort" >&2
      return 2
      ;;
  esac
  local control_plane_env_file existing_bindings_file existing_allow_bound_transit
  local namespace local_node policy_payload traffic_profile out_file
  local -a peer_args route_args
  local peer_spec rc

  control_plane_env_file="${CHIMERA_MESH_CONTROL_PLANE_ENV_FILE:-${CHIMERA_MESH_PRELAUNCH_ENV_FILE:-${CHIMERA_MESH_LAUNCH_ENV_FILE:-$MESH_CONTROL_PLANE_ENV_FILE}}}"
  if [[ -n "$control_plane_env_file" && -f "$control_plane_env_file" ]]; then
    if ! validate_mesh_control_plane_env_file_for_source "$control_plane_env_file"; then
      peer_egress_transit_lane_bindings_publish_skip "$strict_publish" "invalid_control_plane_env"
      return $?
    fi
    # shellcheck disable=SC1090
    source "$control_plane_env_file"
  fi
  if ! mesh_control_plane_has_preflight_env && [[ -f "$BOOTSTRAP_ENV_FILE" ]]; then
    load_bootstrap_env_if_present || {
      peer_egress_transit_lane_bindings_publish_skip "$strict_publish" "invalid_bootstrap_env" 2
      return $?
    }
  fi

  existing_bindings_file=""
  existing_allow_bound_transit=""
  if [[ -f "$PEER_EGRESS_ENV_FILE" ]]; then
    existing_bindings_file="$(awk -F= '
      index($0, "CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE=") == 1 {
        print substr($0, length("CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE=") + 1)
        exit
      }
    ' "$PEER_EGRESS_ENV_FILE" 2>/dev/null || true)"
    existing_allow_bound_transit="$(awk -F= '
      index($0, "CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=") == 1 {
        print substr($0, length("CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=") + 1)
        exit
      }
    ' "$PEER_EGRESS_ENV_FILE" 2>/dev/null || true)"
  fi
  existing_bindings_file="$(trim_ascii "$existing_bindings_file")"
  existing_allow_bound_transit="$(trim_ascii "$existing_allow_bound_transit")"
  if [[ -n "$existing_bindings_file" && "$strict_publish" -eq 0 && -s "$existing_bindings_file" && ! mesh_control_plane_has_preflight_env ]]; then
    printf '%s\n' "peer_egress_transit_lane_bindings_publish=skipped reason=already_configured" >&2
    return 0
  fi
  if [[ ! -f "$PEER_EGRESS_ENV_FILE" ]]; then
    peer_egress_transit_lane_bindings_publish_skip "$strict_publish" "peer_env_missing"
    return $?
  fi

  namespace="$(trim_ascii "${CHIMERA_MESH_NAMESPACE:-}")"
  local_node="$(trim_ascii "${CHIMERA_MESH_LOCAL_NODE:-}")"
  policy_payload="$(trim_ascii "${CHIMERA_MESH_POLICY_PAYLOAD:-}")"
  traffic_profile="$(trim_ascii "${CHIMERA_MESH_TRAFFIC_PROFILE:-}")"
  if [[ -z "$namespace" || -z "$local_node" ]]; then
    peer_egress_transit_lane_bindings_publish_skip "$strict_publish" "missing_authoritative_mesh_context"
    return $?
  fi
  if [[ -z "$policy_payload" && -z "$traffic_profile" ]]; then
    peer_egress_transit_lane_bindings_publish_skip "$strict_publish" "missing_authoritative_policy"
    return $?
  fi
  if [[ "${existing_allow_bound_transit:-${CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT:-false}}" != "true" ]]; then
    peer_egress_transit_lane_bindings_publish_skip "$strict_publish" "bound_transit_disabled"
    return $?
  fi

  peer_args=()
  if [[ -n "${CHIMERA_MESH_REMOTE_PEER_SPEC:-}" ]]; then
    peer_spec="$(trim_ascii "${CHIMERA_MESH_REMOTE_PEER_SPEC:-}")"
    [[ -n "$peer_spec" ]] && peer_args+=("$peer_spec")
  fi
  if [[ -n "${CHIMERA_MESH_EXTRA_PEERS:-}" ]]; then
    while IFS= read -r peer_spec; do
      peer_spec="$(trim_ascii "$peer_spec")"
      [[ -z "$peer_spec" ]] && continue
      peer_args+=("$peer_spec")
    done < <(printf '%s' "${CHIMERA_MESH_EXTRA_PEERS:-}" | tr ',\n' '\n\n')
  fi
  if [[ "${#peer_args[@]}" -eq 0 && -n "${CHIMERA_MESH_REMOTE_NODE:-}" && -n "${CHIMERA_MESH_REMOTE_ENDPOINT:-}" && -n "${CHIMERA_MESH_REMOTE_REGION:-}" && -n "${CHIMERA_MESH_REMOTE_LOAD_SCORE:-}" && -n "${CHIMERA_MESH_REMOTE_RELIABILITY_SCORE:-}" ]]; then
    peer_args+=("${CHIMERA_MESH_REMOTE_NODE}@${CHIMERA_MESH_REMOTE_ENDPOINT}@${CHIMERA_MESH_REMOTE_REGION}@${CHIMERA_MESH_REMOTE_LOAD_SCORE}@${CHIMERA_MESH_REMOTE_RELIABILITY_SCORE}")
  fi
  if [[ "${#peer_args[@]}" -eq 0 ]]; then
    peer_egress_transit_lane_bindings_publish_skip "$strict_publish" "missing_authoritative_peer_list"
    return $?
  fi

  out_file="${CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/peer-egress-transit-lane-bindings.csv}"
  ensure_parent_dir "$out_file"

  route_args=(
    mesh route-explain
    --namespace "$namespace"
    --node "$local_node"
  )
  if [[ -n "$policy_payload" ]]; then
    route_args+=(--policy-payload "$policy_payload")
  else
    route_args+=(--traffic-profile "$traffic_profile")
  fi
  for peer_spec in "${peer_args[@]}"; do
    route_args+=(--peer "$peer_spec")
  done
  route_args+=(--transit-lane-bindings-out "$out_file")

  if run_chimera_cli "${route_args[@]}" >/dev/null 2>&1; then
    if [[ -s "$out_file" ]]; then
      upsert_env_kv "$PEER_EGRESS_ENV_FILE" "CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE" "$out_file"
      printf '%s\n' "peer_egress_transit_lane_bindings_publish=ok" >&2
      return 0
    fi
    peer_egress_transit_lane_bindings_publish_skip "$strict_publish" "generated_file_missing" 1 "" "$out_file"
    return $?
  else
    rc=$?
  fi

  peer_egress_transit_lane_bindings_publish_skip "$strict_publish" "control_plane_derivation_failed" "$rc" " exit=$rc" "$out_file"
  return $?
}

mesh_control_plane_has_preflight_env() {
  [[ -n "${CHIMERA_MESH_NAMESPACE:-}" ]] && return 0
  [[ -n "${CHIMERA_MESH_LOCAL_NODE:-}" ]] && return 0
  [[ -n "${CHIMERA_MESH_POLICY_PAYLOAD:-}" ]] && return 0
  [[ -n "${CHIMERA_MESH_TRAFFIC_PROFILE:-}" ]] && return 0
  [[ -n "${CHIMERA_MESH_REMOTE_PEER_SPEC:-}" ]] && return 0
  [[ -n "${CHIMERA_MESH_REMOTE_NODE:-}" ]] && return 0
  [[ -n "${CHIMERA_MESH_REMOTE_ENDPOINT:-}" ]] && return 0
  [[ -n "${CHIMERA_MESH_EXTRA_PEERS:-}" ]] && return 0
  return 1
}

mesh_control_plane_env_from_preflight() {
  local strict="${1:?strict_required}"
  local control_plane_env_file writer output rc
  control_plane_env_file="${CHIMERA_MESH_CONTROL_PLANE_ENV_FILE:-${CHIMERA_MESH_PRELAUNCH_ENV_FILE:-${CHIMERA_MESH_LAUNCH_ENV_FILE:-$MESH_CONTROL_PLANE_ENV_FILE}}}"
  writer="$ROOT_DIR/scripts/mesh_control_plane_env_from_preflight.sh"
  if [[ ! -x "$writer" ]]; then
    echo "mesh_control_plane_env=skipped reason=writer_missing" >&2
    [[ "$strict" == "1" ]] && return 1
    return 0
  fi
  if ! mesh_control_plane_has_preflight_env; then
    if [[ "$strict" != "1" && -f "$control_plane_env_file" ]]; then
      echo "mesh_control_plane_env=skipped reason=existing_control_plane_env"
      return 0
    fi
    echo "mesh_control_plane_env=skipped reason=missing_authoritative_mesh_context" >&2
    [[ "$strict" == "1" ]] && return 1
    return 0
  fi

  rc=0
  output="$(CHIMERA_MESH_CONTROL_PLANE_ENV_FILE="$control_plane_env_file" "$writer" "$control_plane_env_file" 2>&1)" || rc=$?
  [[ -n "$output" ]] && printf '%s\n' "$output"
  if [[ "$rc" -ne 0 ]]; then
    [[ "$strict" == "1" ]] && return "$rc"
    return 0
  fi
  if [[ "$output" == *"mesh_control_plane_env=ok"* ]]; then
    return 0
  fi
  [[ "$strict" == "1" ]] && return 1
  return 0
}

mesh_control_plane_context_ready_from_current_env() {
  local namespace="" local_node="" policy_payload="" traffic_profile="" remote_peer_spec="" extra_peers=""
  namespace="$(trim_ascii "${CHIMERA_MESH_NAMESPACE:-}")"
  local_node="$(trim_ascii "${CHIMERA_MESH_LOCAL_NODE:-}")"
  policy_payload="$(trim_ascii "${CHIMERA_MESH_POLICY_PAYLOAD:-}")"
  traffic_profile="$(trim_ascii "${CHIMERA_MESH_TRAFFIC_PROFILE:-}")"
  remote_peer_spec="$(trim_ascii "${CHIMERA_MESH_REMOTE_PEER_SPEC:-}")"
  extra_peers="$(trim_ascii "${CHIMERA_MESH_EXTRA_PEERS:-}")"
  [[ -n "$namespace" && -n "$local_node" ]] || return 1
  [[ -n "$policy_payload" || -n "$traffic_profile" ]] || return 1
  if [[ -n "$remote_peer_spec" || -n "$extra_peers" ]]; then
    return 0
  fi
  [[ -n "${CHIMERA_MESH_REMOTE_NODE:-}" && -n "${CHIMERA_MESH_REMOTE_ENDPOINT:-}" && -n "${CHIMERA_MESH_REMOTE_REGION:-}" && -n "${CHIMERA_MESH_REMOTE_LOAD_SCORE:-}" && -n "${CHIMERA_MESH_REMOTE_RELIABILITY_SCORE:-}" ]]
}

bootstrap_control_plane_context_ready() {
  [[ -f "$BOOTSTRAP_ENV_FILE" ]] || return 1
  load_bootstrap_env_if_present || return $?
  mesh_control_plane_context_ready_from_current_env
}

mesh_bind_control_plane() {
  local strict=1
  local preflight_context_ready=0
  case "${1:-}" in
    ""|--strict|strict) strict=1 ;;
    --best-effort|best-effort) strict=0 ;;
    *)
      echo "error: mesh-bind-control-plane accepts --strict or --best-effort" >&2
      return 2
      ;;
  esac
  if mesh_control_plane_context_ready_from_current_env; then
    preflight_context_ready=1
  fi
  if [[ "$preflight_context_ready" -eq 0 ]] && bootstrap_control_plane_context_ready; then
    :
  else
    mesh_control_plane_env_from_preflight "$strict"
  fi
  publish_peer_egress_transit_lane_bindings_from_control_plane "$([[ "$strict" == "1" ]] && echo strict || echo best-effort)"
}

ensure_mesh_bootstrap_env() {
  mkdir -p "$(dirname "$BOOTSTRAP_ENV_FILE")"
  touch "$BOOTSTRAP_ENV_FILE"
  chmod 600 "$BOOTSTRAP_ENV_FILE" 2>/dev/null || true
  if [[ -f "$ROOT_DIR/configs/mesh_bootstrap.env.example" ]]; then
    local discovery_url discovery_pubkey discovery_keyring discovery_probe_timeout
    discovery_url="$(awk -F= '/^CHIMERA_MESH_NODES_DISCOVERY_URL=/{print $2; exit}' "$ROOT_DIR/configs/mesh_bootstrap.env.example" 2>/dev/null || true)"
    discovery_urls="$(awk -F= '/^CHIMERA_MESH_NODES_DISCOVERY_URLS=/{print $2; exit}' "$ROOT_DIR/configs/mesh_bootstrap.env.example" 2>/dev/null || true)"
    discovery_pubkey="$(awk -F= '/^CHIMERA_MESH_NODES_DISCOVERY_PUBKEY=/{print $2; exit}' "$ROOT_DIR/configs/mesh_bootstrap.env.example" 2>/dev/null || true)"
    discovery_keyring="$(awk -F= '/^CHIMERA_MESH_NODES_DISCOVERY_KEYRING=/{print $2; exit}' "$ROOT_DIR/configs/mesh_bootstrap.env.example" 2>/dev/null || true)"
    discovery_probe_timeout="$(awk -F= '/^CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS=/{print $2; exit}' "$ROOT_DIR/configs/mesh_bootstrap.env.example" 2>/dev/null || true)"
    seed_bootstrap_env_value_if_absent "CHIMERA_MESH_NODES_DISCOVERY_URL" "$discovery_url"
    seed_bootstrap_env_value_if_absent "CHIMERA_MESH_NODES_DISCOVERY_URLS" "$discovery_urls"
    seed_bootstrap_env_value_if_absent "CHIMERA_MESH_NODES_DISCOVERY_PUBKEY" "$discovery_pubkey"
    seed_bootstrap_env_value_if_absent "CHIMERA_MESH_NODES_DISCOVERY_KEYRING" "$discovery_keyring"
    seed_bootstrap_env_value_if_absent "CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS" "$discovery_probe_timeout"
  fi
}

bootstrap_template_env_value() {
  local key="${1:?key_required}"
  local example_file="$ROOT_DIR/configs/mesh_bootstrap.env.example"
  [[ -f "$example_file" ]] || return 0
  awk -F= -v key="$key" '
    $0 ~ "^[[:space:]]*" key "=" {
      value = substr($0, index($0, "=") + 1)
      sub(/^[[:space:]]+/, "", value)
      sub(/[[:space:]]+$/, "", value)
      print value
      exit
    }
  ' "$example_file" 2>/dev/null || true
}

default_mesh_local_node() {
  local value=""
  if [[ -f "$STATE_FILE" ]]; then
    value="$(awk -F= '/^mesh_node[[:space:]]*=/{print $2; exit}' "$STATE_FILE" 2>/dev/null || true)"
    if [[ -z "$value" ]]; then
      value="$(awk -F= '/^selected_node[[:space:]]*=/{print $2; exit}' "$STATE_FILE" 2>/dev/null || true)"
    fi
  fi
  if [[ -z "$value" && -f "$UPSTREAM_ENV_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$UPSTREAM_ENV_FILE"
    value="${CHIMERA_MESH_SELF_NODE_ID:-}"
  fi
  value="$(trim_ascii "${value:-${CHIMERA_MESH_SELF_NODE_ID:-}}")"
  if [[ -z "$value" ]]; then
    value="$(hostname -s 2>/dev/null || hostname 2>/dev/null || echo chimera-node)"
  fi
  printf '%s\n' "$value"
}

selected_mesh_remote_peer_spec_from_inventory() {
  local -a mesh_nodes_args=()
  local peer_spec="" best_node_id=""
  if [[ -f "$BOOTSTRAP_ENV_FILE" ]]; then
    load_bootstrap_env_if_present || return $?
  fi
  if [[ -n "${CHIMERA_MESH_NODES_DISCOVERY_URL:-}" ]]; then
    mesh_nodes_args+=(--discovery-url "$CHIMERA_MESH_NODES_DISCOVERY_URL")
  fi
  if ! mesh_discovery_source_present; then
    return 1
  fi
  if [[ -n "${CHIMERA_MESH_NODES_DISCOVERY_PUBKEY:-}" ]]; then
    mesh_nodes_args+=(--discovery-pubkey "$CHIMERA_MESH_NODES_DISCOVERY_PUBKEY")
  fi
  if [[ -n "${CHIMERA_MESH_NODES_DISCOVERY_KEYRING:-}" ]]; then
    mesh_nodes_args+=(--discovery-keyring "$CHIMERA_MESH_NODES_DISCOVERY_KEYRING")
  fi
  mesh_nodes_args+=(--probe-timeout-ms "${CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS:-4000}")
  if run_chimera_cli mesh nodes select "${mesh_nodes_args[@]}" >/dev/null 2>&1; then
    peer_spec="$(run_chimera_cli mesh nodes selected-peer-spec "${mesh_nodes_args[@]}" 2>/dev/null | head -n1 | tr -d '\r\n' || true)"
  fi
  if [[ -z "$peer_spec" ]]; then
    best_node_id="$(run_chimera_cli mesh nodes best "${mesh_nodes_args[@]}" 2>/dev/null | sed -n 's/^node_id=\([^[:space:]]*\).*/\1/p' | head -n1 | tr -d '[:space:]' || true)"
    if [[ -n "$best_node_id" ]]; then
      run_chimera_cli mesh nodes select --id "$best_node_id" "${mesh_nodes_args[@]}" >/dev/null 2>&1 || true
      peer_spec="$(run_chimera_cli mesh nodes selected-peer-spec "${mesh_nodes_args[@]}" 2>/dev/null | head -n1 | tr -d '\r\n' || true)"
    fi
  fi
  peer_spec="$(trim_ascii "$peer_spec")"
  [[ -n "$peer_spec" ]] || return 1
  printf '%s\n' "$peer_spec"
}

seed_mesh_control_plane_authority_from_bootstrap() {
  local strict_seed=0
  case "${1:-best-effort}" in
    best-effort|--best-effort|"") strict_seed=0 ;;
    strict|--strict) strict_seed=1 ;;
    *)
      echo "error: mesh-seed-control-plane accepts --strict or --best-effort" >&2
      return 2
      ;;
  esac
  ensure_mesh_bootstrap_env
  if [[ -f "$BOOTSTRAP_ENV_FILE" ]]; then
    load_bootstrap_env_if_present || return $?
  fi

  local namespace local_node policy_payload traffic_profile remote_peer_spec
  namespace="$(trim_ascii "${CHIMERA_MESH_NAMESPACE:-$(bootstrap_template_env_value CHIMERA_MESH_NAMESPACE)}")"
  local_node="$(trim_ascii "${CHIMERA_MESH_LOCAL_NODE:-$(bootstrap_template_env_value CHIMERA_MESH_LOCAL_NODE)}")"
  if [[ -z "$local_node" ]]; then
    local_node="$(trim_ascii "$(default_mesh_local_node)")"
  fi
  policy_payload="$(trim_ascii "${CHIMERA_MESH_POLICY_PAYLOAD:-$(bootstrap_template_env_value CHIMERA_MESH_POLICY_PAYLOAD)}")"
  traffic_profile="$(trim_ascii "${CHIMERA_MESH_TRAFFIC_PROFILE:-$(bootstrap_template_env_value CHIMERA_MESH_TRAFFIC_PROFILE)}")"
  remote_peer_spec="$(trim_ascii "${CHIMERA_MESH_REMOTE_PEER_SPEC:-$(bootstrap_template_env_value CHIMERA_MESH_REMOTE_PEER_SPEC)}")"

  if [[ -z "$remote_peer_spec" && -n "${CHIMERA_MESH_REMOTE_NODE:-}" && -n "${CHIMERA_MESH_REMOTE_ENDPOINT:-}" && -n "${CHIMERA_MESH_REMOTE_REGION:-}" && -n "${CHIMERA_MESH_REMOTE_LOAD_SCORE:-}" && -n "${CHIMERA_MESH_REMOTE_RELIABILITY_SCORE:-}" ]]; then
    remote_peer_spec="${CHIMERA_MESH_REMOTE_NODE}@${CHIMERA_MESH_REMOTE_ENDPOINT}@${CHIMERA_MESH_REMOTE_REGION}@${CHIMERA_MESH_REMOTE_LOAD_SCORE}@${CHIMERA_MESH_REMOTE_RELIABILITY_SCORE}"
  fi
  if [[ -z "$remote_peer_spec" ]]; then
    remote_peer_spec="$(selected_mesh_remote_peer_spec_from_inventory 2>/dev/null || true)"
    remote_peer_spec="$(trim_ascii "$remote_peer_spec")"
  fi

  if [[ -n "$namespace" ]]; then
    upsert_env_kv "$BOOTSTRAP_ENV_FILE" "CHIMERA_MESH_NAMESPACE" "$namespace"
    CHIMERA_MESH_NAMESPACE="$namespace"
    export CHIMERA_MESH_NAMESPACE
  fi
  if [[ -n "$local_node" ]]; then
    upsert_env_kv "$BOOTSTRAP_ENV_FILE" "CHIMERA_MESH_LOCAL_NODE" "$local_node"
    CHIMERA_MESH_LOCAL_NODE="$local_node"
    export CHIMERA_MESH_LOCAL_NODE
  fi
  if [[ -n "$policy_payload" ]]; then
    upsert_env_kv "$BOOTSTRAP_ENV_FILE" "CHIMERA_MESH_POLICY_PAYLOAD" "$policy_payload"
    CHIMERA_MESH_POLICY_PAYLOAD="$policy_payload"
    export CHIMERA_MESH_POLICY_PAYLOAD
  elif [[ -n "$traffic_profile" ]]; then
    upsert_env_kv "$BOOTSTRAP_ENV_FILE" "CHIMERA_MESH_TRAFFIC_PROFILE" "$traffic_profile"
    CHIMERA_MESH_TRAFFIC_PROFILE="$traffic_profile"
    export CHIMERA_MESH_TRAFFIC_PROFILE
  fi
  if [[ -n "$remote_peer_spec" ]]; then
    upsert_env_kv "$BOOTSTRAP_ENV_FILE" "CHIMERA_MESH_REMOTE_PEER_SPEC" "$remote_peer_spec"
    CHIMERA_MESH_REMOTE_PEER_SPEC="$remote_peer_spec"
    export CHIMERA_MESH_REMOTE_PEER_SPEC
  fi

  if [[ -z "$namespace" || -z "$local_node" ]]; then
    printf '%s\n' "mesh_control_plane_seed=skipped reason=missing_authoritative_mesh_context" >&2
    [[ "$strict_seed" == "1" ]] && return 1
    return 0
  fi
  if [[ -n "$policy_payload" && -n "$traffic_profile" ]]; then
    printf '%s\n' "mesh_control_plane_seed=skipped reason=ambiguous_authoritative_policy" >&2
    [[ "$strict_seed" == "1" ]] && return 1
    return 0
  fi
  if [[ -z "$policy_payload" && -z "$traffic_profile" ]]; then
    printf '%s\n' "mesh_control_plane_seed=skipped reason=missing_authoritative_policy" >&2
    [[ "$strict_seed" == "1" ]] && return 1
    return 0
  fi
  if [[ -z "$remote_peer_spec" ]]; then
    if mesh_discovery_source_present; then
      printf '%s\n' "mesh_control_plane_seed=ok reason=dynamic_discovery" >&2
      return 0
    fi
    printf '%s\n' "mesh_control_plane_seed=skipped reason=missing_authoritative_peer_list" >&2
    [[ "$strict_seed" == "1" ]] && return 1
    return 0
  fi

  printf '%s\n' "mesh_control_plane_seed=ok" >&2
  return 0
}

upsert_node_config_kv() {
  local file="${1:?file_required}"
  local key="${2:?key_required}"
  local value="${3:-}"
  local key_re tmp_file replaced=0 line
  if printf '%s' "$value" | LC_ALL=C grep -q '[[:cntrl:]]'; then
    echo "error: invalid control character in node config value: $key" >&2
    exit 2
  fi
  key_re="${key//./\\.}"
  mkdir -p "$(dirname "$file")"
  touch "$file"
  tmp_file="$(mktemp)"
  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ "$line" =~ ^[[:space:]]*$key_re[[:space:]]*= ]]; then
      if [[ "$replaced" -eq 0 ]]; then
        printf '%s = %s\n' "$key" "$value"
        replaced=1
      fi
      continue
    fi
    printf '%s\n' "$line"
  done <"$file" >"$tmp_file"
  if [[ "$replaced" -eq 0 ]]; then
    printf '%s = %s\n' "$key" "$value" >>"$tmp_file"
  fi
  cat "$tmp_file" >"$file"
  rm -f "$tmp_file"
}

normalize_node_connect_addr() {
  local endpoint="${1:?endpoint_required}"
  case "$endpoint" in
    tcp://*)
      printf '%s\n' "$endpoint"
      ;;
    tcp@*)
      printf 'tcp://%s\n' "${endpoint#tcp@}"
      ;;
    *@*)
      echo "error: unsupported CHIMERA node endpoint transport" >&2
      exit 2
      ;;
    *)
      printf 'tcp://%s\n' "$endpoint"
      ;;
  esac
}

raw_node_endpoint_host_port() {
  local endpoint="${1:?endpoint_required}"
  case "$endpoint" in
    tcp://*)
      printf '%s\n' "${endpoint#tcp://}"
      ;;
    tcp@*)
      printf '%s\n' "${endpoint#tcp@}"
      ;;
    *)
      printf '%s\n' "$endpoint"
      ;;
  esac
}

mesh_peer_spec_endpoint() {
  local peer_spec="${1:-}"
  local node_id="" endpoint="" region="" load_score="" reliability_score="" extra=""
  peer_spec="$(trim_ascii "$peer_spec")"
  [[ -n "$peer_spec" ]] || return 1
  IFS='@' read -r node_id endpoint region load_score reliability_score extra <<<"$peer_spec"
  [[ -n "$node_id" && -n "$endpoint" && -n "$region" && -n "$load_score" && -n "$reliability_score" && -z "$extra" ]] || return 1
  printf '%s\n' "$endpoint"
}

derive_node_server_name() {
  local host_part="${1:?host_part_required}"
  local server_name="${CHIMERA_NODE_SERVER_NAME:-${CHIMERA_CARRIER_SERVER_NAME:-${CHIMERA_MESH_REMOTE_SERVER_NAME:-$host_part}}}"
  server_name="${server_name#[}"
  server_name="${server_name%]}"
  if [[ -z "$server_name" ]]; then
    echo "error: CHIMERA node server name is empty" >&2
    exit 2
  fi
  printf '%s\n' "$server_name"
}

desired_node_listen_addr() {
  local listen_addr
  listen_addr="${CHIMERA_NODE_LISTEN_ADDR:-${CHIMERA_NODE_PEER_LISTEN_ADDR:-auto}}"
  if [[ -z "$listen_addr" ]]; then
    listen_addr="auto"
  fi
  printf '%s\n' "$listen_addr"
}

read_node_config_kv() {
  local file="${1:?file_required}"
  local key="${2:?key_required}"
  awk -F= -v target="$key" '
    {
      raw_key = $1
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", raw_key)
      if (raw_key != target) {
        next
      }
      value = substr($0, index($0, "=") + 1)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
      print value
      exit
    }
  ' "$file" 2>/dev/null || true
}

node_config_value_is_placeholder() {
  local value="${1:-}"
  value="$(trim_ascii "$value")"
  [[ -z "$value" ]] && return 0
  [[ "$value" == "\${"*"}" ]]
}

node_configured_listen_addr() {
  local node_conf="${1:-$(node_config_path)}"
  local listen_addr=""
  if [[ -f "$node_conf" ]]; then
    listen_addr="$(
      awk -F= '
        /^peer\.listen_addr[[:space:]]*=/ {
          value = substr($0, index($0, "=") + 1)
          sub(/^[[:space:]]+/, "", value)
          sub(/[[:space:]]+$/, "", value)
          print value
          exit
        }
      ' "$node_conf" 2>/dev/null || true
    )"
  fi
  if [[ -z "$listen_addr" ]]; then
    listen_addr="$(desired_node_listen_addr)"
  fi
  if [[ -z "$listen_addr" ]]; then
    listen_addr="auto"
  fi
  printf '%s\n' "$listen_addr"
}

listen_addr_is_auto_like() {
  local listen_addr
  listen_addr="$(trim_ascii "${1:-}")"
  case "$listen_addr" in
    ""|auto|\$\{*|*:0)
      return 0
      ;;
  esac
  return 1
}

normalize_node_peer_listen_value() {
  local listen_addr="${1:-}"
  case "$(trim_ascii "$listen_addr")" in
    ""|auto|\$\{*)
      printf '%s\n' "0.0.0.0:0"
      ;;
    *:*)
      printf '%s\n' "$(trim_ascii "$listen_addr")"
      ;;
    *)
      printf '0.0.0.0:%s\n' "$(trim_ascii "$listen_addr")"
      ;;
  esac
}

listen_addr_port() {
  local listen_addr="${1:-}"
  listen_addr="$(trim_ascii "$listen_addr")"
  [[ "$listen_addr" == *:* ]] || return 1
  local port="${listen_addr##*:}"
  [[ "$port" =~ ^[0-9]+$ ]] || return 1
  printf '%s\n' "$port"
}

tcp_listener_port_in_use() {
  local port="${1:-}"
  local ss_output=""
  local rc=0
  [[ "$port" =~ ^[0-9]+$ ]] || return 1
  if command -v ss >/dev/null 2>&1; then
    set +e
    ss_output="$(ss -H -ltn 2>/dev/null)"
    rc=$?
    set -e
    if [[ "$rc" -le 1 ]]; then
      awk -v port="$port" '
      NF >= 4 {
        endpoint = $4
        if (endpoint ~ /^\[/) {
          sub(/^.*]:/, "", endpoint)
        } else {
          sub(/^.*:/, "", endpoint)
        }
        if (endpoint == port) {
          found = 1
        }
      }
      END {
        exit(found ? 0 : 1)
      }
    ' <<<"$ss_output"
      return $?
    fi
  fi
  if command -v lsof >/dev/null 2>&1; then
    lsof -nP -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1
    return $?
  fi
  if command -v netstat >/dev/null 2>&1; then
    netstat -ltn 2>/dev/null | awk -v port="$port" '
      NR > 2 {
        endpoint = $4
        if (endpoint ~ /^\[/) {
          sub(/^.*]:/, "", endpoint)
        } else {
          sub(/^.*:/, "", endpoint)
        }
        if (endpoint == port) {
          found = 1
        }
      }
      END {
        exit(found ? 0 : 1)
      }
    '
    return $?
  fi
  return 1
}

fixed_listen_addr_port_is_blocked() {
  local listen_addr="${1:-}"
  listen_addr="$(trim_ascii "$listen_addr")"
  listen_addr_is_auto_like "$listen_addr" && return 1
  local port=""
  port="$(listen_addr_port "$listen_addr" 2>/dev/null || true)"
  [[ -n "$port" ]] || return 1
  tcp_listener_port_in_use "$port"
}

node_listener_bindings_need_preemptive_repair() {
  local node_conf configured_listen mode current_local_listen current_peer_listen
  node_conf="$(node_config_path)"
  configured_listen="$(normalize_node_peer_listen_value "$(node_configured_listen_addr "$node_conf")")"
  if fixed_listen_addr_port_is_blocked "$configured_listen"; then
    return 0
  fi
  [[ -f "$PEER_EGRESS_ENV_FILE" ]] || return 1
  mode="$(trim_ascii_line "$(read_peer_egress_env_kv CHIMERA_PEER_EGRESS_MODE)")"
  [[ "$mode" == "node" ]] || return 1
  current_local_listen="$(trim_ascii_line "$(read_peer_egress_env_kv CHIMERA_PEER_EGRESS_LOCAL_LISTEN)")"
  if fixed_listen_addr_port_is_blocked "$current_local_listen"; then
    return 0
  fi
  current_peer_listen="$(trim_ascii_line "$(read_peer_egress_env_kv CHIMERA_PEER_EGRESS_PEER_LISTEN)")"
  fixed_listen_addr_port_is_blocked "$current_peer_listen"
}

configure_peer_egress_dynamic_lanes_from_bootstrap() {
  [[ -f "$PEER_EGRESS_ENV_FILE" ]] || return 0
  local mode="$(trim_ascii_line "$(read_peer_egress_env_kv CHIMERA_PEER_EGRESS_MODE)")"
  [[ "$mode" == "node" ]] || return 0
  if ! mesh_discovery_source_present; then
    return 0
  fi
  local lane_document_path="${CHIMERA_PEER_EGRESS_LANE_DOCUMENT_PATH:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/peer-egress-lane-document.v1}"
  upsert_env_kv "$PEER_EGRESS_ENV_FILE" "CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT" "true"
  upsert_env_kv "$PEER_EGRESS_ENV_FILE" "CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE" "$lane_document_path"
  upsert_env_kv "$PEER_EGRESS_ENV_FILE" "CHIMERA_PEER_EGRESS_LANE_DOCUMENT_PATH" "$lane_document_path"
  remove_env_kv_from_file "$PEER_EGRESS_ENV_FILE" "CHIMERA_PEER_EGRESS_SERVER"
  if [[ -n "${CHIMERA_MESH_NODES_DISCOVERY_URL:-}" ]]; then
    upsert_env_kv "$PEER_EGRESS_ENV_FILE" "CHIMERA_MESH_NODES_DISCOVERY_URL" "$CHIMERA_MESH_NODES_DISCOVERY_URL"
  fi
  if [[ -n "${CHIMERA_MESH_NODES_DISCOVERY_PUBKEY:-}" ]]; then
    upsert_env_kv "$PEER_EGRESS_ENV_FILE" "CHIMERA_MESH_NODES_DISCOVERY_PUBKEY" "$CHIMERA_MESH_NODES_DISCOVERY_PUBKEY"
  fi
  if [[ -n "${CHIMERA_MESH_NODES_DISCOVERY_KEYRING:-}" ]]; then
    upsert_env_kv "$PEER_EGRESS_ENV_FILE" "CHIMERA_MESH_NODES_DISCOVERY_KEYRING" "$CHIMERA_MESH_NODES_DISCOVERY_KEYRING"
  fi
  if [[ -n "${CHIMERA_MESH_NAMESPACE:-}" ]]; then
    upsert_env_kv "$PEER_EGRESS_ENV_FILE" "CHIMERA_MESH_NAMESPACE" "$CHIMERA_MESH_NAMESPACE"
  fi
  if [[ -z "${CHIMERA_MESH_SELF_NODE_ID:-}" && -n "${CHIMERA_MESH_LOCAL_NODE:-}" ]]; then
    CHIMERA_MESH_SELF_NODE_ID="$CHIMERA_MESH_LOCAL_NODE"
  fi
  if [[ -n "${CHIMERA_MESH_SELF_NODE_ID:-}" ]]; then
    upsert_env_kv "$PEER_EGRESS_ENV_FILE" "CHIMERA_MESH_SELF_NODE_ID" "$CHIMERA_MESH_SELF_NODE_ID"
  fi
  if [[ -z "${CHIMERA_MESH_POLICY_PAYLOAD:-}" && -n "${CHIMERA_MESH_TRAFFIC_PROFILE:-}" ]]; then
    CHIMERA_MESH_POLICY_PAYLOAD="mesh_traffic_profile=${CHIMERA_MESH_TRAFFIC_PROFILE};mesh_route_binding_id=1"
  fi
  if [[ -n "${CHIMERA_MESH_POLICY_PAYLOAD:-}" ]]; then
    upsert_env_kv "$PEER_EGRESS_ENV_FILE" "CHIMERA_MESH_POLICY_PAYLOAD" "$CHIMERA_MESH_POLICY_PAYLOAD"
  fi
  return 0
}

heal_node_peer_egress_env_bindings() {
  local mode expected_peer_listen current_peer_listen
  [[ -f "$PEER_EGRESS_ENV_FILE" ]] || return 0
  mode="$(trim_ascii_line "$(read_peer_egress_env_kv CHIMERA_PEER_EGRESS_MODE)")"
  [[ "$mode" == "node" ]] || return 0
  [[ -n "${CHIMERA_PEER_EGRESS_PEER_LISTEN:-}" ]] && return 0
  if mesh_discovery_source_present; then
    expected_peer_listen="0.0.0.0:0"
  else
    expected_peer_listen="$(normalize_node_peer_listen_value "$(node_configured_listen_addr)")"
  fi
  current_peer_listen="$(trim_ascii_line "$(read_peer_egress_env_kv CHIMERA_PEER_EGRESS_PEER_LISTEN)")"
  if [[ "$current_peer_listen" != "$expected_peer_listen" ]]; then
    upsert_env_kv "$PEER_EGRESS_ENV_FILE" "CHIMERA_PEER_EGRESS_PEER_LISTEN" "$expected_peer_listen"
  fi
}

heal_node_carrier_addr_from_peer_egress_env() {
  local node_conf peer_server raw addr_part port_part current_addr expected_addr
  local remote_peer_spec node_id region load_score reliability_score new_spec
  [[ -f "$PEER_EGRESS_ENV_FILE" ]] || return 0
  peer_server="$(trim_ascii_line "$(read_peer_egress_env_kv CHIMERA_PEER_EGRESS_SERVER 2>/dev/null || true)")"
  [[ -n "$peer_server" && "$peer_server" != *\$\{* && "$peer_server" != *example* ]] || return 0
  expected_addr="$(normalize_node_connect_addr "$peer_server")"
  raw="$(raw_node_endpoint_host_port "$peer_server")"
  addr_part="${raw%:*}"
  port_part="${raw##*:}"
  [[ -n "$addr_part" && "$port_part" =~ ^[0-9]+$ && "$port_part" -ge 1 && "$port_part" -le 65535 ]] || return 0
  node_conf="$(node_config_path)"
  [[ -f "$node_conf" ]] || return 0
  current_addr="$(read_node_config_kv "$node_conf" "carrier.addr")"
  if [[ "$current_addr" != "$expected_addr" ]]; then
    upsert_node_config_kv "$node_conf" "carrier.addr" "$expected_addr"
    upsert_node_config_kv "$node_conf" "carrier.server_name" "$addr_part"
    echo "healed_node_carrier_addr=$expected_addr"
  fi
  if [[ -f "$BOOTSTRAP_ENV_FILE" ]]; then
    remote_peer_spec="$(trim_ascii_line "$(read_existing_env_kv_from_file "$BOOTSTRAP_ENV_FILE" CHIMERA_MESH_REMOTE_PEER_SPEC 2>/dev/null || true)")"
    if [[ -n "$remote_peer_spec" ]]; then
      IFS='@' read -r node_id _ region load_score reliability_score <<<"$remote_peer_spec"
      if [[ -n "$node_id" && -n "$region" && -n "$load_score" && -n "$reliability_score" ]]; then
        new_spec="${node_id}@${addr_part}:${port_part}@${region}@${load_score}@${reliability_score}"
        if [[ "$remote_peer_spec" != "$new_spec" ]]; then
          upsert_env_kv "$BOOTSTRAP_ENV_FILE" "CHIMERA_MESH_REMOTE_PEER_SPEC" "$new_spec"
          echo "healed_bootstrap_remote_peer_spec=${new_spec/@port/:${port_part}}"
        fi
      fi
    fi
  fi
}

repair_node_listener_bindings_for_retry() {
  local repaired=1
  local desired_local_override="127.0.0.1:18135"
  local desired_peer_override="0.0.0.0:0"
  local current_local_override current_peer_override
  if ! node_listener_bindings_need_preemptive_repair; then
    return 1
  fi
  current_local_override="$(trim_ascii_line "$(read_runtime_listener_override_kv CHIMERA_PEER_EGRESS_LOCAL_LISTEN)")"
  current_peer_override="$(trim_ascii_line "$(read_runtime_listener_override_kv CHIMERA_PEER_EGRESS_PEER_LISTEN)")"
  if [[ "$current_local_override" != "$desired_local_override" ]]; then
    set_runtime_listener_override_kv "CHIMERA_PEER_EGRESS_LOCAL_LISTEN" "$desired_local_override"
    repaired=0
  fi
  if [[ "$current_peer_override" != "$desired_peer_override" ]]; then
    set_runtime_listener_override_kv "CHIMERA_PEER_EGRESS_PEER_LISTEN" "$desired_peer_override"
    repaired=0
  fi
  if [[ "$repaired" -eq 0 ]]; then
    append_runtime_autofix_log "node_listener_reset" "action=retry_with_auto_listen"
    return 0
  fi
  return 1
}

append_runtime_autofix_log() {
  local event="${1:?event_required}"
  shift || true
  [[ -n "${AUTOFIX_LOG_FILE:-}" ]] || return 0
  ensure_parent_dir "$AUTOFIX_LOG_FILE" >/dev/null 2>&1 || true
  touch "$AUTOFIX_LOG_FILE" 2>/dev/null || true
  {
    printf '%s runtime_repair=%s' "$(date '+%F %T')" "$event"
    while (( $# > 0 )); do
      printf ' %s' "$1"
      shift
    done
    printf '\n'
  } >>"$AUTOFIX_LOG_FILE" 2>/dev/null || true
}

repair_peer_update_listener_for_retry() {
  local current_listen desired_override current_override
  current_listen=""
  if [[ -f "$PEER_UPDATE_ENV_FILE" ]]; then
    current_listen="$(trim_ascii_line "$(read_peer_update_env_kv CHIMERA_PEER_UPDATE_LISTEN)")"
  fi
  if [[ -z "$current_listen" && -f "$BOOTSTRAP_ENV_FILE" ]] \
    && grep -q '^CHIMERA_PEER_UPDATE_LISTEN=' "$BOOTSTRAP_ENV_FILE" 2>/dev/null; then
    current_listen="$(trim_ascii_line "$(read_existing_env_kv_from_file "$BOOTSTRAP_ENV_FILE" CHIMERA_PEER_UPDATE_LISTEN)")"
  fi
  if [[ -z "$current_listen" ]]; then
    current_listen="$(trim_ascii "${CHIMERA_PEER_UPDATE_LISTEN:-}")"
  fi
  if listen_addr_is_auto_like "$current_listen"; then
    return 1
  fi
  desired_override="0.0.0.0:0"
  current_override="$(trim_ascii_line "$(read_runtime_listener_override_kv CHIMERA_PEER_UPDATE_LISTEN)")"
  if [[ "$current_override" == "$desired_override" ]]; then
    return 1
  fi
  set_runtime_listener_override_kv "CHIMERA_PEER_UPDATE_LISTEN" "$desired_override"
  append_runtime_autofix_log "peer_update_listener_reset" "action=retry_with_auto_listen"
  return 0
}

peer_update_listener_needs_preemptive_repair() {
  local current_listen=""
  if [[ -f "$PEER_UPDATE_ENV_FILE" ]]; then
    current_listen="$(trim_ascii_line "$(read_peer_update_env_kv CHIMERA_PEER_UPDATE_LISTEN)")"
  fi
  if [[ -z "$current_listen" && -f "$BOOTSTRAP_ENV_FILE" ]]; then
    load_bootstrap_env_if_present || return 1
    current_listen="$(trim_ascii "${CHIMERA_PEER_UPDATE_LISTEN:-}")"
  fi
  [[ -n "$current_listen" ]] || return 1
  fixed_listen_addr_port_is_blocked "$current_listen"
}

node_service_prestart_self_heal() {
  ensure_base_path
  ensure_mesh_bootstrap_env
  [[ -f "$PEER_EGRESS_ENV_FILE" ]] || {
    echo "error: missing peer-egress env: $PEER_EGRESS_ENV_FILE" >&2
    return 1
  }
  if ! validate_safe_env_file_for_source "$PEER_EGRESS_ENV_FILE"; then
    echo "error: invalid peer-egress env: $PEER_EGRESS_ENV_FILE" >&2
    return 2
  fi
  if [[ -f "$RUNTIME_LISTENER_OVERRIDE_FILE" ]] && ! validate_safe_env_file_for_source "$RUNTIME_LISTENER_OVERRIDE_FILE"; then
    echo "error: invalid runtime listener override env: $RUNTIME_LISTENER_OVERRIDE_FILE" >&2
    return 2
  fi
  if [[ -f "$BOOTSTRAP_ENV_FILE" ]]; then
    load_bootstrap_env_if_present || return $?
  fi
  refresh_node_peer_target_from_bootstrap >/dev/null 2>&1 || true
  configure_peer_egress_dynamic_lanes_from_bootstrap
  heal_node_peer_egress_env_bindings
  heal_node_carrier_addr_from_peer_egress_env >/dev/null 2>&1 || true
  ensure_peer_egress_local_listen_aligned
  ensure_peer_egress_service_fwmark
  ensure_transparent_runtime_service_fwmark
  if node_listener_bindings_need_preemptive_repair; then
    repair_node_listener_bindings_for_retry >/dev/null 2>&1 || true
  else
    clear_node_listener_runtime_overrides || true
  fi
  return 0
}

datapath_service_prestart_validate() {
  ensure_base_path
  [[ -f "$TRANSPARENT_RUNTIME_ENV_FILE" ]] || {
    echo "error: missing transparent-runtime env: $TRANSPARENT_RUNTIME_ENV_FILE" >&2
    return 1
  }
  if ! validate_safe_env_file_for_source "$TRANSPARENT_RUNTIME_ENV_FILE"; then
    echo "error: invalid transparent-runtime env: $TRANSPARENT_RUNTIME_ENV_FILE" >&2
    return 2
  fi
  return 0
}

node_service_poststart_reconcile() {
  if ! datapath_apply_proof_ok; then
    clear_stale_publication_runtime_state
    echo "node_poststart_reconcile=deferred"
    return 0
  fi
  site_auto_watch_run_once >/dev/null 2>&1 || true
  site_auto_watch_start >/dev/null 2>&1 || true
  echo "node_poststart_reconcile=ok"
}

materialize_node_runtime_config() {
  local node_conf="${1:?node_conf_required}"
  local connect_addr="${2:?connect_addr_required}"
  local host_part="${3:?host_part_required}"
  local server_name listen_addr existing_server_name existing_listen_addr final_server_name final_listen_addr
  server_name="$(derive_node_server_name "$host_part")"
  listen_addr="$(desired_node_listen_addr)"
  existing_server_name="$(read_node_config_kv "$node_conf" "carrier.server_name")"
  existing_listen_addr="$(read_node_config_kv "$node_conf" "peer.listen_addr")"
  final_server_name="$server_name"
  final_listen_addr="$listen_addr"
  if [[ -z "${CHIMERA_NODE_SERVER_NAME:-${CHIMERA_CARRIER_SERVER_NAME:-${CHIMERA_MESH_REMOTE_SERVER_NAME:-}}}" ]] \
    && ! node_config_value_is_placeholder "$existing_server_name"; then
    final_server_name="$existing_server_name"
  fi
  if [[ -z "${CHIMERA_NODE_LISTEN_ADDR:-${CHIMERA_NODE_PEER_LISTEN_ADDR:-}}" ]] \
    && ! node_config_value_is_placeholder "$existing_listen_addr"; then
    final_listen_addr="$existing_listen_addr"
  fi
  upsert_node_config_kv "$node_conf" "node.mode" "mesh-node"
  upsert_node_config_kv "$node_conf" "carrier.addr" "$connect_addr"
  upsert_node_config_kv "$node_conf" "carrier.server_name" "$final_server_name"
  upsert_node_config_kv "$node_conf" "peer.listen_addr" "$final_listen_addr"
}

refresh_node_peer_target_from_bootstrap() {
  local node_conf candidate raw_candidate connect_addr host_part port_part
  local -a mesh_nodes_args=()
  node_conf="$(node_config_path)"
  candidate="${CHIMERA_NODE_ENDPOINT:-${CHIMERA_PEER_ENDPOINT:-${CHIMERA_CARRIER_ADDR:-${CHIMERA_MESH_REMOTE_ENDPOINT:-}}}}"
  if [[ -z "$candidate" && -f "$BOOTSTRAP_ENV_FILE" ]]; then
    load_bootstrap_env_if_present || return $?
    candidate="${CHIMERA_NODE_ENDPOINT:-${CHIMERA_PEER_ENDPOINT:-${CHIMERA_CARRIER_ADDR:-${CHIMERA_MESH_REMOTE_ENDPOINT:-}}}}"
    if [[ -z "$candidate" && -n "${CHIMERA_MESH_REMOTE_PEER_SPEC:-}" ]]; then
      candidate="$(mesh_peer_spec_endpoint "${CHIMERA_MESH_REMOTE_PEER_SPEC:-}" 2>/dev/null || true)"
    fi
    if [[ -z "$candidate" ]] && mesh_discovery_source_present; then
      if [[ -n "${CHIMERA_MESH_NODES_DISCOVERY_URL:-}" ]]; then
        mesh_nodes_args+=(--discovery-url "$CHIMERA_MESH_NODES_DISCOVERY_URL")
      fi
      if [[ -n "${CHIMERA_MESH_NODES_DISCOVERY_PUBKEY:-}" ]]; then
        mesh_nodes_args+=(--discovery-pubkey "$CHIMERA_MESH_NODES_DISCOVERY_PUBKEY")
      fi
      if [[ -n "${CHIMERA_MESH_NODES_DISCOVERY_KEYRING:-}" ]]; then
        mesh_nodes_args+=(--discovery-keyring "$CHIMERA_MESH_NODES_DISCOVERY_KEYRING")
      fi
      mesh_nodes_args+=(--probe-timeout-ms "${CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS:-4000}")
      if run_chimera_cli mesh nodes select "${mesh_nodes_args[@]}" >/dev/null 2>&1; then
        candidate="$(run_chimera_cli mesh nodes selected-endpoint "${mesh_nodes_args[@]}" 2>/dev/null | head -n1 | tr -d '[:space:]' || true)"
      fi
      if [[ -z "$candidate" ]]; then
        local best_node_id=""
        best_node_id="$(run_chimera_cli mesh nodes best "${mesh_nodes_args[@]}" 2>/dev/null | sed -n 's/^node_id=\([^[:space:]]*\).*/\1/p' | head -n1 | tr -d '[:space:]' || true)"
        if [[ -n "$best_node_id" ]]; then
          run_chimera_cli mesh nodes select --id "$best_node_id" "${mesh_nodes_args[@]}" >/dev/null 2>&1 || true
          candidate="$(run_chimera_cli mesh nodes selected-endpoint "${mesh_nodes_args[@]}" 2>/dev/null | head -n1 | tr -d '[:space:]' || true)"
        fi
      fi
    fi
  fi
  if [[ -z "$candidate" && -f "$PEER_EGRESS_ENV_FILE" ]]; then
    local peer_server=""
    peer_server="$(trim_ascii_line "$(read_peer_egress_env_kv CHIMERA_PEER_EGRESS_SERVER 2>/dev/null || true)")"
    if [[ -n "$peer_server" && "$peer_server" != *\$\{* && "$peer_server" != *example* ]]; then
      candidate="$peer_server"
    fi
  fi
  [[ -n "$candidate" ]] || return 1

  raw_candidate="$(raw_node_endpoint_host_port "$candidate")"
  connect_addr="$(normalize_node_connect_addr "$candidate")"
  if [[ "$raw_candidate" != *:* ]]; then
    echo "error: invalid CHIMERA node endpoint" >&2
    exit 2
  fi
  host_part="${raw_candidate%:*}"
  port_part="${raw_candidate##*:}"
  if [[ -z "$host_part" || ! "$port_part" =~ ^[0-9]+$ || "$port_part" -lt 1 || "$port_part" -gt 65535 ]]; then
    echo "error: invalid CHIMERA node endpoint" >&2
    exit 2
  fi
  if [[ ! -f "$node_conf" && -f "$ROOT_DIR/configs/mesh-node.example.conf" ]]; then
    cp "$ROOT_DIR/configs/mesh-node.example.conf" "$node_conf"
  fi
  [[ -f "$node_conf" ]] || return 1
  materialize_node_runtime_config "$node_conf" "$connect_addr" "$host_part"
  if [[ -f "$PEER_EGRESS_ENV_FILE" ]]; then
    local peer_mode
    peer_mode="$(trim_ascii_line "$(read_peer_egress_env_kv CHIMERA_PEER_EGRESS_MODE)")"
    if [[ "$peer_mode" == "node" && "${CHIMERA_PEER_EGRESS_SERVER+set}" != "set" ]] && ! mesh_discovery_source_present; then
      upsert_env_kv "$PEER_EGRESS_ENV_FILE" "CHIMERA_PEER_EGRESS_SERVER" "$raw_candidate"
    fi
    if [[ "$peer_mode" == "node" ]] && mesh_discovery_source_present; then
      remove_env_kv_from_file "$PEER_EGRESS_ENV_FILE" "CHIMERA_PEER_EGRESS_SERVER"
    fi
  fi
  printf '%s\n' "$raw_candidate" >"$ROOT_DIR/configs/chimera_runtime_endpoint.txt"
  return 0
}

validate_peer_update_base_url() {
  local url="${1:-}"
  [[ -n "$url" ]] || return 1
  case "$url" in
    http://*|https://*) ;;
    *) return 1 ;;
  esac
  case "$url" in
    *\"*|*"'"*|*\`*|*\$*|*\\*|*@*|*\?*|*#*|*$'\r'*|*$'\n'*|*$'\t'*)
      return 1
      ;;
  esac
  [[ "$url" =~ [[:space:]] ]] && return 1
  return 0
}

configure_peer_update_env() {
  local base_url="${CHIMERA_PEER_UPDATE_BASE_URL:-}"
  local listen="${CHIMERA_PEER_UPDATE_LISTEN:-}"
  local existing_listen=""
  if [[ -z "$base_url" && -f "$BOOTSTRAP_ENV_FILE" ]]; then
    load_bootstrap_env_if_present || return $?
    base_url="${CHIMERA_PEER_UPDATE_BASE_URL:-}"
    if [[ "${CHIMERA_PEER_UPDATE_LISTEN+set}" == "set" ]]; then
      listen="${CHIMERA_PEER_UPDATE_LISTEN:-$listen}"
    fi
  fi
  if [[ -z "$base_url" && -f "$PEER_UPDATE_ENV_FILE" ]]; then
    base_url="$(trim_ascii_line "$(read_existing_env_kv_from_file "$PEER_UPDATE_ENV_FILE" CHIMERA_PEER_UPDATE_BASE_URL 2>/dev/null || true)")"
  fi
  existing_listen="$(trim_ascii_line "$(read_existing_env_kv_from_file "$PEER_UPDATE_ENV_FILE" CHIMERA_PEER_UPDATE_LISTEN)")"
  if [[ "${CHIMERA_PEER_UPDATE_LISTEN+set}" != "set" ]] \
    && ! node_config_value_is_placeholder "$existing_listen"; then
    listen="$existing_listen"
  fi
  base_url="$(trim_ascii "$base_url")"
  listen="$(trim_ascii "${listen:-0.0.0.0:0}")"
  if ! validate_peer_update_base_url "$base_url"; then
    return 1
  fi
  mkdir -p "$(dirname "$PEER_UPDATE_ENV_FILE")"
  mkdir -p "$(dirname "$PEER_UPDATE_STATE_FILE")"
  touch "$PEER_UPDATE_ENV_FILE"
  upsert_env_kv "$PEER_UPDATE_ENV_FILE" "CHIMERA_PEER_UPDATE_BASE_URL" "$base_url"
  upsert_env_kv "$PEER_UPDATE_ENV_FILE" "CHIMERA_PEER_UPDATE_LISTEN" "${listen:-0.0.0.0:0}"
  upsert_env_kv "$PEER_UPDATE_ENV_FILE" "CHIMERA_PEER_UPDATE_STATE_FILE" "$PEER_UPDATE_STATE_FILE"
  chmod 600 "$PEER_UPDATE_ENV_FILE" 2>/dev/null || true
  return 0
}

peer_update_runtime_configured() {
  local base_url="${CHIMERA_PEER_UPDATE_BASE_URL:-}"
  if [[ -z "$base_url" && -f "$PEER_UPDATE_ENV_FILE" ]]; then
    base_url="$(trim_ascii_line "$(read_peer_update_env_kv CHIMERA_PEER_UPDATE_BASE_URL 2>/dev/null || true)")"
  fi
  if [[ -z "$base_url" && -f "$BOOTSTRAP_ENV_FILE" ]]; then
    load_bootstrap_env_if_present || return $?
    base_url="${CHIMERA_PEER_UPDATE_BASE_URL:-}"
  fi
  [[ -n "$(trim_ascii "$base_url")" ]]
}

restart_stale_peer_update_runtime_if_state_missing() {
  local pid_file
  pid_file="$(peer_update_pid_path)"
  if ! pidfile_running "$pid_file"; then
    return 1
  fi
  if [[ -s "$PEER_UPDATE_STATE_FILE" ]]; then
    return 1
  fi
  stop_runner_background "peer_update" "$pid_file" >/dev/null 2>&1 || true
  append_runtime_autofix_log "peer_update_state_restart" "action=restart_without_state"
  return 0
}

start_peer_update_runtime() {
  configure_peer_update_env || return 1
  restart_stale_peer_update_runtime_if_state_missing >/dev/null 2>&1 || true
  if ! pidfile_running "$(peer_update_pid_path)"; then
    rm -f "$PEER_UPDATE_STATE_FILE"
  fi
  start_runner_background "peer_update" "$(peer_update_pid_path)" "$PEER_UPDATE_LOG" "$PEER_UPDATE_ENV_FILE" "peer-update" >/dev/null 2>&1 || return 1
  wait_for_file "$PEER_UPDATE_STATE_FILE" 5
}

start_peer_update_runtime_with_retry() {
  if peer_update_listener_needs_preemptive_repair; then
    repair_peer_update_listener_for_retry >/dev/null 2>&1 || true
  else
    clear_peer_update_listener_runtime_override
  fi
  if start_peer_update_runtime; then
    return 0
  fi
  stop_runner_background "peer_update" "$(peer_update_pid_path)" >/dev/null 2>&1 || true
  if repair_peer_update_listener_for_retry; then
    start_peer_update_runtime
    return $?
  fi
  return 1
}

START_RUNTIME_PEER_UPDATE_STATUS="skipped"
START_RUNTIME_TRANSIT_LANE_BINDINGS_STATUS="skipped"
START_RUNTIME_DISCOVERY_PUBLISH_STATUS="skipped"

refresh_runtime_publication_after_node_start() {
  clear_stale_publication_runtime_state
  START_RUNTIME_PEER_UPDATE_STATUS="skipped"
  START_RUNTIME_TRANSIT_LANE_BINDINGS_STATUS="skipped"
  START_RUNTIME_DISCOVERY_PUBLISH_STATUS="skipped"
  if peer_update_runtime_configured; then
    if start_peer_update_runtime_with_retry >/dev/null 2>&1; then
      START_RUNTIME_PEER_UPDATE_STATUS="ok"
    else
      START_RUNTIME_PEER_UPDATE_STATUS="failed"
    fi
  fi
  if peer_egress_bound_transit_requested; then
    if publish_peer_egress_transit_lane_bindings_from_control_plane strict >/dev/null 2>&1; then
      START_RUNTIME_TRANSIT_LANE_BINDINGS_STATUS="ok"
    else
      START_RUNTIME_TRANSIT_LANE_BINDINGS_STATUS="failed"
    fi
  elif [[ -f "$PEER_EGRESS_ENV_FILE" ]]; then
    publish_peer_egress_transit_lane_bindings_from_control_plane >/dev/null 2>&1 || true
  fi
  if mesh_discovery_source_present; then
    if publish_mesh_discovery_snapshot strict >/dev/null 2>&1; then
      START_RUNTIME_DISCOVERY_PUBLISH_STATUS="ok"
    else
      START_RUNTIME_DISCOVERY_PUBLISH_STATUS="failed"
    fi
  elif [[ "$START_RUNTIME_PEER_UPDATE_STATUS" == "ok" ]]; then
    publish_mesh_discovery_snapshot >/dev/null 2>&1 || true
  fi
  [[ "$START_RUNTIME_PEER_UPDATE_STATUS" != "failed" \
    && "$START_RUNTIME_TRANSIT_LANE_BINDINGS_STATUS" != "failed" \
    && "$START_RUNTIME_DISCOVERY_PUBLISH_STATUS" != "failed" ]]
}

runtime_publication_state() {
  local publication_needed=0
  local degraded=0
  if peer_update_runtime_configured; then
    publication_needed=1
    [[ -s "$PEER_UPDATE_STATE_FILE" ]] || degraded=1
  fi
  if peer_egress_bound_transit_requested; then
    publication_needed=1
    peer_egress_transit_lane_bindings_ready || degraded=1
  fi
  if mesh_discovery_source_present; then
    publication_needed=1
    [[ -s "$MESH_DISCOVERY_OUT_FILE" && -s "$MESH_DISCOVERY_PUBKEY_OUT_FILE" ]] || degraded=1
  fi
  if [[ "$publication_needed" -eq 0 ]]; then
    printf '%s\n' "not_configured"
    return 0
  fi
  if [[ "$degraded" -eq 0 ]]; then
    printf '%s\n' "ready"
  else
    printf '%s\n' "degraded"
  fi
}

usage() {
  cat <<'EOF'
Usage: chimera-control.sh <command>

Commands:
  start          Start CHIMERA node services (systemd --user)
  stop           Stop CHIMERA node services
  restart        Restart CHIMERA services
  status         Show status for CHIMERA services and runtime state
  doctor         Run node doctor check
  logs           Tail node logs
  datapath-status
                Show transparent runtime and route status
  app-routes-status  Show parsed app/service routing config
  route-status       Show split routing runtime status
  run-app <app_id> [args...]
                Run selected app under the transparent runtime
  verify-app <app_id> [args...]
                Verify app run under the transparent runtime
  verify-cmd <command...>
                Verify any command/binary under the transparent runtime
  service-route-enable [service...]
                Retired lab-only command; not product datapath evidence
  service-route-disable [service...]
                Retired lab-only command; not product datapath evidence
  verify-service <service...>
                Retired lab-only command; not product datapath evidence
  route-mode [show|full|split|off]
                Set/get CHIMERA routing mode
  split-list-mode [show|allow|deny]
                Set/get split domain list mode:
                allow = only listed domains go through CHIMERA
                deny  = listed domains go direct, all others through CHIMERA
  site-add <domain...>
                Add site domains to CHIMERA split list and apply
  site-remove <domain...>
                Remove site domains from CHIMERA split list and apply
  site-list     Show CHIMERA split site list
  site-auto-resolve <domain...>
                Auto-pick working path for domains and persist decision
  site-auto-status
                Show learned adaptive site decisions
  site-auto-bootstrap
                Seed adaptive DB from known targets and resolve routes now
  site-auto-discover [run|status|clear]
                Discover recent system DNS domains and feed adaptive split logic
  site-auto-watch [start|stop|status|run-once]
                Background adaptive recheck and auto re-pick
  split-transparent [start|stop|status|refresh]
                System-level split capture via transparent TUN runtime
  grant-perms
                Grant CHIMERA runtime sudo permissions for network operations
  preflight-perms [--warn-only]
                Verify required privileges/capabilities before runtime start
  upstream-probe
                Show candidate upstream endpoints and measured connect latency
  upstream-reset
                Clear upstream sticky/health state and force fresh endpoint choice
  upstream-audit [lines]
                Show upstream health snapshot + recent watchdog switch history
  upstream-failover-smoke [wait_sec]
                Force local tunnel drop and print recovery audit
  mesh-seed-control-plane [--strict|--best-effort]
                Persist mesh control-plane hints from configured/bootstrap mesh state
  mesh-bind-control-plane [--strict|--best-effort]
                Generate mesh control-plane env and publish transit lane bindings
  apps-running  Show running applications (process names)
  services-running
                Show running user services
  mesh <args...>
                Pass through to chimera-cli mesh <args...>
  app-route-add <app_id> <command>
                Retired lab-only command; not product datapath evidence
  app-route-add-running <process_name...>
                Retired lab-only command; not product datapath evidence
  service-route-enable-running [service...]
                Retired lab-only command; not product datapath evidence
  uninstall      Full uninstall + OS/network settings cleanup (best-effort, idempotent)
  ui-mode        Show or set UI mode override: auto|tray|dialog|cli

Safety defaults:
  CHIMERA_SYSTEM_INTEGRATION=0 (default) keeps CHIMERA isolated and prevents
  global desktop proxy or third-party app modifications.
EOF
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: required command not found: $1" >&2
    exit 1
  }
}

update_first_launcher() {
  printf '%s\n' "${CHIMERA_UPDATE_FIRST_LAUNCHER:-$ROOT_DIR/scripts/chimera-sh}"
}

require_update_first_checked() {
  local launcher
  [[ "${CHIMERA_UPDATE_FIRST_CHECKED:-0}" == "1" ]] && return 0
  launcher="$(update_first_launcher)"
  if [[ ! -x "$launcher" ]]; then
    echo "error: update-first launcher is missing: $launcher" >&2
    echo "hint: use chimera-sh for start, restart, mesh and connect commands." >&2
    return 1
  fi
  return 2
}

dispatch_update_first() {
  local launcher
  launcher="$(update_first_launcher)"
  exec "$launcher" "$@"
}

update_first_gate() {
  local rc=0
  require_update_first_checked "$@" || rc=$?
  case "$rc" in
    0)
      return 0
      ;;
    2)
      dispatch_update_first "$@"
      ;;
    *)
      return "$rc"
      ;;
  esac
}

ensure_base_path() {
  export PATH="$HOME/.local/bin:$PATH"
}

ensure_runtime_log_paths() {
  local node_log="${NODE_LOG:-}"
  local datapath_log="${DATAPATH_LOG:-}"
  local state_log="${AUTOFIX_LOG_FILE:-}"
  [[ -n "$node_log" ]] && ensure_parent_dir "$node_log" && touch "$node_log"
  [[ -n "$datapath_log" ]] && ensure_parent_dir "$datapath_log" && touch "$datapath_log"
  [[ -n "$state_log" ]] && ensure_parent_dir "$state_log" && touch "$state_log"
}

wait_for_systemd_unit_stable_active() {
  local unit="${1:?unit_required}"
  local active_timeout_polls="${2:-20}"
  local stable_polls="${3:-5}"
  local i=0
  local state=""

  while (( i < active_timeout_polls )); do
    state="$(systemctl --user is-active "$unit" 2>/dev/null || true)"
    case "$state" in
      active)
        break
        ;;
      failed|inactive|deactivating)
        return 1
        ;;
    esac
    sleep 0.1
    i=$((i + 1))
  done

  [[ "$state" == "active" ]] || return 1

  i=0
  while (( i < stable_polls )); do
    sleep 0.2
    state="$(systemctl --user is-active "$unit" 2>/dev/null || true)"
    [[ "$state" == "active" ]] || return 1
    i=$((i + 1))
  done

  return 0
}

node_config_path() {
  if [[ -f "$NODE_CONFIG_FILE" ]]; then
    echo "$NODE_CONFIG_FILE"
    return 0
  fi
  echo "$ROOT_DIR/configs/mesh-node.example.conf"
}

read_peer_egress_env_kv() {
  local key="${1:?key_required}"
  read_existing_env_kv_from_file "$PEER_EGRESS_ENV_FILE" "$key"
}

read_peer_update_env_kv() {
  local key="${1:?key_required}"
  read_existing_env_kv_from_file "$PEER_UPDATE_ENV_FILE" "$key"
}

read_existing_env_kv_from_file() {
  local file="${1:?file_required}"
  local key="${2:?key_required}"
  [[ -f "$file" ]] || return 0
  local raw
  raw="$(awk -F= -v key="$key" '
    index($0, key "=") == 1 {
      print substr($0, length(key) + 2)
      exit
    }
  ' "$file" 2>/dev/null || true)"
  decode_existing_env_rhs "$key" "$raw"
}

remove_env_kv_from_file() {
  local file="${1:?file_required}"
  local key="${2:?key_required}"
  [[ -f "$file" ]] || return 0
  local tmp_file
  tmp_file="$(mktemp)"
  awk -v key="$key" '
    index($0, key "=") == 1 {
      next
    }
    {
      print
    }
  ' "$file" >"$tmp_file"
  cat "$tmp_file" >"$file"
  rm -f "$tmp_file"
}

runtime_listener_override_file_has_entries() {
  [[ -f "$RUNTIME_LISTENER_OVERRIDE_FILE" ]] || return 1
  grep -Eq '^[A-Z0-9_]+=' "$RUNTIME_LISTENER_OVERRIDE_FILE" 2>/dev/null
}

read_runtime_listener_override_kv() {
  local key="${1:?key_required}"
  read_existing_env_kv_from_file "$RUNTIME_LISTENER_OVERRIDE_FILE" "$key"
}

set_runtime_listener_override_kv() {
  local key="${1:?key_required}"
  local value="${2:-}"
  ensure_parent_dir "$RUNTIME_LISTENER_OVERRIDE_FILE" >/dev/null 2>&1 || true
  touch "$RUNTIME_LISTENER_OVERRIDE_FILE"
  chmod 600 "$RUNTIME_LISTENER_OVERRIDE_FILE" 2>/dev/null || true
  upsert_env_kv "$RUNTIME_LISTENER_OVERRIDE_FILE" "$key" "$value"
}

clear_runtime_listener_override_kv() {
  local key="${1:?key_required}"
  [[ -f "$RUNTIME_LISTENER_OVERRIDE_FILE" ]] || return 0
  remove_env_kv_from_file "$RUNTIME_LISTENER_OVERRIDE_FILE" "$key"
  if ! runtime_listener_override_file_has_entries; then
    rm -f "$RUNTIME_LISTENER_OVERRIDE_FILE"
  fi
}

clear_node_listener_runtime_overrides() {
  clear_runtime_listener_override_kv "CHIMERA_PEER_EGRESS_LOCAL_LISTEN" || true
  clear_runtime_listener_override_kv "CHIMERA_PEER_EGRESS_PEER_LISTEN" || true
}

clear_peer_update_listener_runtime_override() {
  clear_runtime_listener_override_kv "CHIMERA_PEER_UPDATE_LISTEN"
}

peer_egress_bound_transit_requested() {
  local value
  value="$(read_peer_egress_env_kv CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT | tr -d '[:space:]')"
  [[ "$value" == "true" ]]
}

peer_egress_transit_lane_bindings_file() {
  local value="${CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE:-}"
  if [[ -z "$value" ]]; then
    value="$(read_peer_egress_env_kv CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE | tr -d '\r\n')"
  fi
  printf '%s' "$value"
}

peer_egress_transit_lane_bindings_ready() {
  local lane_file
  lane_file="$(peer_egress_transit_lane_bindings_file)"
  [[ -n "$lane_file" && -s "$lane_file" ]]
}

bound_transit_authoritative_peer_source_present() {
  if [[ -f "$BOOTSTRAP_ENV_FILE" ]]; then
    load_bootstrap_env_if_present || return $?
  fi
  [[ -n "$(trim_ascii "${CHIMERA_MESH_REMOTE_PEER_SPEC:-}")" ]] && return 0
  [[ -n "$(trim_ascii "${CHIMERA_MESH_EXTRA_PEERS:-}")" ]] && return 0
  if mesh_discovery_source_present; then
    return 0
  fi
  if [[ -n "${CHIMERA_MESH_REMOTE_NODE:-}" && -n "${CHIMERA_MESH_REMOTE_ENDPOINT:-}" && -n "${CHIMERA_MESH_REMOTE_REGION:-}" && -n "${CHIMERA_MESH_REMOTE_LOAD_SCORE:-}" && -n "${CHIMERA_MESH_REMOTE_RELIABILITY_SCORE:-}" ]]; then
    return 0
  fi
  return 1
}

bound_transit_authority_state() {
  if ! peer_egress_bound_transit_requested; then
    printf '%s\n' "disabled"
    return 0
  fi
  if bound_transit_authoritative_peer_source_present; then
    printf '%s\n' "present"
    return 0
  fi
  printf '%s\n' "missing"
}

ensure_bound_transit_start_contract() {
  peer_egress_bound_transit_requested || return 0
  local output="" rc=0
  output="$(seed_mesh_control_plane_authority_from_bootstrap --best-effort 2>&1)" || rc=$?
  if [[ "$rc" -ne 0 && -n "$output" ]]; then
    printf '%s\n' "$output" >&2
  fi
  bound_transit_authoritative_peer_source_present || return 0
  output="$(mesh_bind_control_plane --strict 2>&1)" || rc=$?
  if [[ "$rc" -ne 0 ]]; then
    [[ -n "$output" ]] && printf '%s\n' "$output" >&2
    rc=0
    output="$(mesh_bind_control_plane --best-effort 2>&1)" || rc=$?
    if [[ "$rc" -ne 0 ]]; then
      [[ -n "$output" ]] && printf '%s\n' "$output" >&2
    fi
  fi
  if ! peer_egress_transit_lane_bindings_ready; then
    printf '%s\n' "bound_transit_start_contract=degraded reason=transit_lane_bindings_not_ready" >&2
    return 0
  fi
  printf '%s\n' "bound_transit_start_contract=ok" >&2
  return 0
}

node_listener_only_bootstrap_ready() {
  local mode token local_listen peer_listen
  mode="$(trim_ascii_line "$(read_peer_egress_env_kv CHIMERA_PEER_EGRESS_MODE)")"
  token="$(trim_ascii_line "$(read_peer_egress_env_kv CHIMERA_PEER_EGRESS_TOKEN)")"
  local_listen="$(trim_ascii_line "$(read_peer_egress_env_kv CHIMERA_PEER_EGRESS_LOCAL_LISTEN)")"
  peer_listen="$(trim_ascii_line "$(read_peer_egress_env_kv CHIMERA_PEER_EGRESS_PEER_LISTEN)")"
  [[ "$mode" == "node" ]] || return 1
  [[ -n "$token" && -n "$local_listen" && -n "$peer_listen" ]] || return 1
  return 0
}

node_config_ready() {
  local config_path addr
  config_path="$(node_config_path)"
  [[ -f "$config_path" ]] || return 1
  addr="$(awk -F'=' '
    $1 ~ /^[[:space:]]*carrier\.addr[[:space:]]*$/ {
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2);
      print $2;
      exit
    }
  ' "$config_path" 2>/dev/null || true)"
  addr="${addr#tcp://}"
  case "$addr" in
    ""|\$\{*\}|*CHIMERA_NODE_PEER_ENDPOINT*|127.0.0.1:443|192.0.2.*|198.51.100.*|203.0.113.*|*.invalid:*) return 1 ;;
    *) return 0 ;;
  esac
}

node_config_carrier_addr() {
  local config_path
  config_path="$(node_config_path)"
  [[ -f "$config_path" ]] || return 1
  awk -F'=' '
    $1 ~ /^[[:space:]]*carrier\.addr[[:space:]]*$/ {
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2);
      print $2;
      exit
    }
  ' "$config_path" 2>/dev/null || true
}

carrier_addr_host() {
  local addr="${1:-}"
  addr="${addr#tcp://}"
  addr="${addr#tcp@}"
  if [[ "$addr" == \[*\]:* ]]; then
    addr="${addr#[}"
    printf '%s\n' "${addr%%]:*}"
    return 0
  fi
  if [[ "$addr" == *:* ]]; then
    printf '%s\n' "${addr%:*}"
    return 0
  fi
  printf '%s\n' "$addr"
}

local_host_address_candidates() {
  printf '%s\n' "127.0.0.1" "::1" "localhost"
  if command -v hostname >/dev/null 2>&1; then
    hostname -I 2>/dev/null | tr ' ' '\n' | awk 'NF { print $1 }'
  fi
  if command -v ip >/dev/null 2>&1; then
    ip -o addr show up scope global 2>/dev/null | awk '{split($4, parts, "/"); if (parts[1] != "") print parts[1]}'
    ip route get 1.1.1.1 2>/dev/null | awk '{for (i = 1; i <= NF; i++) if ($i == "src") { print $(i + 1); exit }}'
  fi
}

node_config_self_loop_target() {
  local carrier_addr carrier_host candidate
  carrier_addr="$(node_config_carrier_addr)"
  [[ -n "$carrier_addr" ]] || return 1
  carrier_host="$(carrier_addr_host "$carrier_addr")"
  carrier_host="${carrier_host#[}"
  carrier_host="${carrier_host%]}"
  [[ -n "$carrier_host" ]] || return 1
  while IFS= read -r candidate; do
    candidate="$(trim_ascii_line "$candidate")"
    [[ -n "$candidate" ]] || continue
    if [[ "$carrier_host" == "$candidate" ]]; then
      return 0
    fi
  done < <(local_host_address_candidates | sort -u)
  return 1
}

load_cli_privilege_env() {
  if [[ -f "$TRANSPARENT_RUNTIME_ENV_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$TRANSPARENT_RUNTIME_ENV_FILE"
  fi
}

route_fwmark_env_value() {
  local mark="${CHIMERA_ROUTE_FWMARK:-}"
  if [[ -z "$mark" && -f "$TRANSPARENT_RUNTIME_ENV_FILE" ]]; then
    mark="$(awk -F= '/^CHIMERA_REDIRECT_SERVICE_FWMARK=/{print $2; exit}' "$TRANSPARENT_RUNTIME_ENV_FILE" 2>/dev/null || true)"
  fi
  if [[ -z "$mark" ]]; then
    mark="${CHIMERA_SERVICE_FWMARK:-0x5244}"
  fi
  trim_ascii "$mark"
}

service_fwmark_env_value() {
  local mark="${CHIMERA_SERVICE_FWMARK:-}"
  if [[ -z "$mark" && -f "$TRANSPARENT_RUNTIME_ENV_FILE" ]]; then
    mark="$(awk -F= '/^CHIMERA_REDIRECT_SERVICE_FWMARK=/{print $2; exit}' "$TRANSPARENT_RUNTIME_ENV_FILE" 2>/dev/null || true)"
  fi
  trim_ascii "${mark:-0x5244}"
}

ensure_peer_egress_service_fwmark() {
  local mark
  mark="$(service_fwmark_env_value)"
  [[ -f "$PEER_EGRESS_ENV_FILE" ]] || return 0
  if ! grep -q '^CHIMERA_SERVICE_FWMARK=' "$PEER_EGRESS_ENV_FILE" 2>/dev/null; then
    upsert_env_kv "$PEER_EGRESS_ENV_FILE" 'CHIMERA_SERVICE_FWMARK' "$mark"
  fi
}

ensure_transparent_runtime_service_fwmark() {
  local mark
  mark="$(service_fwmark_env_value)"
  [[ -f "$TRANSPARENT_RUNTIME_ENV_FILE" ]] || return 0
  if ! grep -q '^CHIMERA_REDIRECT_SERVICE_FWMARK=' "$TRANSPARENT_RUNTIME_ENV_FILE" 2>/dev/null; then
    upsert_env_kv "$TRANSPARENT_RUNTIME_ENV_FILE" 'CHIMERA_REDIRECT_SERVICE_FWMARK' "$mark"
  fi
}

ensure_peer_egress_local_listen_aligned() {
  local desired="127.0.0.1:18135"
  local current
  [[ -f "$PEER_EGRESS_ENV_FILE" ]] || return 0
  current="$(trim_ascii_line "$(read_peer_egress_env_kv CHIMERA_PEER_EGRESS_LOCAL_LISTEN)")"
  if [[ -z "$current" || "$current" == "127.0.0.1:0" ]]; then
    upsert_env_kv "$PEER_EGRESS_ENV_FILE" 'CHIMERA_PEER_EGRESS_LOCAL_LISTEN' "$desired"
  fi
}

run_chimera_cli_up_with_retry() {
  local max_attempts=15 wait_seconds=2 attempt rc=2
  for ((attempt=1; attempt<=max_attempts; attempt++)); do
    if run_chimera_cli up "$@" >/dev/null 2>&1; then
      return 0
    else
      rc=$?
      if [[ "$rc" -eq 2 && "$attempt" -lt "$max_attempts" ]]; then
        sleep "$wait_seconds"
        continue
      fi
      return "$rc"
    fi
  done
  return "$rc"
}

should_run_chimera_cli_with_sudo() {
  local subcommand="${1:-}"
  case "$subcommand" in
    up|down|rollback)
      ;;
    *)
      return 1
      ;;
  esac
  load_cli_privilege_env
  [[ "${CHIMERA_RUNNER_USE_SUDO:-0}" == "1" ]] || return 1
  [[ "$(id -u)" != "0" ]] || return 1
  command -v sudo >/dev/null 2>&1 || return 1
  return 0
}

run_chimera_cli() {
  local subcommand="${1:-}"
  if should_run_chimera_cli_with_sudo "$subcommand"; then
    local xdg_cache_home="${XDG_CACHE_HOME:-$HOME/.cache}"
    local xdg_config_home="${XDG_CONFIG_HOME:-$HOME/.config}"
    local -a sudo_env=(
      "HOME=$HOME"
      "PATH=$PATH"
      "XDG_CACHE_HOME=$xdg_cache_home"
      "XDG_CONFIG_HOME=$xdg_config_home"
      "CHIMERA_RUNNER_USE_SUDO=${CHIMERA_RUNNER_USE_SUDO:-1}"
    )
    if [[ -n "${XDG_RUNTIME_DIR:-}" ]]; then
      sudo_env+=("XDG_RUNTIME_DIR=$XDG_RUNTIME_DIR")
    fi
    if [[ -n "${CHIMERA_NFT_PRIVILEGE_MODE:-}" ]]; then
      sudo_env+=("CHIMERA_NFT_PRIVILEGE_MODE=$CHIMERA_NFT_PRIVILEGE_MODE")
    fi
    if [[ -n "${CHIMERA_ALLOW_SELF_UPSTREAM:-}" ]]; then
      sudo_env+=("CHIMERA_ALLOW_SELF_UPSTREAM=$CHIMERA_ALLOW_SELF_UPSTREAM")
    fi
    if [[ -x "$CHIMERA_RUNNER" ]]; then
      sudo -n env "${sudo_env[@]}" "$CHIMERA_RUNNER" cli "$@"
      return $?
    fi
    if [[ -x "$CHIMERA_CLI_BIN" ]]; then
      sudo -n env "${sudo_env[@]}" "$CHIMERA_CLI_BIN" "$@"
      return $?
    fi
  fi
  if [[ -x "$CHIMERA_RUNNER" ]]; then
    "$CHIMERA_RUNNER" cli "$@"
    return $?
  fi
  if [[ -x "$CHIMERA_CLI_BIN" ]]; then
    "$CHIMERA_CLI_BIN" "$@"
    return $?
  fi
  echo "error: shipped chimera-cli binary is missing: $CHIMERA_CLI_BIN" >&2
  return 1
}

remove_state_file_for_datapath_apply() {
  if should_run_chimera_cli_with_sudo up; then
    sudo -n rm -f "$STATE_FILE"
    return $?
  fi
  rm -f "$STATE_FILE"
}

default_tun_device_name() {
  printf '%s\n' "${CHIMERA_TUN_NAME:-chimera0}"
}

run_ip_privileged() {
  if should_run_chimera_cli_with_sudo up; then
    sudo -n ip "$@"
    return $?
  fi
  ip "$@"
}

cleanup_stale_tun_without_state() {
  local tun_name="${1:-$(default_tun_device_name)}"
  [[ -n "$tun_name" ]] || return 0
  [[ -f "$STATE_FILE" ]] && return 0
  if ! run_ip_privileged link show dev "$tun_name" >/dev/null 2>&1; then
    return 0
  fi
  run_ip_privileged link delete dev "$tun_name" >/dev/null 2>&1
}

clear_runtime_generated_state() {
  clear_peer_egress_transit_lane_bindings_runtime_state
  rm -f \
    "$STATE_FILE" \
    "$PEER_EGRESS_STATE_FILE" \
    "$PEER_UPDATE_STATE_FILE" \
    "$MESH_DISCOVERY_OUT_FILE" \
    "$MESH_DISCOVERY_PUBKEY_OUT_FILE" \
    "$RUNTIME_LISTENER_OVERRIDE_FILE" \
    "$SITE_AUTOWATCH_PID_FILE" \
    "$(peer_egress_pid_path)" \
    "$(transparent_runtime_pid_path)" \
    "$(peer_update_pid_path)" >/dev/null 2>&1 || true
}

peer_egress_runtime_live() {
  if systemd_user_ready; then
    [[ "$(systemctl --user is-active "$NODE_SERVICE_UNIT" 2>/dev/null || true)" == "active" ]] && return 0
  fi
  pidfile_running "$(peer_egress_pid_path)"
}

peer_update_runtime_live() {
  pidfile_running "$(peer_update_pid_path)"
}

clear_stale_publication_runtime_state() {
  if ! peer_egress_runtime_live; then
    rm -f "$PEER_EGRESS_STATE_FILE" >/dev/null 2>&1 || true
    clear_mesh_discovery_snapshot_runtime_state
    clear_peer_egress_transit_lane_bindings_runtime_state
  fi
  if ! peer_update_runtime_live; then
    rm -f "$PEER_UPDATE_STATE_FILE" >/dev/null 2>&1 || true
  fi
}

partial_start_fail_closed() {
  if [[ -n "${CHIMERA_MESH_NODES_DISCOVERY_URL:-}${CHIMERA_MESH_NODES_DISCOVERY_URLS:-}" || -s "${CHIMERA_MESH_NODES_DISCOVERY_URLS_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/mesh_nodes_discovery_urls.list}" ]]; then
    return 1
  fi
  [[ "${CHIMERA_FAIL_CLOSED_ON_PARTIAL_START:-1}" != "0" ]]
}

stop_partial_runtime_components() {
  local mode="${1:?mode_required}"
  stop_runner_background "peer_update" "$(peer_update_pid_path)" >/dev/null 2>&1 || true
  case "$mode" in
    systemd_user)
      if systemd_user_ready; then
        systemctl --user stop "$DATAPATH_SERVICE_UNIT" "$NODE_SERVICE_UNIT" >/dev/null 2>&1 || true
      fi
      ;;
    direct)
      stop_runner_background "transparent_runtime" "$(transparent_runtime_pid_path)" >/dev/null 2>&1 || true
      stop_runner_background "peer_egress" "$(peer_egress_pid_path)" >/dev/null 2>&1 || true
      ;;
  esac
  clear_runtime_generated_state
}

run_chimera_node() {
  if [[ -x "$CHIMERA_RUNNER" ]]; then
    "$CHIMERA_RUNNER" node "$@"
    return $?
  fi
  if [[ -x "$CHIMERA_NODE_BIN" ]]; then
    "$CHIMERA_NODE_BIN" "$@"
    return $?
  fi
  echo "error: shipped chimera-node binary is missing: $CHIMERA_NODE_BIN" >&2
  return 1
}

run_chimera_runner() {
  local target="${1:?target_required}"
  shift || true
  if [[ -x "$CHIMERA_RUNNER" ]]; then
    "$CHIMERA_RUNNER" "$target" "$@"
    return $?
  fi
  echo "error: chimera-runner script is missing" >&2
  return 1
}

datapath_apply_proof_state() {
  local output rc
  output="$(run_chimera_cli state proof --state-file "$STATE_FILE" 2>/dev/null)" && rc=0 || rc=$?
  if [[ "$output" =~ (^|[[:space:]])datapath_proof=([A-Za-z0-9_:-]+) ]]; then
    echo "${BASH_REMATCH[2]}"
    return "$rc"
  fi
  echo "proof_command_failed"
  return 1
}

datapath_apply_proof_ok() {
  [[ "$(datapath_apply_proof_state)" == "ok" ]]
}

datapath_strict_flow_proof_state() {
  if ! [[ "$CHIMERA_FLOW_PROOF_MAX_AGE_SEC" =~ ^[0-9]+$ ]] || (( CHIMERA_FLOW_PROOF_MAX_AGE_SEC < 1 )); then
    echo "flow_proof_bad_max_age"
    return 1
  fi
  local output rc
  output="$(
    run_chimera_cli state proof \
      --state-file "$STATE_FILE" \
      --require-flow true \
      --max-flow-age-sec "$CHIMERA_FLOW_PROOF_MAX_AGE_SEC" \
      2>/dev/null
  )" && rc=0 || rc=$?
  if [[ "$output" =~ (^|[[:space:]])datapath_proof=([A-Za-z0-9_:-]+) ]]; then
    echo "${BASH_REMATCH[2]}"
    return "$rc"
  fi
  echo "flow_proof_command_failed"
  return 1
}

PRESTART_SAVED_STATE_PROOF="missing_state"
PRESTART_SAVED_STATE_RECOVERY="not_needed"

recover_saved_runtime_state_if_present() {
  PRESTART_SAVED_STATE_PROOF="missing_state"
  PRESTART_SAVED_STATE_RECOVERY="not_needed"
  [[ -f "$STATE_FILE" ]] || return 0
  local proof_status
  proof_status="$(datapath_apply_proof_state || true)"
  PRESTART_SAVED_STATE_PROOF="$proof_status"
  if run_chimera_cli rollback recover --state-file "$STATE_FILE" >/dev/null 2>&1; then
    PRESTART_SAVED_STATE_RECOVERY="ok"
    clear_runtime_generated_state
  else
    case "$proof_status" in
      network_not_modified|tun_not_applied|route_not_applied|dns_not_applied)
        if ! remove_state_file_for_datapath_apply; then
          PRESTART_SAVED_STATE_RECOVERY="cleanup_failed"
          return 1
        fi
        PRESTART_SAVED_STATE_RECOVERY="cleanup_only"
        clear_runtime_generated_state
        ;;
      *)
        if [[ "${CHIMERA_FAIL_CLOSED_ON_PARTIAL_START:-1}" == "0" ]]; then
          if ! remove_state_file_for_datapath_apply; then
            PRESTART_SAVED_STATE_RECOVERY="cleanup_failed"
            return 1
          fi
          PRESTART_SAVED_STATE_RECOVERY="cleanup_only_forced"
          clear_runtime_generated_state
        else
          PRESTART_SAVED_STATE_RECOVERY="invalid"
          return 2
        fi
        ;;
    esac
  fi
  if ! cleanup_stale_tun_without_state; then
    PRESTART_SAVED_STATE_RECOVERY="cleanup_failed"
    return 1
  fi
  return 0
}

start_runner_background() {
  local name="${1:?name_required}"
  local pid_file="${2:?pid_file_required}"
  local log_file="${3:?log_file_required}"
  local env_file="${4:?env_file_required}"
  local target="${5:?target_required}"
  local override_file="${RUNTIME_LISTENER_OVERRIDE_FILE:-}"

  if pidfile_running "$pid_file"; then
    local pid
    pid="$(tr -d '[:space:]' <"$pid_file" 2>/dev/null || true)"
    echo "${name}_status=running pid=$pid"
    return 0
  fi

  ensure_parent_dir "$pid_file"
  ensure_parent_dir "$log_file"
  if ! validate_safe_env_file_for_source "$env_file"; then
    echo "error: invalid runtime env: $env_file" >&2
    return 1
  fi
  if [[ -n "$override_file" && -f "$override_file" ]] && ! validate_safe_env_file_for_source "$override_file"; then
    echo "error: invalid runtime override env: $override_file" >&2
    return 1
  fi

  nohup bash -lc '
    set -euo pipefail
    env_file="$1"
    runner="$2"
    target="$3"
    override_file="$4"
    if [[ ! -f "$env_file" ]]; then
      echo "error: missing env file: $env_file" >&2
      exit 1
    fi
    set -a
    # shellcheck disable=SC1090
    source "$env_file"
    if [[ -n "$override_file" && -f "$override_file" && ( "$target" == "peer-egress" || "$target" == "peer-update" ) ]]; then
      # shellcheck disable=SC1090
      source "$override_file"
    fi
    exec "$runner" "$target"
  ' _ "$env_file" "$CHIMERA_RUNNER" "$target" "$override_file" >>"$log_file" 2>&1 &

  local pid=$!
  printf '%s\n' "$pid" >"$pid_file"
  echo "${name}_status=started pid=$pid"
}

wait_for_file() {
  local file="${1:?file_required}"
  local timeout_sec="${2:-5}"
  local i=0
  while (( i < timeout_sec * 10 )); do
    [[ -s "$file" ]] && return 0
    sleep 0.1
    i=$((i + 1))
  done
  return 1
}

kill_existing_runner_by_needle() {
  local needle="${1:?needle_required}"
  local pid=""
  local pids=""
  if command -v pgrep >/dev/null 2>&1; then
    pids="$(pgrep -f "$needle" 2>/dev/null || true)"
  elif command -v ps >/dev/null 2>&1; then
    pids="$(ps -eo pid,args 2>/dev/null | awk -v needle="$needle" -v self="$$" '
      index($0, needle) && $1 != self && index($0, "awk") == 0 { print $1 }
    ' || true)"
  fi
  for pid in $pids; do
    [[ "$pid" == "$$" ]] && continue
    [[ "$pid" =~ ^[0-9]+$ ]] || continue
    kill "$pid" >/dev/null 2>&1 || true
    sleep 0.2
    kill -0 "$pid" >/dev/null 2>&1 && kill -9 "$pid" >/dev/null 2>&1 || true
  done
}

stop_runner_background() {
  local name="${1:?name_required}"
  local pid_file="${2:?pid_file_required}"
  local fallback_needle=""

  if [[ -f "$pid_file" ]]; then
    local pid
    pid="$(tr -d '[:space:]' <"$pid_file" 2>/dev/null || true)"
    if [[ -n "$pid" ]]; then
      case "$pid_file" in
        *chimera-peer-egress.pid)
          pid_cmdline_contains "$pid" "$(runner_cmdline_needle_for_target peer-egress)" || {
            rm -f "$pid_file"
            echo "${name}_status=ignored_foreign_pid"
            return 0
          }
          ;;
        *chimera-transparent-runtime.pid)
          pid_cmdline_contains "$pid" "$(runner_cmdline_needle_for_target transparent-runtime)" || {
            rm -f "$pid_file"
            echo "${name}_status=ignored_foreign_pid"
            return 0
          }
          ;;
        *chimera-peer-update.pid)
          pid_cmdline_contains "$pid" "$(runner_cmdline_needle_for_target peer-update)" || {
            rm -f "$pid_file"
            echo "${name}_status=ignored_foreign_pid"
            return 0
          }
          ;;
      esac
      kill "$pid" >/dev/null 2>&1 || true
      sleep 0.2
      kill -0 "$pid" >/dev/null 2>&1 && kill -9 "$pid" >/dev/null 2>&1 || true
    fi
    rm -f "$pid_file"
  fi
  local target_name="$name"
  case "$name" in
    peer_egress) target_name="peer-egress" ;;
    peer_update) target_name="peer-update" ;;
    transparent_runtime) target_name="transparent-runtime" ;;
  esac
  fallback_needle="$(runner_cmdline_needle_for_target "$target_name" 2>/dev/null || true)"
  if [[ -n "$fallback_needle" ]]; then
    kill_existing_runner_by_needle "$fallback_needle" >/dev/null 2>&1 || true
  fi
  echo "${name}_status=stopped"
}

valid_nft_identifier() {
  local value="${1:-}"
  [[ "$value" =~ ^[A-Za-z_][A-Za-z0-9_-]{0,62}$ ]]
}

valid_chimera_redirect_table() {
  local value="${1:-}"
  valid_nft_identifier "$value" || return 1
  [[ "$value" == "chimera_redirect" || "$value" == chimera_redirect_* ]]
}

transparent_redirect_table_name() {
  local table="${CHIMERA_REDIRECT_TABLE:-}"
  if [[ -z "$table" && -f "$TRANSPARENT_RUNTIME_ENV_FILE" ]]; then
    table="$(awk -F= '/^CHIMERA_REDIRECT_TABLE=/{print $2; exit}' "$TRANSPARENT_RUNTIME_ENV_FILE" 2>/dev/null || true)"
    table="$(trim_ascii "$table")"
  fi
  if [[ -z "$table" ]]; then
    table="chimera_redirect"
  fi
  if ! valid_chimera_redirect_table "$table"; then
    echo "transparent_redirect_cleanup=fail reason=invalid_chimera_table_name" >&2
    return 1
  fi
  printf '%s\n' "$table"
}

resolve_nft_command() {
  local nft_cmd="$NFT_BIN"
  if [[ -n "$nft_cmd" ]]; then
    if [[ "${CHIMERA_ALLOW_TEST_NFT_BIN:-0}" != "1" ]]; then
      case "$nft_cmd" in
        /usr/sbin/nft|/usr/bin/nft) ;;
        *) return 127 ;;
      esac
    fi
    [[ -x "$nft_cmd" && "$(basename "$nft_cmd")" == "nft" ]] || return 127
    printf '%s\n' "$nft_cmd"
    return 0
  fi
  if [[ -x /usr/sbin/nft ]]; then
    printf '%s\n' "/usr/sbin/nft"
    return 0
  fi
  if [[ -x /usr/bin/nft ]]; then
    printf '%s\n' "/usr/bin/nft"
    return 0
  fi
  return 127
}

run_nft_command() {
  local nft_cmd
  nft_cmd="$(resolve_nft_command)" || return 127
  if (( EUID == 0 )); then
    "$nft_cmd" "$@"
  else
    sudo -n "$nft_cmd" "$@"
  fi
}

cleanup_transparent_redirect_rules() {
  local table output rc
  table="$(transparent_redirect_table_name)" || return 1
  if ! resolve_nft_command >/dev/null; then
    echo "transparent_redirect_cleanup=skipped reason=nft_missing"
    return 0
  fi
  if output="$(run_nft_command delete table inet "$table" 2>&1)"; then
    rc=0
  else
    rc=$?
  fi
  if [[ "$rc" -eq 0 ]]; then
    echo "transparent_redirect_cleanup=ok table=$table"
    return 0
  fi
  if [[ "$output" == *"No such file"* || "$output" == *"does not exist"* ]]; then
    return 0
  fi
  echo "transparent_redirect_cleanup=fail table=$table reason=nft_delete_failed" >&2
  return 1
}

systemd_user_ready() {
  command -v systemctl >/dev/null 2>&1 && systemctl --user show-environment >/dev/null 2>&1
}

systemd_user_unit_installed_on_disk() {
  local unit="${1:?unit_required}"
  path_exists_or_link "$SYSTEMD_USER_DIR/$unit" || path_exists_or_link "$SYSTEMD_USER_DEFAULT_TARGET_WANTS_DIR/$unit"
}

systemd_runtime_units_installed_on_disk() {
  local unit=""
  for unit in \
    "$RUNTIME_SERVICE_UNIT" \
    "$NODE_SERVICE_UNIT" \
    "$DATAPATH_SERVICE_UNIT" \
    "$SITE_AUTOWATCH_SERVICE_UNIT" \
    "$LEGACY_NODE_COMPAT_SERVICE_UNIT" \
    "$LEGACY_DATAPATH_COMPAT_SERVICE_UNIT"
  do
    if systemd_user_unit_installed_on_disk "$unit"; then
      return 0
    fi
  done
  return 1
}

direct_runtime_mode_blocked_by_orphaned_systemd_units() {
  systemd_user_ready && return 1
  systemd_runtime_units_installed_on_disk
}

systemd_units_active_ok() {
  local datapath_expected="${1:-1}"
  local node_state datapath_state
  node_state="$(systemctl --user is-active "$NODE_SERVICE_UNIT" 2>/dev/null || true)"
  datapath_state="$(systemctl --user is-active "$DATAPATH_SERVICE_UNIT" 2>/dev/null || true)"
  if [[ "$datapath_expected" == "1" ]]; then
    [[ "$node_state" == "active" && "$datapath_state" == "active" ]]
  else
    [[ "$node_state" == "active" ]]
  fi
}

desktop_proxy_supported() {
  if ! command -v gsettings >/dev/null 2>&1; then
    return 1
  fi
  gsettings list-schemas 2>/dev/null | grep -qx 'org.gnome.system.proxy'
}

read_route_mode() {
  if [[ -f "$ROUTE_MODE_FILE" ]]; then
    local mode
    mode="$(tr -d '[:space:]' < "$ROUTE_MODE_FILE" | tr '[:upper:]' '[:lower:]')"
    case "$mode" in
      full|split|off) echo "$mode"; return 0 ;;
      selective) echo "split"; return 0 ;;
    esac
  fi
  echo "split"
}

write_route_mode() {
  local mode="${1:-split}"
  if [[ "$mode" == "selective" ]]; then
    mode="split"
  fi
  mkdir -p "$(dirname "$ROUTE_MODE_FILE")"
  printf '%s\n' "$mode" > "$ROUTE_MODE_FILE"
}

read_split_list_mode() {
  if [[ -f "$SPLIT_LIST_MODE_FILE" ]]; then
    local mode
    mode="$(tr -d '[:space:]' < "$SPLIT_LIST_MODE_FILE" | tr '[:upper:]' '[:lower:]')"
    case "$mode" in
      allow|deny) echo "$mode"; return 0 ;;
    esac
  fi
  echo "allow"
}

write_split_list_mode() {
  local mode="${1:-allow}"
  case "$mode" in
    allow|deny) ;;
    *) mode="allow" ;;
  esac
  mkdir -p "$(dirname "$SPLIT_LIST_MODE_FILE")"
  printf '%s\n' "$mode" > "$SPLIT_LIST_MODE_FILE"
}

trim_ascii() {
  local s="${1:-}"
  s="${s#"${s%%[![:space:]]*}"}"
  s="${s%"${s##*[![:space:]]}"}"
  printf '%s' "$s"
}

split_csv_lines() {
  local csv="${1:-}"
  IFS=',' read -r -a out <<<"$csv"
  local item
  for item in "${out[@]}"; do
    item="$(trim_ascii "$item")"
    [[ -z "$item" ]] && continue
    printf '%s\n' "$item"
  done
}

is_protected_port() {
  local port="${1:-}"
  [[ "$port" =~ ^[0-9]+$ ]] || return 1
  local p
  while IFS= read -r p; do
    [[ -z "$p" ]] && continue
    if [[ "$p" == "$port" ]]; then
      return 0
    fi
  done < <(split_csv_lines "$CHIMERA_PROTECTED_PORTS_CSV")
  return 1
}

ensure_safe_local_host_guard() {
  [[ "$CHIMERA_SAFE_HOST_LOCK" == "1" ]] || return 0
  if [[ "$CHIMERA_ALLOW_LOCAL_NETWORK_MUTATION" != "1" ]]; then
    # Adaptive safe-profile: do not block command, just prevent risky global mutations.
    SPLIT_TRANSPARENT_ENABLED="0"
    CHIMERA_SYSTEM_INTEGRATION="0"
    echo "chimera_local_safety_profile=active action=disable_system_mutation"
  fi
}

foreign_vpn_contours_present() {
  # If host already has non-CHIMERA WEAVE stack, avoid route/tun mutations by default.
  if pgrep -f '/usr/sbin/openvpn|wg-quick|wireguard|xray|hysteria' >/dev/null 2>&1; then
    return 0
  fi
  # Detect third-party network overlays so CHIMERA does not mutate routes over them by surprise.
  if pgrep -af 'sing-box run -c ' 2>/dev/null | grep -vE 'chimera|transparent-runtime\.json' >/dev/null 2>&1; then
    return 0
  fi
  return 1
}

is_local_upstream_host() {
  local host="${1:-}"
  host="$(trim_ascii "${host,,}")"
  [[ -z "$host" ]] && return 0
  [[ "$host" == "localhost" ]] && return 0
  [[ "$host" == "127.0.0.1" ]] && return 0
  [[ "$host" == "::1" ]] && return 0
  return 1
}

is_placeholder_upstream_value() {
  local value="${1:-}"
  value="$(trim_ascii "${value,,}")"
  case "$value" in
    your_user|your_password|your_server_host_or_ip|example|example.invalid)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

ensure_vpn_coexist_guard() {
  if [[ "$CHIMERA_ALLOW_WEAVE_COEXIST_MUTATION" == "1" ]]; then
    return 0
  fi
  if foreign_vpn_contours_present; then
    CHIMERA_SYSTEM_INTEGRATION="0"
    if [[ "$CHIMERA_COEXIST_TRANSPARENT_CAPTURE" == "1" ]]; then
      SPLIT_TRANSPARENT_ENABLED="1"
      echo "chimera_coexist_guard=active action=keep_transparent_datapath_only reason=foreign_vpn_contours_detected"
      echo "chimera_coexist_transparent_capture=enabled mode=kernel_tun_split"
    else
      SPLIT_TRANSPARENT_ENABLED="0"
      echo "chimera_coexist_guard=active action=disable_system_mutation reason=foreign_vpn_contours_detected"
    fi
  fi
}

run_permissions_preflight() {
  local warn_only="${1:-0}"
  local failed=0
  local tmp_tun="chpq$((RANDOM % 900 + 100))"
  local pid_dir
  local sudo_cmd_ok=1
  pid_dir="$(dirname "$SPLIT_TRANSPARENT_PID_FILE")"
  echo "preflight_kind=permissions"
  sudo -n ip -Version >/dev/null 2>&1 || sudo_cmd_ok=0
  if [[ -e /dev/net/tun ]]; then
    echo "check_dev_net_tun=ok"
  else
    echo "check_dev_net_tun=fail"
    failed=1
  fi
  if sudo -n ip tuntap add dev "$tmp_tun" mode tun >/dev/null 2>&1; then
    sudo -n ip link del "$tmp_tun" >/dev/null 2>&1 || true
    echo "check_tun_create=ok"
  else
    echo "check_tun_create=fail"
    failed=1
  fi
  local nft_cmd="$NFT_BIN"
  if [[ -z "$nft_cmd" ]]; then
    if command -v nft >/dev/null 2>&1; then
      nft_cmd="$(command -v nft)"
    elif [[ -x /usr/sbin/nft ]]; then
      nft_cmd="/usr/sbin/nft"
    fi
  fi
  if [[ -n "$nft_cmd" ]]; then
    sudo -n "$nft_cmd" --version >/dev/null 2>&1 || sudo_cmd_ok=0
    if sudo -n "$nft_cmd" list ruleset >/dev/null 2>&1; then
      echo "check_nft_access=ok"
    else
      echo "check_nft_access=fail"
      failed=1
    fi
  else
    echo "check_nft_access=missing"
    failed=1
  fi
  if sudo -n ip rule show >/dev/null 2>&1; then
    echo "check_iprule_access=ok"
  else
    echo "check_iprule_access=fail"
    failed=1
  fi
  if [[ "$sudo_cmd_ok" -eq 1 ]]; then
    echo "check_sudo_nopass=ok"
  else
    echo "check_sudo_nopass=fail"
    failed=1
  fi
  if mkdir -p "$pid_dir" >/dev/null 2>&1 && [[ -w "$pid_dir" ]]; then
    echo "check_pid_dir_writable=ok dir=$pid_dir"
  else
    echo "check_pid_dir_writable=fail dir=$pid_dir"
    failed=1
  fi
  if [[ "$failed" -eq 0 ]]; then
    echo "preflight_status=ok"
    return 0
  fi
  echo "preflight_status=fail"
  echo "preflight_hint=grant_required_permissions_and_reinstall_or_run_chimera_control_preflight_perms"
  if [[ "$warn_only" -eq 1 ]]; then
    return 0
  fi
  return 2
}

grant_runtime_permissions() {
  local user_name=""
  user_name="${SUDO_USER:-$USER}"
  if [[ -z "$user_name" ]]; then
    echo "grant_perms_status=fail reason=user_not_detected"
    return 2
  fi
  local sudoers_dir="/etc/sudoers.d"
  local sudoers_file="$sudoers_dir/chimera-pq"
  local tmp_file
  tmp_file="$(mktemp)"
  cat >"$tmp_file" <<EOF
# Managed by CHIMERA installer/runtime.
Cmnd_Alias CHIMERA_NET_CMDS = /usr/bin/ip, /usr/sbin/ip, /usr/bin/nft, /usr/sbin/nft, /usr/bin/modprobe, /usr/sbin/modprobe
${user_name} ALL=(root) NOPASSWD: CHIMERA_NET_CMDS
EOF

  if ! sudo mkdir -p "$sudoers_dir"; then
    rm -f "$tmp_file"
    echo "grant_perms_status=fail reason=sudoers_dir_create_failed"
    return 2
  fi
  if ! sudo install -m 0440 "$tmp_file" "$sudoers_file"; then
    rm -f "$tmp_file"
    echo "grant_perms_status=fail reason=sudoers_write_failed"
    return 2
  fi
  rm -f "$tmp_file"

  if command -v visudo >/dev/null 2>&1; then
    if ! sudo visudo -cf "$sudoers_file" >/dev/null 2>&1; then
      sudo rm -f "$sudoers_file" >/dev/null 2>&1 || true
      echo "grant_perms_status=fail reason=sudoers_validation_failed"
      return 2
    fi
  fi
  if [[ ! -e /dev/net/tun ]]; then
    sudo modprobe tun >/dev/null 2>&1 || true
  fi
  echo "grant_perms_status=ok"
  echo "grant_perms_file=$sudoers_file"
  return 0
}

build_upstream_candidates() {
  local out=()
  if [[ -n "${CHIMERA_UPSTREAM_TRANSPORTS_CSV:-}" ]]; then
    local entry
    while IFS= read -r entry; do
      [[ -z "$entry" ]] && continue
      out+=("$entry")
    done < <(split_csv_lines "$CHIMERA_UPSTREAM_TRANSPORTS_CSV")
  fi
  if [[ -n "${CHIMERA_UPSTREAM_ENDPOINTS_CSV:-}" ]]; then
    local entry
    while IFS= read -r entry; do
      [[ -z "$entry" ]] && continue
      out+=("$entry")
    done < <(split_csv_lines "$CHIMERA_UPSTREAM_ENDPOINTS_CSV")
  fi
  if [[ "${#out[@]}" -eq 0 && -n "${CHIMERA_UPSTREAM_HOST:-}" ]]; then
    if ! is_local_upstream_host "${CHIMERA_UPSTREAM_HOST}" && ! is_placeholder_upstream_value "${CHIMERA_UPSTREAM_HOST}"; then
      out+=("${CHIMERA_UPSTREAM_HOST}:${CHIMERA_UPSTREAM_PORT:-22}")
    fi
  fi
  local item parsed endpoint host
  for item in "${out[@]}"; do
    parsed="$(parse_transport_endpoint "$item" || true)"
    endpoint="${parsed#*|}"
    host="${endpoint%:*}"
    if is_local_upstream_host "$host"; then
      continue
    fi
    printf '%s\n' "$item"
  done
}

count_upstream_candidates() {
  local n=0 line
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    n=$((n + 1))
  done < <(build_upstream_candidates)
  echo "$n"
}

parse_transport_endpoint() {
  local candidate="${1:-}"
  local transport="ssh"
  local endpoint="$candidate"
  if [[ "$candidate" == *@* ]]; then
    transport="${candidate%%@*}"
    endpoint="${candidate#*@}"
  fi
  transport="$(trim_ascii "${transport,,}")"
  endpoint="$(trim_ascii "$endpoint")"
  if [[ -z "$endpoint" ]]; then
    return 1
  fi
  local host="${endpoint%:*}"
  local port="${endpoint##*:}"
  if [[ "$host" == "$port" ]]; then
    case "$transport" in
      ssh443|ssh-443|tls|tcp443) port="443" ;;
      ssh8443|ssh-8443|tcp8443) port="8443" ;;
      *) port="${CHIMERA_UPSTREAM_PORT:-22}" ;;
    esac
    endpoint="${host}:${port}"
  fi
  printf '%s|%s\n' "$transport" "$endpoint"
}

endpoint_latency_ms_probe() {
  local parsed endpoint
  parsed="$(parse_transport_endpoint "${1:-}" || true)"
  endpoint="${parsed#*|}"
  local host="${endpoint%:*}"
  local port="${endpoint##*:}"
  [[ -z "$host" || -z "$port" ]] && echo 2147483647 && return 0
  local start end
  start="$(date +%s 2>/dev/null || echo 0)"
  if timeout 2 bash -lc "</dev/tcp/$host/$port" >/dev/null 2>&1; then
    end="$(date +%s 2>/dev/null || echo 0)"
    if [[ "$start" =~ ^[0-9]+$ && "$end" =~ ^[0-9]+$ && "$end" -ge "$start" ]]; then
      echo $(((end - start) * 1000))
    else
      echo 1
    fi
  else
    echo 2147483647
  fi
}

load_upstream_env_context() {
  UPSTREAM_ENV_SOURCE="none"
  LEGACY_UPSTREAM_SOURCE_USED="false"
  if [[ -f "$UPSTREAM_ENV_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$UPSTREAM_ENV_FILE"
    if [[ "$UPSTREAM_ENV_FILE" == "$BOOTSTRAP_ENV_FILE" ]]; then
      UPSTREAM_ENV_SOURCE="mesh_bootstrap_env"
    else
      UPSTREAM_ENV_SOURCE="configured_upstream_env"
    fi
  elif [[ -f "$LEGACY_UPSTREAM_ENV_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$LEGACY_UPSTREAM_ENV_FILE"
    UPSTREAM_ENV_SOURCE="legacy_upstream_env"
    LEGACY_UPSTREAM_SOURCE_USED="true"
  elif [[ "$LEGACY_UPSTREAM_ENV_COMPAT_FILE" != "$LEGACY_UPSTREAM_ENV_FILE" && -f "$LEGACY_UPSTREAM_ENV_COMPAT_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$LEGACY_UPSTREAM_ENV_COMPAT_FILE"
    UPSTREAM_ENV_SOURCE="legacy_upstream_env_compat_path"
    LEGACY_UPSTREAM_SOURCE_USED="true"
  fi
}

upstream_probe() {
  load_upstream_env_context
  local endpoint lat parsed transport endpoint_only
  local best="" best_lat=2147483647
  echo "upstream_source=$UPSTREAM_ENV_SOURCE"
  echo "legacy_upstream_source_used=$LEGACY_UPSTREAM_SOURCE_USED"
  echo "upstream_truth_boundary=legacy_lab_only_not_datapath_evidence"
  echo "upstream_product_datapath_evidence=false"
  while IFS= read -r endpoint; do
    [[ -z "$endpoint" ]] && continue
    parsed="$(parse_transport_endpoint "$endpoint" || true)"
    transport="${parsed%%|*}"
    endpoint_only="${parsed#*|}"
    lat="$(endpoint_latency_ms_probe "$endpoint")"
    echo "upstream_candidate transport=${transport:-unknown} endpoint_present=$([[ -n "${endpoint_only:-}" ]] && echo true || echo false) endpoint=<redacted> latency_ms=$lat"
    if [[ "$lat" =~ ^[0-9]+$ ]] && [[ "$lat" -lt "$best_lat" ]]; then
      best_lat="$lat"
      best="$endpoint_only"
    fi
  done < <(build_upstream_candidates)
  if [[ -n "$best" ]]; then
    echo "upstream_best endpoint_present=true endpoint=<redacted> latency_ms=$best_lat strategy=$UPSTREAM_STRATEGY"
  else
    echo "upstream_best endpoint=none"
  fi
}

upstream_reset() {
  rm -f "$LAST_ENDPOINT_FILE" "$UPSTREAM_HEALTH_STATE_FILE"
  echo "upstream_state_reset=ok"
}

upstream_audit() {
  local lines="${1:-30}"
  if ! [[ "$lines" =~ ^[0-9]+$ ]]; then
    lines=30
  fi
  load_upstream_env_context
  echo "upstream_audit_begin"
  echo "upstream_source=$UPSTREAM_ENV_SOURCE"
  echo "legacy_upstream_source_used=$LEGACY_UPSTREAM_SOURCE_USED"
  echo "upstream_truth_boundary=legacy_lab_only_not_datapath_evidence"
  echo "upstream_product_datapath_evidence=false"
  echo "upstream_strategy=$UPSTREAM_STRATEGY"
  local candidates_total
  candidates_total="$(count_upstream_candidates)"
  echo "upstream_candidates_total=$candidates_total"
  if [[ "$candidates_total" =~ ^[0-9]+$ ]] && [[ "$candidates_total" -ge 2 ]]; then
    echo "upstream_adaptation_possible=true"
  else
    echo "upstream_adaptation_possible=false"
  fi
  if [[ -f "$LAST_ENDPOINT_FILE" ]]; then
    local last_endpoint sticky_until
    last_endpoint="$(awk -F'|' 'NR==1{print $1}' "$LAST_ENDPOINT_FILE" 2>/dev/null || true)"
    sticky_until="$(awk -F'|' 'NR==1{print $2}' "$LAST_ENDPOINT_FILE" 2>/dev/null || true)"
    echo "upstream_last_endpoint_present=$([[ -n "${last_endpoint:-}" ]] && echo true || echo false)"
    echo "upstream_last_endpoint=<redacted>"
    echo "upstream_last_endpoint_sticky_until=${sticky_until:-unknown}"
  else
    echo "upstream_last_endpoint=unknown"
  fi
  if [[ -f "$UPSTREAM_HEALTH_STATE_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$UPSTREAM_HEALTH_STATE_FILE"
    echo "upstream_health_listener_up=${listener_up:-unknown}"
    echo "upstream_health_ok=${health_ok:-unknown}"
    echo "upstream_degrade_fails=${degrade_fails:-unknown}"
    echo "upstream_degrade_threshold=${degrade_threshold:-unknown}"
    echo "upstream_last_reason=${last_reason:-unknown}"
    echo "upstream_last_transport=${last_transport:-unknown}"
    echo "upstream_health_ts=${ts:-unknown}"
  else
    echo "upstream_health_state=missing"
  fi
  echo "upstream_probe_now:"
  upstream_probe
  if [[ -f "$SPLIT_TRANSPARENT_LOG_FILE" ]]; then
    echo "upstream_recent_events:"
    tail -n "$lines" "$SPLIT_TRANSPARENT_LOG_FILE" | grep -E 'route|failover|reason=' || true
  fi
  echo "upstream_audit_end"
}

upstream_failover_smoke() {
  local wait_sec="${1:-10}"
  if ! [[ "$wait_sec" =~ ^[0-9]+$ ]]; then
    wait_sec=10
  fi
  echo "upstream_failover_smoke=legacy_lab_only_not_datapath_evidence"
  split_transparent_status
  sleep "$wait_sec"
  local audit_output legacy_source_used
  audit_output="$(upstream_audit 200)"
  printf '%s\n' "$audit_output"
  legacy_source_used="$(printf '%s\n' "$audit_output" | awk -F= '/^legacy_upstream_source_used=/{print $2; exit}')"
  [[ "${legacy_source_used:-false}" != "true" ]]
}

ensure_parent_dir() {
  mkdir -p "$(dirname "${1:?file_required}")"
}

path_exists_or_link() {
  [[ -e "${1:?path_required}" || -L "${1:?path_required}" ]]
}

remove_path_if_present() {
  local path="${1:?path_required}"
  path_exists_or_link "$path" || return 0
  rm -rf "$path"
}

remove_link_if_points_to_root() {
  local path="${1:?path_required}"
  local resolved=""
  [[ -L "$path" ]] || return 0
  resolved="$(readlink -f "$path" 2>/dev/null || true)"
  [[ -n "$resolved" && "$resolved" == "$ROOT_DIR/"* ]] || return 0
  rm -f "$path"
}

remove_previous_release_backups() {
  local release_parent="${1:?release_parent_required}"
  local backup_path=""
  shopt -s nullglob
  for backup_path in "$release_parent"/.chimera-previous.*; do
    remove_path_if_present "$backup_path"
  done
  shopt -u nullglob
}

uninstall_release_tree() {
  local release_parent=""
  release_parent="$(dirname "$ROOT_DIR")"
  remove_link_if_points_to_root "$LOCAL_BIN_DIR/chimera"
  remove_link_if_points_to_root "$LOCAL_BIN_DIR/chimera.sh"
  remove_link_if_points_to_root "$LOCAL_BIN_DIR/chimera-sh"
  remove_systemd_user_unit_link "$RUNTIME_SERVICE_UNIT"
  remove_systemd_user_unit_link "$NODE_SERVICE_UNIT"
  remove_systemd_user_unit_link "$DATAPATH_SERVICE_UNIT"
  remove_systemd_user_unit_link "$LEGACY_NODE_COMPAT_SERVICE_UNIT"
  remove_systemd_user_unit_link "$LEGACY_DATAPATH_COMPAT_SERVICE_UNIT"
  remove_path_if_present "$SYSTEMD_USER_DIR/$RUNTIME_SERVICE_UNIT"
  remove_path_if_present "$SYSTEMD_USER_DIR/$NODE_SERVICE_UNIT"
  remove_path_if_present "$SYSTEMD_USER_DIR/$DATAPATH_SERVICE_UNIT"
  remove_path_if_present "$SYSTEMD_USER_DIR/$LEGACY_NODE_COMPAT_SERVICE_UNIT"
  remove_path_if_present "$SYSTEMD_USER_DIR/$LEGACY_DATAPATH_COMPAT_SERVICE_UNIT"
  remove_path_if_present "$APPLICATIONS_DIR/chimera-control-gui.desktop"
  remove_path_if_present "$APPLICATIONS_DIR/chimera-control.desktop"
  remove_path_if_present "$CHIMERA_CONFIG_DIR"
  remove_path_if_present "$CHIMERA_CACHE_DIR"
  remove_path_if_present "$ROOT_DIR"
  remove_previous_release_backups "$release_parent"
}

trim_ascii_line() {
  local s="${1:-}"
  s="${s#"${s%%[![:space:]]*}"}"
  s="${s%"${s##*[![:space:]]}"}"
  printf '%s' "$s"
}

read_config_value() {
  local file="${1:?file_required}"
  local prefix="${2:?prefix_required}"
  local key="${3:?key_required}"
  awk -F= -v p="$prefix" -v k="$key" '
    $1==p k {
      sub(/^[^=]*=[[:space:]]*/, "", $0);
      print $0;
      exit
    }
  ' "$file" 2>/dev/null || true
}

count_config_prefix() {
  local file="${1:?file_required}"
  local prefix="${2:?prefix_required}"
  awk -v p="$prefix" 'index($0, p)==1 {n++} END {print n+0}' "$file" 2>/dev/null || echo 0
}

append_unique_line() {
  local file="${1:?file_required}"
  local line="${2:-}"
  [[ -n "$line" ]] || return 0
  ensure_parent_dir "$file"
  touch "$file"
  if ! grep -Fxq "$line" "$file"; then
    printf '%s\n' "$line" >>"$file"
  fi
}

remove_exact_line() {
  local file="${1:?file_required}"
  local line="${2:-}"
  [[ -f "$file" ]] || return 0
  local tmp
  tmp="$(mktemp)"
  grep -Fxv "$line" "$file" >"$tmp" 2>/dev/null || true
  mv "$tmp" "$file"
}

normalize_domain_token() {
  local token="${1:-}"
  token="$(trim_ascii_line "$token")"
  token="${token#*://}"
  token="${token#*@}"
  token="${token%%/*}"
  token="${token%%\?*}"
  token="${token%%\#*}"
  token="${token%%:*}"
  token="${token,,}"
  printf '%s' "$token"
}

extract_domains_from_text() {
  local text="${1:-}"
  grep -oE 'https?://[^[:space:]]+|[[:alnum:]-]+(\.[[:alnum:]-]+)+' <<<"$text" 2>/dev/null \
    | while IFS= read -r token; do
        token="$(normalize_domain_token "$token")"
        [[ -z "$token" ]] && continue
        [[ "$token" == *.* ]] || continue
        printf '%s\n' "$token"
      done
}

merge_unique_domain_sources() {
  local out_file="${1:?file_required}"
  shift || true
  local tmp
  tmp="$(mktemp)"
  : >"$tmp"
  local src
  for src in "$@"; do
    [[ -f "$src" ]] || continue
    while IFS= read -r line; do
      line="$(trim_ascii_line "$line")"
      [[ -z "$line" ]] && continue
      [[ "$line" == \#* ]] && continue
      normalize_domain_token "$line" >>"$tmp"
    done <"$src"
  done
  if [[ -f "$APP_ROUTES_FILE" ]]; then
    while IFS= read -r line; do
      [[ "$line" == app:*=* ]] || continue
      local command_part="${line#*=}"
      extract_domains_from_text "$command_part" >>"$tmp"
    done <"$APP_ROUTES_FILE"
  fi
  awk 'NF { print tolower($0) }' "$tmp" | sort -u >"$out_file"
  rm -f "$tmp"
}

runtime_state_is_up() {
  if systemd_user_ready; then
    local node_state datapath_state
    node_state="$(systemctl --user is-active "$NODE_SERVICE_UNIT" 2>/dev/null || true)"
    datapath_state="$(systemctl --user is-active "$DATAPATH_SERVICE_UNIT" 2>/dev/null || true)"
    if node_config_ready; then
      [[ "$node_state" == "active" && "$datapath_state" == "active" ]] || return 1
      if peer_egress_bound_transit_requested; then
        ensure_bound_transit_start_contract >/dev/null 2>&1 || return 1
      fi
      return 0
    else
      [[ "$node_state" == "active" ]]
    fi
    return $?
  fi
  if node_config_ready; then
    pidfile_running "$(peer_egress_pid_path)" && pidfile_running "$(transparent_runtime_pid_path)" || return 1
    if peer_egress_bound_transit_requested; then
      ensure_bound_transit_start_contract >/dev/null 2>&1 || return 1
    fi
    return 0
  fi
  if pidfile_running "$(peer_egress_pid_path)"; then
    return 0
  fi
  return 1
}

read_runtime_service_state() {
  local unit="${1:?unit_required}"
  if systemd_user_ready; then
    systemctl --user is-active "$unit" 2>/dev/null || true
  else
    if [[ "$unit" == "$NODE_SERVICE_UNIT" || "$unit" == "$LEGACY_NODE_COMPAT_SERVICE_UNIT" ]] && pidfile_running "$(peer_egress_pid_path)"; then
      echo "active"
    elif [[ "$unit" == "$DATAPATH_SERVICE_UNIT" || "$unit" == "$LEGACY_DATAPATH_COMPAT_SERVICE_UNIT" ]] && pidfile_running "$(transparent_runtime_pid_path)"; then
      echo "active"
    else
      echo "unknown"
    fi
  fi
}

read_runtime_service_enable_state() {
  local unit="${1:?unit_required}"
  local wants_link="$SYSTEMD_USER_DEFAULT_TARGET_WANTS_DIR/$unit"
  if systemd_user_ready; then
    local enabled_state
    enabled_state="$(systemctl --user is-enabled "$unit" 2>/dev/null || true)"
    if [[ -n "$enabled_state" ]]; then
      printf '%s\n' "$enabled_state"
      return 0
    fi
  fi
  if [[ -L "$wants_link" ]]; then
    echo "enabled"
  else
    echo "disabled"
  fi
}

remove_systemd_user_unit_link() {
  local unit="${1:?unit_required}"
  rm -f "$SYSTEMD_USER_DEFAULT_TARGET_WANTS_DIR/$unit"
}

service_route_override_state() {
  local id="${1:?id_required}"
  if [[ -f "$SERVICE_ROUTE_OVERRIDES_FILE" ]]; then
    awk -F= -v k="$id" '$1=="service_route_override[" k "]" {print $2; exit}' "$SERVICE_ROUTE_OVERRIDES_FILE" 2>/dev/null || true
  fi
}

set_service_route_override() {
  local id="${1:?id_required}"
  local state="${2:-enabled}"
  ensure_parent_dir "$SERVICE_ROUTE_OVERRIDES_FILE"
  touch "$SERVICE_ROUTE_OVERRIDES_FILE"
  local tmp
  tmp="$(mktemp)"
  awk -F= -v k="$id" -v s="$state" '
    $1=="service_route_override[" k "]" { print $1"="s; seen=1; next }
    { print }
    END {
      if (!seen) print "service_route_override[" k "]="s
    }
  ' "$SERVICE_ROUTE_OVERRIDES_FILE" >"$tmp"
  mv "$tmp" "$SERVICE_ROUTE_OVERRIDES_FILE"
}

delete_service_route_override() {
  local id="${1:?id_required}"
  [[ -f "$SERVICE_ROUTE_OVERRIDES_FILE" ]] || return 0
  local tmp
  tmp="$(mktemp)"
  awk -F= -v k="$id" '$1!="service_route_override[" k "]" { print }' "$SERVICE_ROUTE_OVERRIDES_FILE" >"$tmp"
  mv "$tmp" "$SERVICE_ROUTE_OVERRIDES_FILE"
}

resolve_app_command() {
  local app_id="${1:?app_id_required}"
  read_config_value "$APP_ROUTES_FILE" "app:" "$app_id"
}

resolve_app_env() {
  local app_id="${1:?app_id_required}"
  read_config_value "$APP_ROUTES_FILE" "app-env:" "$app_id"
}

resolve_service_name() {
  local service_id="${1:?service_id_required}"
  read_config_value "$APP_ROUTES_FILE" "service:" "$service_id"
}

resolve_service_env() {
  local service_id="${1:?service_id_required}"
  read_config_value "$APP_ROUTES_FILE" "service-env:" "$service_id"
}

list_config_ids() {
  local prefix="${1:?prefix_required}"
  awk -F= -v p="$prefix" '
    index($1, p)==1 {
      id=$1;
      sub("^" p, "", id);
      print id;
    }
  ' "$APP_ROUTES_FILE" 2>/dev/null | sort -u
}

run_shell_command_with_env() {
  local env_spec="${1:-}"
  shift || true
  local -a env_exports=()
  if [[ -n "$env_spec" ]]; then
    local entry
    IFS=';' read -r -a env_entries <<<"$env_spec"
    for entry in "${env_entries[@]}"; do
      entry="$(trim_ascii_line "$entry")"
      [[ -z "$entry" ]] && continue
      env_exports+=("$entry")
    done
  fi
  if [[ "${#env_exports[@]}" -gt 0 ]]; then
    env "${env_exports[@]}" bash -lc "$*"
  else
    bash -lc "$*"
  fi
}

site_auto_discover_run() {
  ensure_parent_dir "$SITE_DISCOVERY_DOMAINS_FILE"
  local tmp
  tmp="$(mktemp)"
  : >"$tmp"
  local file
  for file in "$SITE_AUTO_SEEDS_FILE" "$MANUAL_TRANSIT_DOMAINS_FILE" "$LEGACY_MANUAL_COMPAT_DOMAINS_FILE" "$ADAPTIVE_DOMAINS_FILE"; do
    if [[ -f "$file" ]]; then
      while IFS= read -r line; do
        line="$(trim_ascii_line "$line")"
        [[ -z "$line" ]] && continue
        [[ "$line" == \#* ]] && continue
        normalize_domain_token "$line" >>"$tmp"
      done <"$file"
    fi
  done
  if [[ -f "$APP_ROUTES_FILE" ]]; then
    while IFS= read -r line; do
      [[ "$line" == app:*=* ]] || continue
      extract_domains_from_text "${line#*=}" >>"$tmp"
    done <"$APP_ROUTES_FILE"
  fi
  if command -v journalctl >/dev/null 2>&1; then
    local lookback="${SITE_AUTO_DISCOVERY_LOOKBACK_SEC:-120}"
    journalctl -u systemd-resolved --since "-${lookback} sec" 2>/dev/null \
      | grep -oE '[[:alnum:]-]+(\.[[:alnum:]-]+)+' \
      | while IFS= read -r token; do
          token="$(normalize_domain_token "$token")"
          [[ -z "$token" ]] && continue
          printf '%s\n' "$token"
        done >>"$tmp" || true
  fi
  awk 'NF { print tolower($0) }' "$tmp" | sort -u >"$SITE_DISCOVERY_DOMAINS_FILE"
  local count
  count="$(wc -l <"$SITE_DISCOVERY_DOMAINS_FILE" 2>/dev/null || echo 0)"
  echo "site_auto_discover_status=ok"
  echo "site_auto_discover_count=$count"
  echo "site_auto_discover_file=$SITE_DISCOVERY_DOMAINS_FILE"
  rm -f "$tmp"
}

site_auto_bootstrap_run() {
  ensure_parent_dir "$ADAPTIVE_DOMAINS_FILE"
  local tmp
  tmp="$(mktemp)"
  : >"$tmp"
  local file
  for file in "$SITE_AUTO_SEEDS_FILE" "$MANUAL_TRANSIT_DOMAINS_FILE" "$LEGACY_MANUAL_COMPAT_DOMAINS_FILE" "$SITE_DISCOVERY_DOMAINS_FILE"; do
    if [[ -f "$file" ]]; then
      while IFS= read -r line; do
        line="$(trim_ascii_line "$line")"
        [[ -z "$line" ]] && continue
        [[ "$line" == \#* ]] && continue
        normalize_domain_token "$line" >>"$tmp"
      done <"$file"
    fi
  done
  if [[ -f "$APP_ROUTES_FILE" ]]; then
    while IFS= read -r line; do
      [[ "$line" == app:*=* ]] || continue
      extract_domains_from_text "${line#*=}" >>"$tmp"
    done <"$APP_ROUTES_FILE"
  fi
  awk 'NF { print tolower($0) }' "$tmp" | sort -u >"$ADAPTIVE_DOMAINS_FILE"
  rm -f "$tmp"
  if [[ -x "$AUTOFIX_SCRIPT" ]]; then
    bash "$AUTOFIX_SCRIPT" >/dev/null 2>&1 || true
  fi
  echo "site_auto_bootstrap_status=ok"
  echo "site_auto_bootstrap_domains=$(wc -l <"$ADAPTIVE_DOMAINS_FILE" 2>/dev/null || echo 0)"
  echo "site_auto_bootstrap_policy=$POLICY_FILE"
}

site_auto_resolve_run() {
  local ids=("$@")
  [[ "${#ids[@]}" -gt 0 ]] || {
    echo "site_auto_resolve_status=fail reason=no_domains"
    return 2
  }
  for id in "${ids[@]}"; do
    append_unique_line "$ADAPTIVE_DOMAINS_FILE" "$(normalize_domain_token "$id")"
  done
  site_auto_bootstrap_run
}

site_auto_status() {
  echo "site_auto_status=ok"
  echo "site_adaptive_db_file=$SITE_ADAPTIVE_DB_FILE"
  echo "site_discovery_file=$SITE_DISCOVERY_DOMAINS_FILE"
  echo "adaptive_domains_file=$ADAPTIVE_DOMAINS_FILE"
  echo "manual_transit_domains_file=$MANUAL_TRANSIT_DOMAINS_FILE"
  echo "legacy_manual_compat_domains_file=$LEGACY_MANUAL_COMPAT_DOMAINS_FILE"
  echo "adaptive_domains_count=$(count_noncomment_lines "$ADAPTIVE_DOMAINS_FILE")"
  echo "manual_transit_domains_count=$(count_noncomment_lines "$MANUAL_TRANSIT_DOMAINS_FILE")"
  echo "legacy_manual_compat_domains_count=$(count_noncomment_lines "$LEGACY_MANUAL_COMPAT_DOMAINS_FILE")"
  echo "discovered_domains_count=$(count_noncomment_lines "$SITE_DISCOVERY_DOMAINS_FILE")"
}

count_noncomment_lines() {
  local file="${1:?file_required}"
  [[ -f "$file" ]] || {
    echo 0
    return 0
  }
  awk 'NF && $0 !~ /^[[:space:]]*#/' "$file" | wc -l | tr -d '[:space:]'
}

site_list() {
  echo "manual_transit_domains_file=$MANUAL_TRANSIT_DOMAINS_FILE"
  echo "legacy_manual_compat_domains_file=$LEGACY_MANUAL_COMPAT_DOMAINS_FILE"
  echo "adaptive_domains_file=$ADAPTIVE_DOMAINS_FILE"
  echo "manual_transit_domains_count=$(count_noncomment_lines "$MANUAL_TRANSIT_DOMAINS_FILE")"
  echo "legacy_manual_compat_domains_count=$(count_noncomment_lines "$LEGACY_MANUAL_COMPAT_DOMAINS_FILE")"
  echo "adaptive_domains_count=$(count_noncomment_lines "$ADAPTIVE_DOMAINS_FILE")"
  echo "manual_transit_domains:"
  if [[ -f "$MANUAL_TRANSIT_DOMAINS_FILE" ]]; then
    awk 'NF && $0 !~ /^[[:space:]]*#/' "$MANUAL_TRANSIT_DOMAINS_FILE"
  fi
  echo "legacy_manual_compat_domains:"
  if [[ -f "$LEGACY_MANUAL_COMPAT_DOMAINS_FILE" ]]; then
    awk 'NF && $0 !~ /^[[:space:]]*#/' "$LEGACY_MANUAL_COMPAT_DOMAINS_FILE"
  fi
  echo "adaptive_domains:"
  if [[ -f "$ADAPTIVE_DOMAINS_FILE" ]]; then
    awk 'NF && $0 !~ /^[[:space:]]*#/' "$ADAPTIVE_DOMAINS_FILE"
  fi
}

site_add() {
  local domain
  local added=0
  for domain in "$@"; do
    domain="$(normalize_domain_token "$domain")"
    [[ -z "$domain" ]] && continue
    append_unique_line "$MANUAL_TRANSIT_DOMAINS_FILE" "$domain"
    added=$((added + 1))
  done
  site_auto_bootstrap_run >/dev/null 2>&1 || true
  echo "site_add_status=ok count=$added"
}

site_remove() {
  local domain
  local removed=0
  for domain in "$@"; do
    domain="$(normalize_domain_token "$domain")"
    [[ -z "$domain" ]] && continue
    remove_exact_line "$MANUAL_TRANSIT_DOMAINS_FILE" "$domain"
    remove_exact_line "$LEGACY_MANUAL_COMPAT_DOMAINS_FILE" "$domain"
    removed=$((removed + 1))
  done
  site_auto_bootstrap_run >/dev/null 2>&1 || true
  echo "site_remove_status=ok count=$removed"
}

site_auto_watch_run_once() {
  clear_stale_publication_runtime_state
  local peer_update_status="skipped"
  local discover_status="ok"
  local bootstrap_status="ok"
  local bindings_status="ok"
  local discovery_status="ok"
  local watch_rc=0
  local bindings_mode="best-effort"
  refresh_node_peer_target_from_bootstrap >/dev/null 2>&1 || true
  if peer_update_runtime_configured; then
    if start_peer_update_runtime_with_retry >/dev/null 2>&1; then
      peer_update_status="ok"
    else
      peer_update_status="failed"
      watch_rc=1
    fi
  fi
  site_auto_discover_run >/dev/null 2>&1 || {
    discover_status="failed"
    watch_rc=1
  }
  site_auto_bootstrap_run >/dev/null 2>&1 || {
    bootstrap_status="failed"
    watch_rc=1
  }
  if peer_egress_bound_transit_requested; then
    bindings_mode="strict"
  fi
  publish_peer_egress_transit_lane_bindings_from_control_plane "$bindings_mode" || {
    bindings_status="failed"
    watch_rc=1
  }
  publish_mesh_discovery_snapshot strict >/dev/null 2>&1 || {
    discovery_status="failed"
    watch_rc=1
  }
  if [[ "$watch_rc" -eq 0 ]]; then
    echo "site_auto_watch_run_once=ok peer_update_publish=$peer_update_status"
    return 0
  fi
  echo "site_auto_watch_run_once=partial peer_update_publish=$peer_update_status site_auto_discover=$discover_status site_auto_bootstrap=$bootstrap_status transit_lane_bindings_publish=$bindings_status discovery_publish=$discovery_status"
  return 1
}

site_auto_watch_failure_budget() {
  local budget="${SITE_AUTOWATCH_FAILURE_BUDGET:-3}"
  if ! [[ "$budget" =~ ^[0-9]+$ ]]; then
    budget="3"
  fi
  if (( budget < 1 )); then
    budget="1"
  fi
  printf '%s\n' "$budget"
}

site_auto_watch_loop() {
  local consecutive_failures=0
  local failure_budget
  failure_budget="$(site_auto_watch_failure_budget)"
  while true; do
    if site_auto_watch_run_once >/dev/null 2>&1; then
      consecutive_failures=0
    else
      consecutive_failures=$((consecutive_failures + 1))
      if (( consecutive_failures >= failure_budget )); then
        echo "site_auto_watch_loop=fail consecutive_failures=$consecutive_failures failure_budget=$failure_budget" >&2
        return 1
      fi
    fi
    sleep "$SITE_AUTOWATCH_INTERVAL_SEC"
  done
}

site_auto_watch_enabled_requested() {
  [[ "${SITE_AUTOWATCH_ENABLED}" == "1" ]]
}

site_auto_watch_systemd_installed() {
  systemd_user_ready || return 1
  [[ -f "$SYSTEMD_USER_DIR/$SITE_AUTOWATCH_SERVICE_UNIT" ]]
}

site_auto_watch_status() {
  if site_auto_watch_systemd_installed; then
    local service_state
    service_state="$(systemctl --user is-active "$SITE_AUTOWATCH_SERVICE_UNIT" 2>/dev/null || true)"
    if [[ "$service_state" == "active" ]]; then
      echo "site_auto_watch_status=running mode=systemd_user service_state=$service_state interval_sec=$SITE_AUTOWATCH_INTERVAL_SEC"
    else
      echo "site_auto_watch_status=stopped mode=systemd_user service_state=${service_state:-unknown}"
    fi
    return 0
  fi
  local pid="unknown"
  if [[ -f "$SITE_AUTOWATCH_PID_FILE" ]]; then
    pid="$(tr -d '[:space:]' <"$SITE_AUTOWATCH_PID_FILE")"
    if [[ -n "$pid" ]] && kill -0 "$pid" >/dev/null 2>&1; then
      echo "site_auto_watch_status=running mode=direct pid=$pid interval_sec=$SITE_AUTOWATCH_INTERVAL_SEC"
      return 0
    fi
  fi
  echo "site_auto_watch_status=stopped mode=direct"
}

site_auto_watch_start() {
  if ! site_auto_watch_enabled_requested; then
    echo "site_auto_watch_status=disabled"
    return 0
  fi
  if site_auto_watch_systemd_installed; then
    systemctl --user start "$SITE_AUTOWATCH_SERVICE_UNIT" >/dev/null 2>&1 || {
      echo "site_auto_watch_status=failed mode=systemd_user"
      return 1
    }
    if wait_for_systemd_unit_stable_active "$SITE_AUTOWATCH_SERVICE_UNIT" 20 3; then
      echo "site_auto_watch_status=running mode=systemd_user service_state=active interval_sec=$SITE_AUTOWATCH_INTERVAL_SEC"
      return 0
    fi
    echo "site_auto_watch_status=failed mode=systemd_user"
    return 1
  fi
  if [[ -f "$SITE_AUTOWATCH_PID_FILE" ]]; then
    local pid
    pid="$(tr -d '[:space:]' <"$SITE_AUTOWATCH_PID_FILE" 2>/dev/null || true)"
    if [[ -n "$pid" ]] && kill -0 "$pid" >/dev/null 2>&1; then
      echo "site_auto_watch_status=running mode=direct pid=$pid interval_sec=$SITE_AUTOWATCH_INTERVAL_SEC"
      return 0
    fi
  fi
  ensure_parent_dir "$SITE_AUTOWATCH_PID_FILE"
  nohup "$SCRIPT_PATH" __site-auto-watch-loop >/dev/null 2>&1 &
  local pid=$!
  printf '%s\n' "$pid" >"$SITE_AUTOWATCH_PID_FILE"
  if kill -0 "$pid" >/dev/null 2>&1; then
    echo "site_auto_watch_status=started mode=direct pid=$pid interval_sec=$SITE_AUTOWATCH_INTERVAL_SEC"
    return 0
  fi
  rm -f "$SITE_AUTOWATCH_PID_FILE"
  echo "site_auto_watch_status=failed mode=direct"
  return 1
}

site_auto_watch_stop() {
  if site_auto_watch_systemd_installed; then
    systemctl --user stop "$SITE_AUTOWATCH_SERVICE_UNIT" >/dev/null 2>&1 || true
  fi
  if [[ -f "$SITE_AUTOWATCH_PID_FILE" ]]; then
    local pid
    pid="$(tr -d '[:space:]' <"$SITE_AUTOWATCH_PID_FILE" 2>/dev/null || true)"
    if [[ -n "$pid" ]]; then
      kill "$pid" >/dev/null 2>&1 || true
    fi
    rm -f "$SITE_AUTOWATCH_PID_FILE"
  fi
  echo "site_auto_watch_status=stopped"
}

app_routes_status() {
  local app_routes_count service_routes_count
  app_routes_count="$(count_config_prefix "$APP_ROUTES_FILE" "app:")"
  service_routes_count="$(count_config_prefix "$APP_ROUTES_FILE" "service:")"
  echo "app_routes_file=$APP_ROUTES_FILE"
  echo "app_routes_count=$app_routes_count"
  echo "service_routes_count=$service_routes_count"
  echo "service_route_overrides_file=$SERVICE_ROUTE_OVERRIDES_FILE"
  if [[ -f "$APP_ROUTES_FILE" ]]; then
    awk -F= '
      $1 ~ /^app-env:/ {
        id=$1; sub(/^app-env:/, "", id); sub(/[[:space:]]+$/, "", id);
        sub(/^[[:space:]]*/, "", $2);
        print "app_env[" id "]=" $2
        next
      }
      $1 ~ /^app:/ {
        id=$1; sub(/^app:/, "", id); sub(/[[:space:]]+$/, "", id);
        sub(/^[[:space:]]*/, "", $2);
        print "app_route[" id "]=" $2
        next
      }
      $1 ~ /^service-env:/ {
        id=$1; sub(/^service-env:/, "", id); sub(/[[:space:]]+$/, "", id);
        sub(/^[[:space:]]*/, "", $2);
        print "service_env[" id "]=" $2
        next
      }
      $1 ~ /^service:/ {
        id=$1; sub(/^service:/, "", id); sub(/[[:space:]]+$/, "", id);
        sub(/^[[:space:]]*/, "", $2);
        print "service_route[" id "]=" $2
        next
      }
    ' "$APP_ROUTES_FILE"
  fi
  if [[ -f "$SERVICE_ROUTE_OVERRIDES_FILE" ]]; then
    awk -F= '
      $1 ~ /^service_route_override\[/ {
        print
      }
    ' "$SERVICE_ROUTE_OVERRIDES_FILE"
  fi
}

resolve_service_ids_for_args() {
  local arg matched
  for arg in "$@"; do
    matched=""
    if [[ -f "$APP_ROUTES_FILE" ]]; then
      matched="$(awk -F= -v id="$arg" '
        $1=="service:" id { print id; exit }
      ' "$APP_ROUTES_FILE" 2>/dev/null || true)"
      if [[ -z "$matched" ]]; then
        matched="$(awk -F= -v svc="$arg" '
          $1 ~ /^service:/ {
            id=$1; sub(/^service:/, "", id);
            if ($2 == svc) { print id; exit }
          }
        ' "$APP_ROUTES_FILE" 2>/dev/null || true)"
      fi
    fi
    [[ -n "$matched" ]] && printf '%s\n' "$matched" || printf '%s\n' "$arg"
  done
}

service_route_enable() {
  legacy_app_workflow_disabled "service_route_enable"
}

service_route_disable() {
  legacy_app_workflow_disabled "service_route_disable"
}

service_route_enable_running() {
  legacy_app_workflow_disabled "service_route_enable_running"
}

verify_service() {
  legacy_app_workflow_disabled "verify_service"
}

run_app() {
  legacy_app_workflow_disabled "run_app"
}

verify_app() {
  legacy_app_workflow_disabled "verify_app"
}

verify_cmd() {
  legacy_app_workflow_disabled "verify_cmd"
}

legacy_app_workflow_disabled() {
  local command_name="${1:?command_name_required}"
  echo "${command_name}_status=fail reason=legacy_lab_only_not_datapath_evidence"
  echo "product_datapath_evidence=false"
  echo "hint=use_normal_application_workflow_after_transparent_datapath_start"
  return 2
}

apps_running() {
  ps -eo comm= 2>/dev/null | awk 'NF {print}' | sort -u
}

services_running() {
  if systemd_user_ready; then
    systemctl --user list-units --type=service --state=running --no-legend 2>/dev/null | awk '{print $1}'
    return 0
  fi
  ps -eo comm= 2>/dev/null | awk 'NF {print}' | sort -u
}

redact_diagnostic_stream() {
  sed -E \
    -e 's#([A-Za-z0-9._%+-]+://)[^[:space:]"'"'"'<>]+#\1<redacted>#g' \
    -e 's#([Aa]uthorization:[[:space:]]*[Bb]earer)[[:space:]]+[^[:space:]"'"'"'<>]+#\1 <redacted>#g' \
    -e 's#([Pp]ass(word)?|TOKEN|Token|token|SECRET|Secret|secret|PRIVATE_KEY|PrivateKey|private_key)[[:space:]]*=[[:space:]]*([^[:space:]"'"'"']+)#\1=<redacted>#g' \
    -e 's#([Pp]ass(word)?|TOKEN|Token|token|SECRET|Secret|secret|PRIVATE_KEY|PrivateKey|private_key)[[:space:]]*=[[:space:]]*("([^"]*)"|'\''([^'\'']*)'\'')#\1=<redacted>#g' \
    -e 's#([Pp]ass(word)?|TOKEN|Token|token|SECRET|Secret|secret|PRIVATE_KEY|PrivateKey|private_key)[[:space:]]*:[[:space:]]*([^[:space:]"'"'"']+)#\1:<redacted>#g' \
    -e 's#([Pp]ass(word)?|TOKEN|Token|token|SECRET|Secret|secret|PRIVATE_KEY|PrivateKey|private_key)[[:space:]]*:[[:space:]]*("([^"]*)"|'\''([^'\'']*)'\'')#\1:<redacted>#g' \
    -e 's#(CHIMERA_[A-Za-z0-9_]*(TOKEN|SECRET|PRIVATE_KEY|PASSWORD)[A-Za-z0-9_]*)[[:space:]]*=[[:space:]]*([^[:space:]"'"'"']+)#\1=<redacted>#g' \
    -e 's#(CHIMERA_[A-Za-z0-9_]*(TOKEN|SECRET|PRIVATE_KEY|PASSWORD)[A-Za-z0-9_]*)[[:space:]]*=[[:space:]]*("([^"]*)"|'\''([^'\'']*)'\'')#\1=<redacted>#g' \
    -e 's#(endpoint|carrier_addr|carrier_server_name|server_name|listen_addr|server|target|listen|remote|host)[[:space:]]*=[[:space:]]*([^[:space:]"'"'"']+)#\1=<redacted>#Ig' \
    -e 's#"(endpoint|carrier_addr|carrier_server_name|server|target|listen|listen_addr|remote|host)"[[:space:]]*:[[:space:]]*"[^"]*"#"\1":"<redacted>"#Ig' \
    -e 's#"(password|token|secret|private_key|privateKey)"[[:space:]]*:[[:space:]]*"[^"]*"#"\1":"<redacted>"#Ig' \
    -e 's#\[[0-9A-Fa-f:]{2,}\](:[0-9]{1,5})?#<redacted-ip>#g' \
    -e 's#[0-9]{1,3}(\.[0-9]{1,3}){3}(:[0-9]{1,5})?#<redacted-ip>#g' \
    -e 's#([A-Za-z0-9._-]+\.)+[A-Za-z]{2,}(:[0-9]{1,5})?#<redacted-host>#g' \
    -e 's#(/home/)[^/[:space:]]+#\1<redacted-user>#g'
}

logs_tail() {
  local lines="${1:-200}"
  if ! [[ "$lines" =~ ^[0-9]+$ ]]; then
    lines=200
  fi
  echo "=== node log: <redacted> ==="
  echo "node_log_path_state=present"
  tail -n "$lines" "$NODE_LOG" 2>/dev/null | redact_diagnostic_stream || true
  echo "=== transparent-runtime log: <redacted> ==="
  echo "transparent_runtime_log_path_state=present"
  tail -n "$lines" "$DATAPATH_LOG" 2>/dev/null | redact_diagnostic_stream || true
  echo "=== autofix log: <redacted> ==="
  echo "autofix_log_path_state=present"
  tail -n "$lines" "$AUTOFIX_LOG_FILE" 2>/dev/null | redact_diagnostic_stream || true
}

write_doctor_fail_json() {
  local out="${1:?out_required}"
  local reason="${2:?reason_required}"
  local node_ready="${3:-false}"
  mkdir -p "$(dirname "$out")" >/dev/null 2>&1 || true
  cat >"$out" <<EOF
{"status":"fail","kind":"doctor","message_en":"Doctor check is blocked until CHIMERA runtime state is ready.","message_ru":"Проверка doctor заблокирована до готовности состояния CHIMERA.","reason":"${reason}","secrets":"<redacted>","node_config_ready":${node_ready},"network_state":"not_modified"}
EOF
}

doctor_run() {
  local out_file="$ROOT_DIR/docs/doctor_latest.json"
  local config_path rc=0
  mkdir -p "$(dirname "$out_file")" >/dev/null 2>&1 || true
  if ! node_config_ready; then
    write_doctor_fail_json "$out_file" "node_endpoint_unconfigured" "false"
    echo "doctor_status=fail reason=node_endpoint_unconfigured" >&2
    return 2
  fi
  if peer_egress_bound_transit_requested \
    && ! ensure_bound_transit_start_contract >/dev/null 2>&1; then
    write_doctor_fail_json "$out_file" "bound_transit_unready" "true"
    echo "doctor_status=fail reason=bound_transit_unready" >&2
    return 2
  fi
  config_path="$(node_config_path)"
  run_chimera_cli doctor --config "$config_path" --json --out "$out_file" || rc=$?
  if [[ "$rc" -eq 0 ]]; then
    echo "doctor_status=ok"
    return 0
  fi
  echo "doctor_status=fail exit=$rc" >&2
  return "$rc"
}

start_runtime() {
  if ! load_bootstrap_env_if_present; then
    site_auto_watch_stop >/dev/null 2>&1 || true
    echo "start_status=fail mode=preflight node_runtime=stopped transparent_runtime=stopped reason=bootstrap_env_invalid"
    return 2
  fi
  ensure_base_path
  ensure_runtime_log_paths
  ensure_mesh_bootstrap_env
  local recovery_rc=0
  recover_saved_runtime_state_if_present || recovery_rc=$?
  if [[ "$recovery_rc" -ne 0 ]]; then
    site_auto_watch_stop >/dev/null 2>&1 || true
    if [[ "$recovery_rc" -eq 2 ]]; then
      echo "start_status=fail mode=preflight node_runtime=stopped transparent_runtime=stopped datapath_proof=$PRESTART_SAVED_STATE_PROOF recovery_state=$PRESTART_SAVED_STATE_RECOVERY reason=saved_state_invalid"
      return 2
    fi
    echo "start_status=fail mode=preflight node_runtime=stopped transparent_runtime=stopped datapath_proof=$PRESTART_SAVED_STATE_PROOF recovery_state=$PRESTART_SAVED_STATE_RECOVERY reason=saved_state_recovery_failed"
    return 1
  fi
  clear_stale_publication_runtime_state
  refresh_node_peer_target_from_bootstrap >/dev/null 2>&1 || true
  heal_node_peer_egress_env_bindings
  heal_node_carrier_addr_from_peer_egress_env >/dev/null 2>&1 || true
  ensure_peer_egress_local_listen_aligned
  ensure_peer_egress_service_fwmark
  ensure_transparent_runtime_service_fwmark
  # Best-effort listener repair/cleanup must not abort start under set -e.
  set +e
  if node_listener_bindings_need_preemptive_repair; then
    repair_node_listener_bindings_for_retry >/dev/null 2>&1 || true
  else
    clear_node_listener_runtime_overrides || true
  fi
  set -e
  if ! ensure_bound_transit_start_contract; then
    site_auto_watch_stop >/dev/null 2>&1 || true
    echo "start_status=fail mode=preflight node_runtime=stopped transparent_runtime=stopped mesh_ready=false reason=bound_transit_unready"
    return 2
  fi
  local node_config_is_ready=0
  if node_config_ready; then
    node_config_is_ready=1
  elif ! node_listener_only_bootstrap_ready; then
    site_auto_watch_stop >/dev/null 2>&1 || true
    echo "start_status=fail mode=preflight node_runtime=stopped transparent_runtime=stopped reason=datapath_unconfigured"
    return 2
  fi
  local route_fwmark
  route_fwmark="$(route_fwmark_env_value)"
  : "${CHIMERA_APPLY_TUN:=true}"
  : "${CHIMERA_APPLY_ROUTE:=true}"
  : "${CHIMERA_APPLY_DNS:=true}"
  export CHIMERA_APPLY_TUN CHIMERA_APPLY_ROUTE CHIMERA_APPLY_DNS
  if ! cleanup_stale_tun_without_state; then
    site_auto_watch_stop >/dev/null 2>&1 || true
    echo "start_status=fail mode=preflight node_runtime=stopped transparent_runtime=stopped reason=stale_tun_cleanup_failed"
    return 1
  fi
  if systemd_user_ready; then
    systemctl --user daemon-reload >/dev/null 2>&1 || true
    local systemd_start_rc=0
    local systemd_publication_ready=1
    systemctl --user start "$NODE_SERVICE_UNIT" >/dev/null 2>&1 || systemd_start_rc=$?
    local node_state node_runtime node_status
    if wait_for_systemd_unit_stable_active "$NODE_SERVICE_UNIT" 20 5; then
      node_state="active"
      node_runtime="running"
      node_status="started"
    else
      node_state="$(systemctl --user is-active "$NODE_SERVICE_UNIT" 2>/dev/null || true)"
      node_runtime="stopped"
      node_status="failed"
    fi
    if [[ "$node_status" != "started" ]] && repair_node_listener_bindings_for_retry; then
      systemctl --user stop "$NODE_SERVICE_UNIT" >/dev/null 2>&1 || true
      systemd_start_rc=0
      systemctl --user start "$NODE_SERVICE_UNIT" >/dev/null 2>&1 || systemd_start_rc=$?
      if wait_for_systemd_unit_stable_active "$NODE_SERVICE_UNIT" 20 5; then
        node_state="active"
        node_runtime="running"
        node_status="started"
      else
        node_state="$(systemctl --user is-active "$NODE_SERVICE_UNIT" 2>/dev/null || true)"
        node_runtime="stopped"
        node_status="failed"
      fi
    fi
    if [[ "$node_status" != "started" ]]; then
      stop_partial_runtime_components "systemd_user"
      echo "start_status=fail mode=systemd_user node_runtime=$node_runtime node=$node_status transparent_runtime=stopped reason=node_service_failed systemctl_start_rc=$systemd_start_rc"
      return 1
    fi
    if [[ "$node_config_is_ready" -eq 0 ]]; then
      local listener_only_fail_closed="false"
      local listener_only_exit_rc=0
      local listener_only_node_runtime="$node_runtime"
      local listener_only_node_status="$node_status"
      local listener_only_transparent_runtime="skipped"
      site_auto_watch_stop >/dev/null 2>&1 || true
      if partial_start_fail_closed; then
        stop_partial_runtime_components "systemd_user"
        listener_only_fail_closed="true"
        listener_only_exit_rc=2
        listener_only_node_runtime="stopped"
        listener_only_node_status="stopped"
        listener_only_transparent_runtime="stopped"
      else
        if mesh_discovery_source_present; then
          site_auto_watch_start >/dev/null 2>&1 || true
        fi
        refresh_runtime_publication_after_node_start >/dev/null 2>&1 || true
      fi
      echo "start_status=partial mode=listener_only node_runtime=$listener_only_node_runtime node=$listener_only_node_status transparent_runtime=$listener_only_transparent_runtime datapath_apply=skipped recovery_state=$PRESTART_SAVED_STATE_RECOVERY peer_update_publish=$START_RUNTIME_PEER_UPDATE_STATUS transit_lane_bindings_publish=$START_RUNTIME_TRANSIT_LANE_BINDINGS_STATUS discovery_publish=$START_RUNTIME_DISCOVERY_PUBLISH_STATUS mesh_ready=false fail_closed=$listener_only_fail_closed reason=node_endpoint_unconfigured_listener_only"
      return "$listener_only_exit_rc"
    fi
    if node_config_self_loop_target; then
      local self_loop_fail_closed="false"
      local self_loop_exit_rc=0
      local self_loop_node_runtime="$node_runtime"
      local self_loop_node_status="$node_status"
      local self_loop_transparent_runtime="skipped"
      site_auto_watch_stop >/dev/null 2>&1 || true
      if partial_start_fail_closed; then
        stop_partial_runtime_components "systemd_user"
        self_loop_fail_closed="true"
        self_loop_exit_rc=2
        self_loop_node_runtime="stopped"
        self_loop_node_status="stopped"
        self_loop_transparent_runtime="stopped"
      else
        if mesh_discovery_source_present; then
          site_auto_watch_start >/dev/null 2>&1 || true
        fi
        refresh_runtime_publication_after_node_start >/dev/null 2>&1 || true
      fi
      echo "start_status=partial mode=listener_only node_runtime=$self_loop_node_runtime node=$self_loop_node_status transparent_runtime=$self_loop_transparent_runtime datapath_apply=skipped recovery_state=$PRESTART_SAVED_STATE_RECOVERY peer_update_publish=$START_RUNTIME_PEER_UPDATE_STATUS transit_lane_bindings_publish=$START_RUNTIME_TRANSIT_LANE_BINDINGS_STATUS discovery_publish=$START_RUNTIME_DISCOVERY_PUBLISH_STATUS mesh_ready=false fail_closed=$self_loop_fail_closed reason=self_loop_listener_only"
      return "$self_loop_exit_rc"
    fi
    local transparent_start_rc=0
    systemctl --user start "$DATAPATH_SERVICE_UNIT" >/dev/null 2>&1 || transparent_start_rc=$?
    local transparent_state transparent_runtime transparent_status
    if wait_for_systemd_unit_stable_active "$DATAPATH_SERVICE_UNIT" 20 5; then
      transparent_state="active"
      transparent_runtime="running"
      transparent_status="started"
    else
      transparent_state="$(systemctl --user is-active "$DATAPATH_SERVICE_UNIT" 2>/dev/null || true)"
      transparent_runtime="stopped"
      transparent_status="failed"
    fi
    if [[ "$transparent_status" != "started" ]]; then
      stop_partial_runtime_components "systemd_user"
      echo "start_status=fail mode=systemd_user node_runtime=stopped node=$node_status transparent_runtime=$transparent_runtime reason=transparent_service_failed systemctl_start_rc=$transparent_start_rc"
      return 1
    fi
    local systemd_datapath_apply_status="skipped"
    local systemd_datapath_apply_rc="0"
    local systemd_datapath_proof_status="skipped"
    local systemd_datapath_rollback_status="skipped"
    local systemd_datapath_rollback_rc="0"
    if ! remove_state_file_for_datapath_apply; then
      systemctl --user stop "$DATAPATH_SERVICE_UNIT" "$NODE_SERVICE_UNIT" >/dev/null 2>&1 || true
      echo "start_status=fail mode=systemd_user node_runtime=stopped node=$node_status transparent_runtime=stopped datapath_apply=skipped datapath_proof=stale_state_cleanup_failed reason=stale_state_cleanup_failed"
      return 1
    fi
    local -a systemd_up_args=(
      --config "$(node_config_path)"
      --state-file "$STATE_FILE"
      --apply-tun "${CHIMERA_APPLY_TUN}"
    )
    if [[ "${CHIMERA_APPLY_ROUTE}" == "true" ]]; then
      systemd_up_args+=(
        --apply-route true
        --route-policy true
        --route-table 51820
        --route-rule-priority 11000
        --route-fwmark "$route_fwmark"
        --route-cidr "$CHIMERA_ROUTE_CIDR"
      )
    else
      systemd_up_args+=(--apply-route false)
    fi
    systemd_up_args+=(--apply-dns "${CHIMERA_APPLY_DNS}")
    if [[ "${CHIMERA_APPLY_TUN}" == "false" && "${CHIMERA_APPLY_ROUTE}" == "false" && "${CHIMERA_APPLY_DNS}" == "false" ]]; then
      systemd_up_args+=(--skip-connect-check true)
    fi
    if run_chimera_cli_up_with_retry "${systemd_up_args[@]}" >/dev/null 2>&1; then
      systemd_datapath_apply_status="ok"
    else
      systemd_datapath_apply_rc=$?
      systemd_datapath_apply_status="failed"
    fi
    if [[ "$systemd_datapath_apply_status" == "ok" ]]; then
      systemd_datapath_proof_status="$(datapath_apply_proof_state || true)"
      if [[ "$systemd_datapath_proof_status" != "ok" ]]; then
        if [[ "$systemd_datapath_proof_status" == "network_not_modified" \
          && "${CHIMERA_APPLY_TUN}" == "false" \
          && "${CHIMERA_APPLY_ROUTE}" == "false" \
          && "${CHIMERA_APPLY_DNS}" == "false" ]]; then
          systemd_datapath_proof_status="ok"
        else
          systemd_datapath_apply_status="unverified"
        fi
      fi
    fi
    if [[ "$systemd_datapath_apply_status" != "ok" ]]; then
      site_auto_watch_stop >/dev/null 2>&1 || true
      if run_chimera_cli rollback recover \
        --state-file "$STATE_FILE" >/dev/null 2>&1; then
        systemd_datapath_rollback_status="ok"
      else
        systemd_datapath_rollback_rc=$?
        systemd_datapath_rollback_status="failed"
      fi
      systemctl --user stop "$DATAPATH_SERVICE_UNIT" "$NODE_SERVICE_UNIT" >/dev/null 2>&1 || true
      if ! cleanup_stale_tun_without_state; then
        systemd_datapath_rollback_status="failed"
      fi
      clear_runtime_generated_state
      local systemd_datapath_fail_reason="datapath_apply_failed"
      [[ "$systemd_datapath_apply_status" == "unverified" ]] && systemd_datapath_fail_reason="datapath_proof_failed"
      echo "start_status=fail mode=systemd_user node_runtime=stopped node=$node_status transparent_runtime=stopped datapath_apply=$systemd_datapath_apply_status apply_rc=$systemd_datapath_apply_rc datapath_proof=$systemd_datapath_proof_status datapath_rollback=$systemd_datapath_rollback_status rollback_rc=$systemd_datapath_rollback_rc reason=$systemd_datapath_fail_reason"
      return 1
    fi
    refresh_runtime_publication_after_node_start || systemd_publication_ready=0
    local publication_auto_reconcile="disabled"
    if site_auto_watch_enabled_requested; then
      if site_auto_watch_start >/dev/null 2>&1; then
        publication_auto_reconcile="armed"
      else
        publication_auto_reconcile="failed"
      fi
    fi
    if [[ "$systemd_publication_ready" -eq 0 ]]; then
      local publication_fail_closed="false"
      local publication_exit_rc=0
      local publication_node_runtime="$node_runtime"
      local publication_node_status="$node_status"
      local publication_transparent_runtime="$transparent_runtime"
      local publication_rollback_status="skipped"
      local publication_rollback_rc="0"
      if [[ "$publication_auto_reconcile" == "armed" ]]; then
        echo "start_status=partial mode=systemd_user node_runtime=$publication_node_runtime node=$publication_node_status transparent_runtime=$publication_transparent_runtime datapath_apply=$systemd_datapath_apply_status datapath_proof=$systemd_datapath_proof_status datapath_rollback=$publication_rollback_status rollback_rc=$publication_rollback_rc recovery_state=$PRESTART_SAVED_STATE_RECOVERY peer_update_publish=$START_RUNTIME_PEER_UPDATE_STATUS transit_lane_bindings_publish=$START_RUNTIME_TRANSIT_LANE_BINDINGS_STATUS discovery_publish=$START_RUNTIME_DISCOVERY_PUBLISH_STATUS fail_closed=false auto_reconcile=armed reason=runtime_publication_unready"
        return 0
      fi
      if partial_start_fail_closed; then
        site_auto_watch_stop >/dev/null 2>&1 || true
        if run_chimera_cli rollback recover \
          --state-file "$STATE_FILE" >/dev/null 2>&1; then
          publication_rollback_status="ok"
        else
          publication_rollback_rc=$?
          publication_rollback_status="failed"
        fi
        if ! cleanup_stale_tun_without_state; then
          publication_rollback_status="failed"
        fi
        stop_partial_runtime_components "systemd_user"
        publication_fail_closed="true"
        publication_exit_rc=2
        publication_node_runtime="stopped"
        publication_node_status="stopped"
        publication_transparent_runtime="stopped"
      fi
      echo "start_status=partial mode=systemd_user node_runtime=$publication_node_runtime node=$publication_node_status transparent_runtime=$publication_transparent_runtime datapath_apply=$systemd_datapath_apply_status datapath_proof=$systemd_datapath_proof_status datapath_rollback=$publication_rollback_status rollback_rc=$publication_rollback_rc recovery_state=$PRESTART_SAVED_STATE_RECOVERY peer_update_publish=$START_RUNTIME_PEER_UPDATE_STATUS transit_lane_bindings_publish=$START_RUNTIME_TRANSIT_LANE_BINDINGS_STATUS discovery_publish=$START_RUNTIME_DISCOVERY_PUBLISH_STATUS fail_closed=$publication_fail_closed auto_reconcile=$publication_auto_reconcile reason=runtime_publication_unready"
      return "$publication_exit_rc"
    fi
    echo "start_status=ok mode=systemd_user node_runtime=$node_runtime node=$node_status transparent_runtime=$transparent_runtime datapath_apply=$systemd_datapath_apply_status datapath_proof=$systemd_datapath_proof_status recovery_state=$PRESTART_SAVED_STATE_RECOVERY peer_update_publish=$START_RUNTIME_PEER_UPDATE_STATUS transit_lane_bindings_publish=$START_RUNTIME_TRANSIT_LANE_BINDINGS_STATUS discovery_publish=$START_RUNTIME_DISCOVERY_PUBLISH_STATUS"
    return 0
  fi
  if direct_runtime_mode_blocked_by_orphaned_systemd_units; then
    site_auto_watch_stop >/dev/null 2>&1 || true
    echo "start_status=fail mode=preflight node_runtime=stopped transparent_runtime=stopped reason=user_systemd_session_unavailable units_on_disk=true"
    return 2
  fi
  local direct_node_status="skipped"
  local direct_datapath_status="skipped"
  local direct_datapath_apply_status="skipped"
  local direct_datapath_apply_rc="0"
  local direct_datapath_proof_status="skipped"
  local direct_datapath_rollback_status="skipped"
  local direct_datapath_rollback_rc="0"
  local node_runtime="stopped"
  local direct_publication_ready=1
  if [[ -f "$PEER_EGRESS_ENV_FILE" ]]; then
    start_runner_background "peer_egress" "$(peer_egress_pid_path)" "$NODE_LOG" "$PEER_EGRESS_ENV_FILE" "peer-egress" >/dev/null 2>&1 || true
    if runner_started "$(peer_egress_pid_path)" 10; then
      direct_node_status="started"
      node_runtime="running"
    else
      stop_runner_background "peer_egress" "$(peer_egress_pid_path)" >/dev/null 2>&1 || true
      if repair_node_listener_bindings_for_retry; then
        start_runner_background "peer_egress" "$(peer_egress_pid_path)" "$NODE_LOG" "$PEER_EGRESS_ENV_FILE" "peer-egress" >/dev/null 2>&1 || true
        if runner_started "$(peer_egress_pid_path)" 10; then
          direct_node_status="started"
          node_runtime="running"
        else
          direct_node_status="failed"
          node_runtime="stopped"
        fi
      else
        direct_node_status="failed"
        node_runtime="stopped"
      fi
    fi
  fi
  if [[ "$direct_node_status" == "started" && "$node_config_is_ready" -eq 0 ]]; then
    local direct_listener_fail_closed="false"
    local direct_listener_exit_rc=0
    local direct_listener_node_runtime="$node_runtime"
    local direct_listener_node_status="$direct_node_status"
    local direct_listener_transparent_runtime="skipped"
    site_auto_watch_stop >/dev/null 2>&1 || true
    if partial_start_fail_closed; then
      stop_partial_runtime_components "direct"
      direct_listener_fail_closed="true"
      direct_listener_exit_rc=2
      direct_listener_node_runtime="stopped"
      direct_listener_node_status="stopped"
      direct_listener_transparent_runtime="stopped"
    else
      if mesh_discovery_source_present; then
        site_auto_watch_start >/dev/null 2>&1 || true
      fi
      refresh_runtime_publication_after_node_start >/dev/null 2>&1 || true
    fi
    echo "start_status=partial mode=listener_only node_runtime=$direct_listener_node_runtime node=$direct_listener_node_status transparent_runtime=$direct_listener_transparent_runtime datapath_apply=skipped recovery_state=$PRESTART_SAVED_STATE_RECOVERY peer_update_publish=$START_RUNTIME_PEER_UPDATE_STATUS transit_lane_bindings_publish=$START_RUNTIME_TRANSIT_LANE_BINDINGS_STATUS discovery_publish=$START_RUNTIME_DISCOVERY_PUBLISH_STATUS mesh_ready=false fail_closed=$direct_listener_fail_closed reason=node_endpoint_unconfigured_listener_only"
    return "$direct_listener_exit_rc"
  fi
  if [[ "$direct_node_status" == "started" ]] && node_config_self_loop_target; then
    local direct_self_loop_fail_closed="false"
    local direct_self_loop_exit_rc=0
    local direct_self_loop_node_runtime="$node_runtime"
    local direct_self_loop_node_status="$direct_node_status"
    local direct_self_loop_transparent_runtime="skipped"
    site_auto_watch_stop >/dev/null 2>&1 || true
    if partial_start_fail_closed; then
      stop_partial_runtime_components "direct"
      direct_self_loop_fail_closed="true"
      direct_self_loop_exit_rc=2
      direct_self_loop_node_runtime="stopped"
      direct_self_loop_node_status="stopped"
      direct_self_loop_transparent_runtime="stopped"
    else
      if mesh_discovery_source_present; then
        site_auto_watch_start >/dev/null 2>&1 || true
      fi
      refresh_runtime_publication_after_node_start >/dev/null 2>&1 || true
    fi
    echo "start_status=partial mode=listener_only node_runtime=$direct_self_loop_node_runtime node=$direct_self_loop_node_status transparent_runtime=$direct_self_loop_transparent_runtime datapath_apply=skipped recovery_state=$PRESTART_SAVED_STATE_RECOVERY peer_update_publish=$START_RUNTIME_PEER_UPDATE_STATUS transit_lane_bindings_publish=$START_RUNTIME_TRANSIT_LANE_BINDINGS_STATUS discovery_publish=$START_RUNTIME_DISCOVERY_PUBLISH_STATUS mesh_ready=false fail_closed=$direct_self_loop_fail_closed reason=self_loop_listener_only"
    return "$direct_self_loop_exit_rc"
  fi
  if [[ "$direct_node_status" == "started" && -f "$TRANSPARENT_RUNTIME_ENV_FILE" ]]; then
    start_runner_background "transparent_runtime" "$(transparent_runtime_pid_path)" "$DATAPATH_LOG" "$TRANSPARENT_RUNTIME_ENV_FILE" "transparent-runtime" >/dev/null 2>&1 || true
    if runner_started "$(transparent_runtime_pid_path)" 10; then
      direct_datapath_status="started"
    else
      direct_datapath_status="failed"
    fi
    if [[ "$direct_datapath_status" == "started" ]] && ! remove_state_file_for_datapath_apply; then
      stop_runner_background "transparent_runtime" "$(transparent_runtime_pid_path)" >/dev/null 2>&1 || true
      stop_runner_background "peer_egress" "$(peer_egress_pid_path)" >/dev/null 2>&1 || true
      echo "start_status=fail mode=direct node_runtime=stopped node=$direct_node_status transparent_runtime=stopped datapath_apply=skipped datapath_proof=stale_state_cleanup_failed reason=stale_state_cleanup_failed"
      return 1
    fi
    if [[ "$direct_datapath_status" == "started" ]]; then
      local -a direct_up_args=(
        --config "$(node_config_path)"
        --state-file "$STATE_FILE"
        --apply-tun ${CHIMERA_APPLY_TUN}
      )
      if [[ "${CHIMERA_APPLY_ROUTE}" == "true" ]]; then
        direct_up_args+=(
          --apply-route true
          --route-policy true
          --route-table 51820
          --route-rule-priority 11000
          --route-fwmark "$route_fwmark"
          --route-cidr "$CHIMERA_ROUTE_CIDR"
        )
      else
        direct_up_args+=(--apply-route false)
      fi
      direct_up_args+=(--apply-dns ${CHIMERA_APPLY_DNS})
      if [[ "${CHIMERA_APPLY_TUN}" == "false" && "${CHIMERA_APPLY_ROUTE}" == "false" && "${CHIMERA_APPLY_DNS}" == "false" ]]; then
        direct_up_args+=(--skip-connect-check true)
      fi
      if run_chimera_cli_up_with_retry "${direct_up_args[@]}" >/dev/null 2>&1; then
        direct_datapath_apply_status="ok"
      else
        direct_datapath_apply_rc=$?
        direct_datapath_apply_status="failed"
      fi
    fi
    if [[ "$direct_datapath_apply_status" == "ok" ]]; then
      direct_datapath_proof_status="$(datapath_apply_proof_state || true)"
      if [[ "$direct_datapath_proof_status" != "ok" ]]; then
        if [[ "$direct_datapath_proof_status" == "network_not_modified" \
          && "${CHIMERA_APPLY_TUN}" == "false" \
          && "${CHIMERA_APPLY_ROUTE}" == "false" \
          && "${CHIMERA_APPLY_DNS}" == "false" ]]; then
          direct_datapath_proof_status="ok"
        else
          direct_datapath_apply_status="unverified"
        fi
      fi
    fi
  else
    site_auto_watch_stop >/dev/null 2>&1 || true
  fi
  if [[ "$direct_node_status" != "started" ]]; then
    site_auto_watch_stop >/dev/null 2>&1 || true
    stop_partial_runtime_components "direct"
    echo "start_status=fail mode=direct node_runtime=$node_runtime node=$direct_node_status transparent_runtime=$direct_datapath_status reason=node_service_failed"
    return 1
  fi
  if [[ "$direct_datapath_status" != "started" ]]; then
    site_auto_watch_stop >/dev/null 2>&1 || true
    stop_partial_runtime_components "direct"
    echo "start_status=fail mode=direct node_runtime=$node_runtime node=$direct_node_status transparent_runtime=$direct_datapath_status reason=transparent_service_failed"
    return 1
  fi
    if [[ "$direct_datapath_apply_status" != "ok" ]]; then
      site_auto_watch_stop >/dev/null 2>&1 || true
      if run_chimera_cli rollback recover \
        --state-file "$STATE_FILE" >/dev/null 2>&1; then
        direct_datapath_rollback_status="ok"
      else
        direct_datapath_rollback_rc=$?
        direct_datapath_rollback_status="failed"
      fi
      stop_runner_background "transparent_runtime" "$(transparent_runtime_pid_path)" >/dev/null 2>&1 || true
      stop_runner_background "peer_egress" "$(peer_egress_pid_path)" >/dev/null 2>&1 || true
      if ! cleanup_stale_tun_without_state; then
        direct_datapath_rollback_status="failed"
      fi
      clear_runtime_generated_state
      local direct_datapath_fail_reason="datapath_apply_failed"
      [[ "$direct_datapath_apply_status" == "unverified" ]] && direct_datapath_fail_reason="datapath_proof_failed"
      echo "start_status=fail mode=direct node_runtime=stopped node=$direct_node_status transparent_runtime=stopped datapath_apply=$direct_datapath_apply_status apply_rc=$direct_datapath_apply_rc datapath_proof=$direct_datapath_proof_status datapath_rollback=$direct_datapath_rollback_status rollback_rc=$direct_datapath_rollback_rc reason=$direct_datapath_fail_reason"
      return 1
  fi
  refresh_runtime_publication_after_node_start || direct_publication_ready=0
  local direct_publication_auto_reconcile="disabled"
  if site_auto_watch_enabled_requested; then
    if site_auto_watch_start >/dev/null 2>&1; then
      direct_publication_auto_reconcile="armed"
    else
      direct_publication_auto_reconcile="failed"
    fi
  fi
  if [[ "$direct_publication_ready" -eq 0 ]]; then
    local direct_publication_fail_closed="false"
    local direct_publication_exit_rc=0
    local direct_publication_node_runtime="$node_runtime"
    local direct_publication_node_status="$direct_node_status"
    local direct_publication_transparent_runtime="$direct_datapath_status"
    local direct_publication_rollback_status="skipped"
    local direct_publication_rollback_rc="0"
    if [[ "$direct_publication_auto_reconcile" == "armed" ]]; then
      echo "start_status=partial mode=direct node_runtime=$direct_publication_node_runtime node=$direct_publication_node_status transparent_runtime=$direct_publication_transparent_runtime datapath_apply=$direct_datapath_apply_status datapath_proof=$direct_datapath_proof_status datapath_rollback=$direct_publication_rollback_status rollback_rc=$direct_publication_rollback_rc recovery_state=$PRESTART_SAVED_STATE_RECOVERY peer_update_publish=$START_RUNTIME_PEER_UPDATE_STATUS transit_lane_bindings_publish=$START_RUNTIME_TRANSIT_LANE_BINDINGS_STATUS discovery_publish=$START_RUNTIME_DISCOVERY_PUBLISH_STATUS fail_closed=false auto_reconcile=armed reason=runtime_publication_unready"
      return 0
    fi
    if partial_start_fail_closed; then
      site_auto_watch_stop >/dev/null 2>&1 || true
      if run_chimera_cli rollback recover \
        --state-file "$STATE_FILE" >/dev/null 2>&1; then
        direct_publication_rollback_status="ok"
      else
        direct_publication_rollback_rc=$?
        direct_publication_rollback_status="failed"
      fi
      if ! cleanup_stale_tun_without_state; then
        direct_publication_rollback_status="failed"
      fi
      stop_partial_runtime_components "direct"
      direct_publication_fail_closed="true"
      direct_publication_exit_rc=2
      direct_publication_node_runtime="stopped"
      direct_publication_node_status="stopped"
      direct_publication_transparent_runtime="stopped"
    fi
    echo "start_status=partial mode=direct node_runtime=$direct_publication_node_runtime node=$direct_publication_node_status transparent_runtime=$direct_publication_transparent_runtime datapath_apply=$direct_datapath_apply_status datapath_proof=$direct_datapath_proof_status datapath_rollback=$direct_publication_rollback_status rollback_rc=$direct_publication_rollback_rc recovery_state=$PRESTART_SAVED_STATE_RECOVERY peer_update_publish=$START_RUNTIME_PEER_UPDATE_STATUS transit_lane_bindings_publish=$START_RUNTIME_TRANSIT_LANE_BINDINGS_STATUS discovery_publish=$START_RUNTIME_DISCOVERY_PUBLISH_STATUS fail_closed=$direct_publication_fail_closed auto_reconcile=$direct_publication_auto_reconcile reason=runtime_publication_unready"
    return "$direct_publication_exit_rc"
  fi
  echo "start_status=ok mode=direct node_runtime=$node_runtime node=$direct_node_status transparent_runtime=$direct_datapath_status datapath_apply=$direct_datapath_apply_status datapath_proof=$direct_datapath_proof_status recovery_state=$PRESTART_SAVED_STATE_RECOVERY peer_update_publish=$START_RUNTIME_PEER_UPDATE_STATUS transit_lane_bindings_publish=$START_RUNTIME_TRANSIT_LANE_BINDINGS_STATUS discovery_publish=$START_RUNTIME_DISCOVERY_PUBLISH_STATUS"
}

stop_runtime() {
  site_auto_watch_stop >/dev/null 2>&1 || true
  local cleanup_rc=0
  local down_rc=0
  local route_fwmark
  route_fwmark="$(route_fwmark_env_value)"
  if systemd_user_ready; then
    systemctl --user stop "$DATAPATH_SERVICE_UNIT" "$LEGACY_DATAPATH_COMPAT_SERVICE_UNIT" >/dev/null 2>&1 || true
    systemctl --user stop "$NODE_SERVICE_UNIT" "$LEGACY_NODE_COMPAT_SERVICE_UNIT" >/dev/null 2>&1 || true
    stop_runner_background "peer_update" "$(peer_update_pid_path)" >/dev/null 2>&1 || true
    run_chimera_cli down \
      --config "$(node_config_path)" \
      --state-file "$STATE_FILE" \
      --apply-tun true \
      --apply-route true \
      --route-policy true \
      --route-table 51820 \
      --route-rule-priority 11000 \
      --route-fwmark "$route_fwmark" \
      --route-cidr "$CHIMERA_ROUTE_CIDR" \
      --apply-dns ${CHIMERA_APPLY_DNS} >/dev/null 2>&1 || down_rc=$?
    cleanup_transparent_redirect_rules || cleanup_rc=$?
    if [[ "$down_rc" -ne 0 ]]; then
      echo "stop_status=fail mode=systemd_user reason=datapath_down_failed down_rc=$down_rc"
      return 1
    fi
    if [[ "$cleanup_rc" -ne 0 ]]; then
      echo "stop_status=fail mode=systemd_user reason=transparent_redirect_cleanup_failed"
      return 1
    fi
    clear_runtime_generated_state
    echo "stop_status=ok mode=systemd_user"
    return 0
  fi
  if direct_runtime_mode_blocked_by_orphaned_systemd_units; then
    echo "stop_status=fail mode=preflight reason=user_systemd_session_unavailable units_on_disk=true"
    return 2
  fi
  stop_runner_background "transparent_runtime" "$(transparent_runtime_pid_path)" >/dev/null 2>&1 || true
  stop_runner_background "peer_egress" "$(peer_egress_pid_path)" >/dev/null 2>&1 || true
  stop_runner_background "peer_update" "$(peer_update_pid_path)" >/dev/null 2>&1 || true
  run_chimera_cli down \
    --config "$(node_config_path)" \
    --state-file "$STATE_FILE" \
    --apply-tun true \
    --apply-route true \
    --route-policy true \
    --route-table 51820 \
    --route-rule-priority 11000 \
    --route-fwmark "$route_fwmark" \
    --route-cidr "$CHIMERA_ROUTE_CIDR" \
    --apply-dns ${CHIMERA_APPLY_DNS} >/dev/null 2>&1 || down_rc=$?
  cleanup_transparent_redirect_rules || cleanup_rc=$?
  if [[ "$down_rc" -ne 0 ]]; then
    echo "stop_status=fail mode=direct reason=datapath_down_failed down_rc=$down_rc"
    return 1
  fi
  if [[ "$cleanup_rc" -ne 0 ]]; then
    echo "stop_status=fail mode=direct reason=transparent_redirect_cleanup_failed"
    return 1
  fi
  clear_runtime_generated_state
  echo "stop_status=ok mode=direct"
}

restart_runtime() {
  stop_runtime >/dev/null 2>&1 || {
    echo "restart_status=fail reason=stop_failed"
    return 1
  }
  start_runtime
}

runtime_status() {
  local runtime_state runtime_enabled_state node_state datapath_state route_mode split_mode watch_status
  runtime_state="$(read_runtime_service_state "$RUNTIME_SERVICE_UNIT")"
  runtime_enabled_state="$(read_runtime_service_enable_state "$RUNTIME_SERVICE_UNIT")"
  node_state="$(read_runtime_service_state "$NODE_SERVICE_UNIT")"
  datapath_state="$(read_runtime_service_state "$DATAPATH_SERVICE_UNIT")"
  route_mode="$(read_route_mode)"
  split_mode="$(read_split_list_mode)"
  watch_status="$(site_auto_watch_status)"
  echo "runtime_root=<redacted>"
  echo "runtime_root_state=present"
  echo "runtime_boot_service_state=$runtime_state"
  echo "runtime_boot_enabled_state=$runtime_enabled_state"
  echo "node_service_state=$node_state"
  echo "transparent_runtime_service_state=$datapath_state"
  echo "peer_egress_state_file=<redacted>"
  echo "$watch_status"
  if [[ "$node_state" == "active" ]]; then
    echo "node_runtime=running"
  else
    echo "node_runtime=stopped"
  fi
  if runtime_state_is_up; then
    if node_config_ready; then
      echo "transparent_runtime=$([[ "$datapath_state" == "active" ]] && echo running || echo stopped)"
    else
      echo "transparent_runtime=stopped"
    fi
    echo "runtime_state_status=up"
  else
    if systemd_user_ready; then
      if node_config_ready; then
        echo "transparent_runtime=$([[ "$datapath_state" == "active" ]] && echo running || echo stopped)"
      else
        echo "transparent_runtime=stopped"
      fi
    else
      if node_config_ready; then
        if pidfile_running "$(peer_egress_pid_path)" && pidfile_running "$(transparent_runtime_pid_path)"; then
          echo "transparent_runtime=running"
        else
          echo "transparent_runtime=stopped"
        fi
      else
        echo "transparent_runtime=stopped"
      fi
    fi
  echo "runtime_state_status=unknown"
  fi
  echo "route_mode=$route_mode"
  echo "split_list_mode=$split_mode"
  echo "bound_transit_authority_state=$(bound_transit_authority_state)"
  echo "runtime_publication_state=$(runtime_publication_state)"
  if node_config_ready; then
    echo "node_config_ready=true"
  else
    echo "node_config_ready=false"
  fi
  if [[ -f "$STATE_FILE" ]]; then
    echo "state_file=<redacted>"
    echo "state_file_state=present"
    awk -F= '
      /^carrier\.addr[[:space:]]*=/ { print "carrier_addr=<redacted>"; print "carrier_addr_state=present" }
      /^selected_node[[:space:]]*=/ { print "selected_node=<redacted>"; print "selected_node_state=present" }
      /^mesh_node[[:space:]]*=/ { print "mesh_node=<redacted>"; print "mesh_node_state=present" }
      /^autoconnect[[:space:]]*=/ { print "autoconnect=" $2 }
    ' "$STATE_FILE" 2>/dev/null || true
  else
    echo "state_file_state=missing"
  fi
  if [[ -f "$PEER_UPDATE_STATE_FILE" ]]; then
    echo "peer_update_state_file=<redacted>"
    echo "peer_update_state_file_state=present"
  else
    echo "peer_update_state_file_state=missing"
  fi
  if [[ -f "$(mesh_discovery_out_path)" ]]; then
    echo "discovery_snapshot_state=present"
  else
    echo "discovery_snapshot_state=missing"
  fi
  if [[ -f "$(peer_egress_state_path)" ]]; then
    echo "peer_egress_state=<redacted>"
    echo "peer_egress_state_file_state=present"
    awk -F= '
      /^resolved_local_listen[[:space:]]*=/ { print "peer_egress_resolved_local_listen=<redacted>"; print "peer_egress_resolved_local_listen_state=present" }
      /^resolved_peer_listen[[:space:]]*=/ { print "peer_egress_resolved_peer_listen=<redacted>"; print "peer_egress_resolved_peer_listen_state=present" }
      /^mode[[:space:]]*=/ { print "peer_egress_mode=" $2 }
    ' "$(peer_egress_state_path)" 2>/dev/null || true
  else
    echo "peer_egress_state_file_state=missing"
  fi
}

emit_route_status_lines() {
  local app_routes_count service_routes_count manual_count adaptive_count
  local datapath_proof_state datapath_flow_proof_state datapath_mode datapath_apply
  app_routes_count="$(count_config_prefix "$APP_ROUTES_FILE" "app:")"
  service_routes_count="$(count_config_prefix "$APP_ROUTES_FILE" "service:")"
  manual_count="$(count_noncomment_lines "$MANUAL_TRANSIT_DOMAINS_FILE")"
  adaptive_count="$(count_noncomment_lines "$ADAPTIVE_DOMAINS_FILE")"
  datapath_proof_state="$(datapath_apply_proof_state || true)"
  if [[ "$datapath_proof_state" == "ok" ]]; then
    datapath_apply="ok"
    datapath_flow_proof_state="$(datapath_strict_flow_proof_state || true)"
  else
    datapath_apply="unverified"
    datapath_flow_proof_state="skipped_apply_unverified"
  fi
  if [[ "$datapath_proof_state" == "ok" && "$datapath_flow_proof_state" == "ok" ]]; then
    datapath_mode="transparent"
  else
    datapath_mode="unknown"
  fi
  echo "datapath_mode=$datapath_mode"
  echo "datapath_apply=$datapath_apply"
  echo "datapath_proof=$datapath_proof_state"
  echo "datapath_flow_proof=$datapath_flow_proof_state"
  echo "app_routes_count=$app_routes_count"
  echo "service_routes_count=$service_routes_count"
  echo "manual_transit_domains_count=$manual_count"
  echo "adaptive_domains_count=$adaptive_count"
}

datapath_status() {
  runtime_status
  emit_route_status_lines
}

route_status() {
  emit_route_status_lines
  echo "runtime_state_status=$(runtime_state_is_up && echo up || echo unknown)"
  echo "route_mode=$(read_route_mode)"
  echo "split_list_mode=$(read_split_list_mode)"
  if [[ -f "$SERVICE_ROUTE_OVERRIDES_FILE" ]]; then
    awk -F= '/^service_route_override\[/{print}' "$SERVICE_ROUTE_OVERRIDES_FILE"
  fi
}

split_transparent_status() {
  datapath_status
}

split_transparent_dispatch() {
  local sub="${1:-status}"
  case "$sub" in
    start) start_runtime ;;
    stop) stop_runtime ;;
    status) split_transparent_status ;;
    refresh) restart_runtime ;;
    *)
      echo "error: unknown split-transparent subcommand: $sub" >&2
      return 2
      ;;
  esac
}

ui_mode_dispatch() {
  local mode="${1:-show}"
  mkdir -p "$(dirname "$UI_MODE_FILE")"
  case "$mode" in
    show|"")
      if [[ -f "$UI_MODE_FILE" ]]; then
        tr -d '[:space:]' <"$UI_MODE_FILE"
      else
        echo "auto"
      fi
      ;;
    auto|tray|dialog|cli)
      printf '%s\n' "$mode" >"$UI_MODE_FILE"
      echo "ui_mode=$mode"
      ;;
    *)
      echo "error: ui-mode must be one of auto|tray|dialog|cli|show" >&2
      return 2
      ;;
  esac
}

uninstall_runtime() {
  stop_runtime >/dev/null 2>&1 || {
    echo "uninstall_status=fail reason=stop_failed"
    return 1
  }
  if systemd_user_ready; then
    systemctl --user disable --now "$RUNTIME_SERVICE_UNIT" "$NODE_SERVICE_UNIT" "$DATAPATH_SERVICE_UNIT" "$SITE_AUTOWATCH_SERVICE_UNIT" "$LEGACY_NODE_COMPAT_SERVICE_UNIT" "$LEGACY_DATAPATH_COMPAT_SERVICE_UNIT" >/dev/null 2>&1 || true
  fi
  rm -f "$STATE_FILE" "$NODE_LOG" "$DATAPATH_LOG" "$LAST_ENDPOINT_FILE" "$UPSTREAM_HEALTH_STATE_FILE" "$SITE_AUTOWATCH_PID_FILE" "$(peer_egress_pid_path)" "$(transparent_runtime_pid_path)"
  rm -f "$(peer_egress_state_path)"
  rm -f "${XDG_CONFIG_HOME:-$HOME/.config}/chimera/peer-egress.env" "${XDG_CONFIG_HOME:-$HOME/.config}/chimera/transparent-runtime.env"
  rm -f "$SERVICE_ROUTE_OVERRIDES_FILE"
  rm -f "${XDG_CONFIG_HOME:-$HOME/.config}/chimera/site_adaptive_routes.db"
  uninstall_release_tree || {
    echo "uninstall_status=fail reason=cleanup_failed"
    return 1
  }
  if systemd_user_ready; then
    systemctl --user daemon-reload >/dev/null 2>&1 || true
  fi
  echo "uninstall_status=ok"
}

main() {
  local cmd="${1:-}"
  case "$cmd" in
    start)
      update_first_gate -start
      start_runtime
      ;;
    stop)
      stop_runtime
      ;;
    restart)
      update_first_gate -restart
      restart_runtime
      ;;
    status)
      runtime_status
      ;;
    doctor)
      doctor_run
      ;;
    logs)
      shift || true
      logs_tail "${1:-200}"
      ;;
    datapath-status)
      datapath_status
      ;;
    app-routes-status)
      app_routes_status
      ;;
    route-status)
      route_status
      ;;
    run-app)
      shift || true
      [[ $# -ge 1 ]] || { echo "error: run-app requires app_id" >&2; exit 2; }
      run_app "$@"
      ;;
    verify-app)
      shift || true
      [[ $# -ge 1 ]] || { echo "error: verify-app requires app_id" >&2; exit 2; }
      verify_app "$@"
      ;;
    verify-cmd)
      shift || true
      [[ $# -ge 1 ]] || { echo "error: verify-cmd requires a command" >&2; exit 2; }
      verify_cmd "$@"
      ;;
    service-route-enable)
      shift || true
      service_route_enable "$@"
      ;;
    service-route-disable)
      shift || true
      service_route_disable "$@"
      ;;
    verify-service)
      shift || true
      verify_service "$@"
      ;;
    route-mode)
      case "${2:-show}" in
        show|"")
          echo "route_mode=$(read_route_mode)"
          ;;
        full|split|off)
          write_route_mode "${2}"
          echo "route_mode=$(read_route_mode)"
          ;;
        *)
          echo "error: route-mode must be show|full|split|off" >&2
          exit 2
          ;;
      esac
      ;;
    split-list-mode)
      case "${2:-show}" in
        show|"")
          echo "split_list_mode=$(read_split_list_mode)"
          ;;
        allow|deny)
          write_split_list_mode "${2}"
          echo "split_list_mode=$(read_split_list_mode)"
          ;;
        *)
          echo "error: split-list-mode must be show|allow|deny" >&2
          exit 2
          ;;
      esac
      ;;
    site-add)
      shift || true
      [[ $# -ge 1 ]] || { echo "error: site-add requires domain(s)" >&2; exit 2; }
      site_add "$@"
      ;;
    site-remove)
      shift || true
      [[ $# -ge 1 ]] || { echo "error: site-remove requires domain(s)" >&2; exit 2; }
      site_remove "$@"
      ;;
    site-list)
      site_list
      ;;
    site-auto-resolve)
      shift || true
      site_auto_resolve_run "$@"
      ;;
    site-auto-status)
      site_auto_status
      ;;
    site-auto-bootstrap)
      site_auto_bootstrap_run
      ;;
    site-auto-discover)
      case "${2:-run}" in
        run)
          site_auto_discover_run
          ;;
        status)
          echo "site_auto_discover_status=ok"
          echo "site_discovery_file=$SITE_DISCOVERY_DOMAINS_FILE"
          echo "site_discovery_count=$(count_noncomment_lines "$SITE_DISCOVERY_DOMAINS_FILE")"
          ;;
        clear)
          rm -f "$SITE_DISCOVERY_DOMAINS_FILE"
          echo "site_auto_discover_status=cleared"
          ;;
        *)
          echo "error: site-auto-discover must be run|status|clear" >&2
          exit 2
          ;;
      esac
      ;;
    site-auto-watch)
      case "${2:-status}" in
        start)
          site_auto_watch_start
          ;;
        stop)
          site_auto_watch_stop
          ;;
        status)
          site_auto_watch_status
          ;;
        run-once)
          site_auto_watch_run_once
          ;;
        *)
          echo "error: site-auto-watch must be start|stop|status|run-once" >&2
          exit 2
          ;;
      esac
      ;;
    __site-auto-watch-loop)
      site_auto_watch_loop
      ;;
    split-transparent)
      case "${2:-status}" in
        start)
          update_first_gate -start
          ;;
        refresh)
          update_first_gate -restart
          ;;
      esac
      split_transparent_dispatch "${2:-status}"
      ;;
    grant-perms)
      grant_runtime_permissions
      ;;
    preflight-perms)
      if [[ "${2:-}" == "--warn-only" ]]; then
        run_permissions_preflight 1
      else
        run_permissions_preflight 0
      fi
      ;;
    upstream-probe)
      upstream_probe
      ;;
    upstream-reset)
      upstream_reset
      ;;
    upstream-audit)
      upstream_audit "${2:-30}"
      ;;
    upstream-failover-smoke)
      upstream_failover_smoke "${2:-10}"
      ;;
    mesh-seed-control-plane)
      shift || true
      seed_mesh_control_plane_authority_from_bootstrap "${1:---strict}"
      ;;
    mesh-bind-control-plane)
      shift || true
      mesh_bind_control_plane "${1:---strict}"
      ;;
    apps-running)
      apps_running
      ;;
    services-running)
      services_running
      ;;
    service-route-enable-running)
      shift || true
      service_route_enable_running "$@"
      ;;
    ui-mode)
      shift || true
      ui_mode_dispatch "${1:-show}"
      ;;
    uninstall)
      uninstall_runtime
      ;;
    mesh)
      update_first_gate -mesh "${@:2}"
      shift || true
      run_chimera_cli mesh "$@"
      ;;
    __service-preflight-node)
      node_service_prestart_self_heal
      ;;
    __service-preflight-datapath)
      datapath_service_prestart_validate
      ;;
    __service-poststart-node)
      node_service_poststart_reconcile
      ;;
    -h|--help|help|"")
      usage
      ;;
    *)
      echo "error: unknown command: $cmd" >&2
      usage
      exit 2
      ;;
  esac
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  main "$@"
fi
