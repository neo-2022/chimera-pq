#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
STATE_FILE="${STATE_FILE:-$ROOT_DIR/docs/runtime_state_latest.json}"
GATEWAY_LOG="${GATEWAY_LOG:-$ROOT_DIR/docs/chimera_gateway.service.log}"
CLIENT_LOG="${CLIENT_LOG:-$ROOT_DIR/docs/chimera_client.service.log}"
UI_MODE_FILE="${UI_MODE_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/ui_mode}"
UPSTREAM_ENV_FILE="${UPSTREAM_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/upstream_proxy.env}"
CHIMERA_PROTECTED_PORTS_CSV="${CHIMERA_PROTECTED_PORTS_CSV:-11080,22180}"
CHIMERA_SAFE_HOST_LOCK="${CHIMERA_SAFE_HOST_LOCK:-1}"
CHIMERA_ALLOW_LOCAL_NETWORK_MUTATION="${CHIMERA_ALLOW_LOCAL_NETWORK_MUTATION:-0}"
POLICY_FILE="${POLICY_FILE:-$ROOT_DIR/configs/policy.runtime.conf}"
MANUAL_GATEWAY_DOMAINS_FILE="${MANUAL_GATEWAY_DOMAINS_FILE:-$ROOT_DIR/configs/manual_gateway_domains.txt}"
ADAPTIVE_DOMAINS_FILE="${ADAPTIVE_DOMAINS_FILE:-$ROOT_DIR/configs/adaptive_domains.txt}"
APP_ROUTES_FILE="${APP_ROUTES_FILE:-$ROOT_DIR/configs/chimera-app-routes.conf}"
SERVICE_ROUTE_OVERRIDES_FILE="${SERVICE_ROUTE_OVERRIDES_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/service_route_overrides.conf}"
PAC_FILE="${PAC_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/chimera-proxy.pac}"
ROUTE_MODE_FILE="${ROUTE_MODE_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/route_mode}"
SPLIT_LIST_MODE_FILE="${SPLIT_LIST_MODE_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/split_list_mode}"
AUTOFIX_SCRIPT="$ROOT_DIR/scripts/chimera-autofix.sh"
UPSTREAM_AUTOBOOTSTRAP_SCRIPT="${UPSTREAM_AUTOBOOTSTRAP_SCRIPT:-$ROOT_DIR/scripts/chimera_upstream_autobootstrap.sh}"
AUTOFIX_TIMEOUT="${CHIMERA_AUTOFIX_MAX_TIME:-25}"
UPSTREAM_SSH_KEY_FILE="${CHIMERA_UPSTREAM_SSH_KEY_FILE:-$HOME/.ssh/id_ed25519}"
CHIMERA_ALLOW_PGREP_KILL="${CHIMERA_ALLOW_PGREP_KILL:-0}"
AUTO_RESTART_CHROMIUM="${CHIMERA_AUTO_RESTART_CHROMIUM:-0}"
PROXY_AUTOSYNC="${CHIMERA_PROXY_AUTOSYNC:-1}"
CHIMERA_SYSTEM_INTEGRATION="${CHIMERA_SYSTEM_INTEGRATION:-0}"
UPSTREAM_STRATEGY="${CHIMERA_UPSTREAM_STRATEGY:-balanced}"
UPSTREAM_STICKY_SEC="${CHIMERA_UPSTREAM_STICKY_SEC:-120}"
LAST_ENDPOINT_FILE="${XDG_CACHE_HOME:-$HOME/.cache}/chimera/last_upstream_endpoint"
UPSTREAM_HEALTH_STATE_FILE="${XDG_CACHE_HOME:-$HOME/.cache}/chimera/upstream_health_state"
SITE_ADAPTIVE_DB_FILE="${SITE_ADAPTIVE_DB_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/site_adaptive_routes.db}"
SITE_AUTO_SEEDS_FILE="${SITE_AUTO_SEEDS_FILE:-$ROOT_DIR/configs/auto_failover_seeds.txt}"
SITE_AUTOWATCH_PID_FILE="${SITE_AUTOWATCH_PID_FILE:-${XDG_RUNTIME_DIR:-/tmp}/chimera-site-autowatch.pid}"
SITE_AUTOWATCH_INTERVAL_SEC="${SITE_AUTOWATCH_INTERVAL_SEC:-60}"
SITE_AUTOWATCH_ENABLED="${SITE_AUTOWATCH_ENABLED:-1}"
SITE_AUTO_DISCOVERY_ENABLED="${SITE_AUTO_DISCOVERY_ENABLED:-1}"
SITE_AUTO_DISCOVERY_LOOKBACK_SEC="${SITE_AUTO_DISCOVERY_LOOKBACK_SEC:-120}"
SITE_DISCOVERY_DOMAINS_FILE="${SITE_DISCOVERY_DOMAINS_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/discovered_domains.txt}"
SITE_FAILOVER_PROXY_THRESHOLD="${SITE_FAILOVER_PROXY_THRESHOLD:-1}"
SITE_FAILBACK_DIRECT_THRESHOLD="${SITE_FAILBACK_DIRECT_THRESHOLD:-3}"
SITE_ADAPTIVE_ENTRY_TTL_SEC="${SITE_ADAPTIVE_ENTRY_TTL_SEC:-86400}"
AUTOFIX_LOG_FILE="${AUTOFIX_LOG_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/autofix.log}"
CHIMERA_CLI_BIN="${CHIMERA_CLI_BIN:-$ROOT_DIR/bin/chimera-cli}"
CHIMERA_GATEWAY_BIN="${CHIMERA_GATEWAY_BIN:-$ROOT_DIR/bin/chimera-gateway}"
CHIMERA_RUNNER="${CHIMERA_RUNNER:-$ROOT_DIR/scripts/chimera-runner.sh}"
RUNTIME_BOOTSTRAP_SCRIPT="${RUNTIME_BOOTSTRAP_SCRIPT:-$ROOT_DIR/scripts/chimera_runtime_bootstrap.sh}"
SINGBOX_BIN="${SINGBOX_BIN:-${XDG_DATA_HOME:-$HOME/.local/share}/chimera-pq/runtime/singbox/sing-box}"
CLIENT_CONFIG_FILE="${CLIENT_CONFIG_FILE:-$ROOT_DIR/configs/client.conf}"
PEER_EGRESS_ENV_FILE="${PEER_EGRESS_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/peer-egress.env}"
PEER_EGRESS_STATE_FILE="${PEER_EGRESS_STATE_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/peer-egress.state}"
MESH_DISCOVERY_OUT_FILE="${MESH_DISCOVERY_OUT_FILE:-$ROOT_DIR/mesh_nodes.discovery.json}"
MESH_DISCOVERY_PUBKEY_OUT_FILE="${MESH_DISCOVERY_PUBKEY_OUT_FILE:-$ROOT_DIR/mesh_nodes.discovery.pubkey}"
TRANSPARENT_RUNTIME_ENV_FILE="${TRANSPARENT_RUNTIME_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/transparent-runtime.env}"
SPLIT_TRANSPARENT_ENABLED="${SPLIT_TRANSPARENT_ENABLED:-1}"
SPLIT_TRANSPARENT_TUN_NAME="${SPLIT_TRANSPARENT_TUN_NAME:-chimera-tun}"
SPLIT_TRANSPARENT_TUN_ADDR="${SPLIT_TRANSPARENT_TUN_ADDR:-172.19.0.1/30}"
SPLIT_TRANSPARENT_TUN_ADDR6="${SPLIT_TRANSPARENT_TUN_ADDR6:-fd5a:7c0a:1::1/126}"
SPLIT_TRANSPARENT_AUTO_REDIRECT="${SPLIT_TRANSPARENT_AUTO_REDIRECT:-1}"
SPLIT_TRANSPARENT_CONFIG_FILE="${SPLIT_TRANSPARENT_CONFIG_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/singbox-split.json}"
SPLIT_TRANSPARENT_PID_FILE="${SPLIT_TRANSPARENT_PID_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/chimera-singbox.pid}"
SPLIT_TRANSPARENT_LOG_FILE="${SPLIT_TRANSPARENT_LOG_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/singbox-split.log}"
SPLIT_TRANSPARENT_LOG_LEVEL="${SPLIT_TRANSPARENT_LOG_LEVEL:-warn}"
SPLIT_TRANSPARENT_DNS_STRATEGY="${SPLIT_TRANSPARENT_DNS_STRATEGY:-prefer_ipv4}"
SPLIT_TRANSPARENT_WATCHDOG_PID_FILE="${SPLIT_TRANSPARENT_WATCHDOG_PID_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/chimera-split-watchdog.pid}"
CHIMERA_ALLOW_WEAVE_COEXIST_MUTATION="${CHIMERA_ALLOW_WEAVE_COEXIST_MUTATION:-0}"
CHIMERA_COEXIST_TRANSPARENT_CAPTURE="${CHIMERA_COEXIST_TRANSPARENT_CAPTURE:-1}"
CHIMERA_FORCE_PROXY_NONE="${CHIMERA_FORCE_PROXY_NONE:-0}"
CHIMERA_REQUIRE_UPSTREAM_FOR_FAILOVER="${CHIMERA_REQUIRE_UPSTREAM_FOR_FAILOVER:-1}"
CHIMERA_STRICT_FAILOVER_GATE="${CHIMERA_STRICT_FAILOVER_GATE:-1}"
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
  mkdir -p "$(dirname "$file")"
  touch "$file"
  if grep -qE "^${key}=" "$file"; then
    sed -i "s|^${key}=.*|${key}=${value}|" "$file"
  else
    printf '%s=%s\n' "$key" "$value" >> "$file"
  fi
}

pid_cmdline_contains() {
  local pid="${1:-}"
  local needle="${2:-}"
  [[ -n "$pid" && -n "$needle" ]] || return 1
  [[ -r "/proc/$pid/cmdline" ]] || return 1
  tr '\0' ' ' <"/proc/$pid/cmdline" | grep -Fq -- "$needle"
}

pidfile_running() {
  local pidfile="${1:?pidfile_required}"
  local pid=""
  [[ -f "$pidfile" ]] || return 1
  pid="$(tr -d '[:space:]' <"$pidfile" 2>/dev/null || true)"
  [[ -n "$pid" ]] || return 1
  kill -0 "$pid" >/dev/null 2>&1
}

peer_egress_pid_path() {
  printf '%s' "${PEER_EGRESS_PID_FILE:-${XDG_RUNTIME_DIR:-/tmp}/chimera-peer-egress.pid}"
}

transparent_runtime_pid_path() {
  printf '%s' "${TRANSPARENT_RUNTIME_PID_FILE:-${XDG_RUNTIME_DIR:-/tmp}/chimera-transparent-runtime.pid}"
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

publish_mesh_discovery_snapshot() {
  local state_path
  state_path="$(peer_egress_state_path)"
  [[ -f "$state_path" ]] || return 0
  local discovery_out
  discovery_out="$(mesh_discovery_out_path)"
  local pubkey_out
  pubkey_out="$(mesh_discovery_pubkey_out_path)"
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
    self_node_id="${CHIMERA_MESH_SELF_NODE_ID:-${CHIMERA_UPSTREAM_NODE_ID:-}}"
  fi
  self_node_id="${self_node_id:-$(hostname -s 2>/dev/null || hostname 2>/dev/null || echo chimera-node)}"
  if wait_for_file "$state_path" 5; then
    CHIMERA_MESH_PEER_EGRESS_STATE_PATH="$state_path" \
    CHIMERA_MESH_SELF_NODE_ID="$self_node_id" \
    "$CHIMERA_RUNNER" cli mesh nodes advertise \
      --state-file "$state_path" \
      --out "$discovery_out" \
      --pubkey-out "$pubkey_out" >/dev/null 2>&1 || return 1
    echo "discovery_snapshot_out=$discovery_out"
    echo "discovery_snapshot_pubkey=$pubkey_out"
  fi
}

ensure_upstream_env_bootstrapped() {
  count_candidates_csv() {
    local raw="${1:-}"
    local count=0 item
    IFS=',' read -r -a arr <<<"$raw"
    for item in "${arr[@]}"; do
      item="$(trim_ascii "$item")"
      [[ -z "$item" ]] && continue
      count=$((count + 1))
    done
    echo "$count"
  }

  local need_bootstrap="1"
  local existing_user="" existing_pass="" existing_candidates_csv=""
  if [[ -f "$UPSTREAM_ENV_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$UPSTREAM_ENV_FILE"
    existing_user="${CHIMERA_UPSTREAM_USER:-}"
    existing_pass="${CHIMERA_UPSTREAM_PASS:-}"
    existing_candidates_csv="${CHIMERA_UPSTREAM_ENDPOINTS_CSV:-}"
    if [[ -n "${CHIMERA_UPSTREAM_USER:-}" && -n "${CHIMERA_UPSTREAM_HOST:-}" && -n "${CHIMERA_UPSTREAM_PASS:-}" ]]; then
      need_bootstrap="0"
    fi
  fi

  local candidate_count="0"
  candidate_count="$(count_candidates_csv "$existing_candidates_csv")"
  # Force re-bootstrap when we have only a single egress candidate.
  if [[ "$need_bootstrap" == "0" && "$candidate_count" =~ ^[0-9]+$ && "$candidate_count" -lt 2 ]]; then
    need_bootstrap="1"
  fi

  if [[ "$need_bootstrap" == "1" && -x "$UPSTREAM_AUTOBOOTSTRAP_SCRIPT" ]]; then
    CHIMERA_UPSTREAM_POOL_USER="${CHIMERA_UPSTREAM_POOL_USER:-$existing_user}" \
    CHIMERA_UPSTREAM_POOL_PASS="${CHIMERA_UPSTREAM_POOL_PASS:-$existing_pass}" \
    UPSTREAM_ENV_FILE="$UPSTREAM_ENV_FILE" \
    "$UPSTREAM_AUTOBOOTSTRAP_SCRIPT" >/dev/null 2>&1 || true
  fi

  if [[ -f "$ROOT_DIR/configs/upstream_proxy.env.example" ]]; then
    local discovery_url discovery_pubkey discovery_probe_timeout
    discovery_url="$(awk -F= '/^CHIMERA_MESH_NODES_DISCOVERY_URL=/{print $2; exit}' "$ROOT_DIR/configs/upstream_proxy.env.example" 2>/dev/null || true)"
    discovery_pubkey="$(awk -F= '/^CHIMERA_MESH_NODES_DISCOVERY_PUBKEY=/{print $2; exit}' "$ROOT_DIR/configs/upstream_proxy.env.example" 2>/dev/null || true)"
    discovery_probe_timeout="$(awk -F= '/^CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS=/{print $2; exit}' "$ROOT_DIR/configs/upstream_proxy.env.example" 2>/dev/null || true)"
    if [[ -n "$discovery_url" ]] && [[ -f "$UPSTREAM_ENV_FILE" ]] && ! grep -q '^CHIMERA_MESH_NODES_DISCOVERY_URL=' "$UPSTREAM_ENV_FILE"; then
      printf '\nCHIMERA_MESH_NODES_DISCOVERY_URL=%s\n' "$discovery_url" >> "$UPSTREAM_ENV_FILE"
    fi
    if [[ -n "$discovery_pubkey" ]] && [[ -f "$UPSTREAM_ENV_FILE" ]] && ! grep -q '^CHIMERA_MESH_NODES_DISCOVERY_PUBKEY=' "$UPSTREAM_ENV_FILE"; then
      printf 'CHIMERA_MESH_NODES_DISCOVERY_PUBKEY=%s\n' "$discovery_pubkey" >> "$UPSTREAM_ENV_FILE"
    fi
    if [[ -n "$discovery_probe_timeout" ]] && [[ -f "$UPSTREAM_ENV_FILE" ]] && ! grep -q '^CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS=' "$UPSTREAM_ENV_FILE"; then
      printf 'CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS=%s\n' "$discovery_probe_timeout" >> "$UPSTREAM_ENV_FILE"
    fi
  fi
}

usage() {
  cat <<'EOF'
Usage: chimera-control.sh <command>

Commands:
  start          Start CHIMERA gateway+client services (systemd --user)
  stop           Stop CHIMERA gateway+client services
  restart        Restart CHIMERA services
  status         Show status for CHIMERA services and runtime state
  doctor         Run client doctor check
  logs           Tail gateway+client logs
  proxy-status   Show transparent runtime and route status
  app-routes-status  Show parsed app/service routing config
  route-status       Show split routing runtime status
  run-app <app_id> [args...]
                Run selected app under the transparent runtime
  verify-app <app_id> [args...]
                Verify app run under the transparent runtime
  verify-cmd <command...>
                Verify any command/binary under the transparent runtime
  service-proxy-enable [service...]
                Retired legacy command; transparent runtime is the default
  service-proxy-disable [service...]
                Retired legacy command; transparent runtime is the default
  verify-service <service...>
                Retired legacy command; transparent runtime is the default
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
  apps-running  Show running applications (process names)
  services-running
                Show running user services
  mesh <args...>
                Pass through to chimera-cli mesh <args...>
  app-route-add <app_id> <command>
                Add/update app route entry in config
  app-route-add-running <process_name...>
                Add running processes as app routes automatically
  service-proxy-enable-running [service...]
                Enable CHIMERA proxy for running user services
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

ensure_base_path() {
  export PATH="$HOME/.local/bin:$PATH"
}

client_config_path() {
  if [[ -f "$CLIENT_CONFIG_FILE" ]]; then
    echo "$CLIENT_CONFIG_FILE"
    return 0
  fi
  echo "$ROOT_DIR/configs/client.example.conf"
}

client_config_ready() {
  local config_path addr
  config_path="$(client_config_path)"
  [[ -f "$config_path" ]] || return 1
  addr="$(awk -F'=' '
    $1 ~ /^[[:space:]]*carrier\.addr[[:space:]]*$/ {
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2);
      print $2;
      exit
    }
  ' "$config_path" 2>/dev/null || true)"
  case "$addr" in
    ""|203.0.113.10:443|127.0.0.1:443) return 1 ;;
    *) return 0 ;;
  esac
}

run_chimera_cli() {
  if [[ -x "$CHIMERA_RUNNER" ]]; then
    "$CHIMERA_RUNNER" cli "$@"
    return $?
  fi
  if [[ -x "$CHIMERA_CLI_BIN" ]]; then
    "$CHIMERA_CLI_BIN" "$@"
    return $?
  fi
  if [[ "${CHIMERA_ALLOW_CARGO_FALLBACK:-0}" == "1" ]] && command -v cargo >/dev/null 2>&1; then
    (
      cd "$ROOT_DIR"
      cargo run -q -p chimera-cli -- "$@"
    )
    return $?
  fi
  echo "error: chimera-cli binary is missing and cargo fallback is disabled" >&2
  return 1
}

run_chimera_gateway() {
  if [[ -x "$CHIMERA_RUNNER" ]]; then
    "$CHIMERA_RUNNER" gateway "$@"
    return $?
  fi
  if [[ -x "$CHIMERA_GATEWAY_BIN" ]]; then
    "$CHIMERA_GATEWAY_BIN" "$@"
    return $?
  fi
  if [[ "${CHIMERA_ALLOW_CARGO_FALLBACK:-0}" == "1" ]] && command -v cargo >/dev/null 2>&1; then
    (
      cd "$ROOT_DIR"
      cargo run -q -p chimera-gateway -- "$@"
    )
    return $?
  fi
  echo "error: chimera-gateway binary is missing and cargo fallback is disabled" >&2
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

start_runner_background() {
  local name="${1:?name_required}"
  local pid_file="${2:?pid_file_required}"
  local log_file="${3:?log_file_required}"
  local env_file="${4:?env_file_required}"
  local target="${5:?target_required}"
  local use_sudo="0"
  if [[ -f "$env_file" ]] && grep -q '^CHIMERA_RUNNER_USE_SUDO=1$' "$env_file"; then
    use_sudo="1"
  fi

  if pidfile_running "$pid_file"; then
    local pid
    pid="$(tr -d '[:space:]' <"$pid_file" 2>/dev/null || true)"
    echo "${name}_status=running pid=$pid"
    return 0
  fi

  ensure_parent_dir "$pid_file"
  ensure_parent_dir "$log_file"

  if [[ "$use_sudo" == "1" ]]; then
    nohup sudo -n bash -lc '
      set -euo pipefail
      env_file="$1"
      runner="$2"
      target="$3"
      if [[ ! -f "$env_file" ]]; then
        echo "error: missing env file: $env_file" >&2
        exit 1
      fi
      set -a
      # shellcheck disable=SC1090
      source "$env_file"
      exec "$runner" "$target"
    ' _ "$env_file" "$CHIMERA_RUNNER" "$target" >>"$log_file" 2>&1 &
  else
    nohup bash -lc '
      set -euo pipefail
      env_file="$1"
      runner="$2"
      target="$3"
      if [[ ! -f "$env_file" ]]; then
        echo "error: missing env file: $env_file" >&2
        exit 1
      fi
      set -a
      # shellcheck disable=SC1090
      source "$env_file"
      exec "$runner" "$target"
    ' _ "$env_file" "$CHIMERA_RUNNER" "$target" >>"$log_file" 2>&1 &
  fi

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

stop_runner_background() {
  local name="${1:?name_required}"
  local pid_file="${2:?pid_file_required}"

  if [[ -f "$pid_file" ]]; then
    local pid
    pid="$(tr -d '[:space:]' <"$pid_file" 2>/dev/null || true)"
    if [[ -n "$pid" ]]; then
      kill "$pid" >/dev/null 2>&1 || true
      sleep 0.2
      kill -0 "$pid" >/dev/null 2>&1 && kill -9 "$pid" >/dev/null 2>&1 || true
    fi
    rm -f "$pid_file"
  fi
  echo "${name}_status=stopped"
}

systemd_user_ready() {
  command -v systemctl >/dev/null 2>&1 && systemctl --user show-environment >/dev/null 2>&1
}

systemd_chimera_units_present() {
  local units
  units="$(systemctl --user list-unit-files 2>/dev/null || true)"
  if command -v rg >/dev/null 2>&1; then
    printf '%s\n' "$units" | rg -q '^chimera-gateway\.service|^chimera-client\.service'
  else
    printf '%s\n' "$units" | grep -Eq '^chimera-gateway\.service|^chimera-client\.service'
  fi
}

systemd_units_active_ok() {
  local client_expected="${1:-1}"
  local gateway_state client_state
  gateway_state="$(systemctl --user is-active chimera-gateway.service 2>/dev/null || true)"
  client_state="$(systemctl --user is-active chimera-client.service 2>/dev/null || true)"
  if [[ "$client_expected" == "1" ]]; then
    [[ "$gateway_state" == "active" && "$client_state" == "active" ]]
  else
    [[ "$gateway_state" == "active" ]]
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
  # Detect non-CHIMERA sing-box by config path.
  if pgrep -af 'sing-box run -c ' 2>/dev/null | grep -vE 'chimera|singbox-split\.json' >/dev/null 2>&1; then
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

ensure_vpn_coexist_guard() {
  if [[ "$CHIMERA_ALLOW_WEAVE_COEXIST_MUTATION" == "1" ]]; then
    return 0
  fi
  if foreign_vpn_contours_present; then
    CHIMERA_SYSTEM_INTEGRATION="0"
    if [[ "$CHIMERA_COEXIST_TRANSPARENT_CAPTURE" == "1" ]]; then
      SPLIT_TRANSPARENT_ENABLED="1"
      PROXY_AUTOSYNC="0"
      CHIMERA_FORCE_PROXY_NONE="1"
      echo "chimera_coexist_guard=active action=disable_desktop_proxy_only reason=foreign_vpn_contours_detected"
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
    if ! is_local_upstream_host "${CHIMERA_UPSTREAM_HOST}"; then
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

upstream_probe() {
  if [[ -f "$UPSTREAM_ENV_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$UPSTREAM_ENV_FILE"
  fi
  local endpoint lat parsed transport endpoint_only
  local best="" best_lat=2147483647
  while IFS= read -r endpoint; do
    [[ -z "$endpoint" ]] && continue
    parsed="$(parse_transport_endpoint "$endpoint" || true)"
    transport="${parsed%%|*}"
    endpoint_only="${parsed#*|}"
    lat="$(endpoint_latency_ms_probe "$endpoint")"
    echo "upstream_candidate transport=${transport:-unknown} endpoint=${endpoint_only:-unknown} latency_ms=$lat"
    if [[ "$lat" =~ ^[0-9]+$ ]] && [[ "$lat" -lt "$best_lat" ]]; then
      best_lat="$lat"
      best="$endpoint_only"
    fi
  done < <(build_upstream_candidates)
  if [[ -n "$best" ]]; then
    echo "upstream_best endpoint=$best latency_ms=$best_lat strategy=$UPSTREAM_STRATEGY"
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
  if [[ -f "$UPSTREAM_ENV_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$UPSTREAM_ENV_FILE"
  fi
  echo "upstream_audit_begin"
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
    echo "upstream_last_endpoint=$(awk -F'|' 'NR==1{print $1}' "$LAST_ENDPOINT_FILE" 2>/dev/null || true)"
    echo "upstream_last_endpoint_sticky_until=$(awk -F'|' 'NR==1{print $2}' "$LAST_ENDPOINT_FILE" 2>/dev/null || true)"
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
  if [[ -f "${XDG_CACHE_HOME:-$HOME/.cache}/chimera/singbox-split.log" ]]; then
    echo "upstream_recent_events:"
    tail -n "$lines" "${XDG_CACHE_HOME:-$HOME/.cache}/chimera/singbox-split.log" | grep -E 'route|failover|reason=' || true
  fi
  echo "upstream_audit_end"
}

upstream_failover_smoke() {
  local wait_sec="${1:-10}"
  if ! [[ "$wait_sec" =~ ^[0-9]+$ ]]; then
    wait_sec=10
  fi
  echo "upstream_failover_smoke=transparent_runtime"
  split_transparent_status
  sleep "$wait_sec"
  upstream_audit 200
}

ensure_parent_dir() {
  mkdir -p "$(dirname "${1:?file_required}")"
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

resolve_proxy_url() {
  if [[ -n "${CHIMERA_PROXY_URL:-}" ]]; then
    printf '%s' "$CHIMERA_PROXY_URL"
    return 0
  fi
  if [[ -f "$UPSTREAM_ENV_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$UPSTREAM_ENV_FILE"
    if [[ -n "${CHIMERA_PROXY_URL:-}" ]]; then
      printf '%s' "$CHIMERA_PROXY_URL"
      return 0
    fi
    if [[ -n "${CHIMERA_REAL_WORLD_PROXY_URL:-}" ]]; then
      printf '%s' "$CHIMERA_REAL_WORLD_PROXY_URL"
      return 0
    fi
  fi
  printf 'http://127.0.0.1:18080'
}

runtime_state_is_up() {
  if systemd_user_ready; then
    local gateway_state client_state
    gateway_state="$(systemctl --user is-active chimera-gateway.service 2>/dev/null || true)"
    client_state="$(systemctl --user is-active chimera-client.service 2>/dev/null || true)"
    if client_config_ready; then
      [[ "$gateway_state" == "active" && "$client_state" == "active" ]]
    else
      [[ "$gateway_state" == "active" ]]
    fi
    return $?
  fi
  if client_config_ready; then
    pidfile_running "$(peer_egress_pid_path)" && pidfile_running "$(transparent_runtime_pid_path)"
    return $?
  fi
  if pidfile_running "$(peer_egress_pid_path)"; then
    return 0
  fi
  [[ -f "$STATE_FILE" ]] || return 1
  grep -q '"status"[[:space:]]*:[[:space:]]*"up"' "$STATE_FILE" 2>/dev/null
}

proxy_url_hostport() {
  local url="${1:-}"
  local rest="${url#*://}"
  rest="${rest#*@}"
  rest="${rest%%/*}"
  printf '%s' "$rest"
}

listener_state_for_proxy() {
  local proxy_url="${1:-}"
  local hostport
  hostport="$(proxy_url_hostport "$proxy_url")"
  local host="${hostport%:*}"
  local port="${hostport##*:}"
  if [[ -n "$host" && -n "$port" && "$hostport" != "$proxy_url" ]]; then
    if ss -ltnH 2>/dev/null | awk '{print $4}' | grep -Fxq "127.0.0.1:${port}"; then
      echo "up"
      return 0
    fi
    if ss -ltnH 2>/dev/null | awk '{print $4}' | grep -Fxq "[::1]:${port}"; then
      echo "up"
      return 0
    fi
    if ss -ltnH 2>/dev/null | awk '{print $4}' | grep -Fxq "*:${port}"; then
      echo "up"
      return 0
    fi
  fi
  echo "down"
}

read_runtime_service_state() {
  local unit="${1:?unit_required}"
  if systemd_user_ready; then
    systemctl --user is-active "$unit" 2>/dev/null || true
  else
    if [[ "$unit" == "chimera-gateway.service" ]] && pidfile_running "$(peer_egress_pid_path)"; then
      echo "active"
    elif [[ "$unit" == "chimera-client.service" ]] && pidfile_running "$(transparent_runtime_pid_path)"; then
      echo "active"
    else
      echo "unknown"
    fi
  fi
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
  for file in "$SITE_AUTO_SEEDS_FILE" "$MANUAL_GATEWAY_DOMAINS_FILE" "$ADAPTIVE_DOMAINS_FILE"; do
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
  for file in "$SITE_AUTO_SEEDS_FILE" "$MANUAL_GATEWAY_DOMAINS_FILE" "$SITE_DISCOVERY_DOMAINS_FILE"; do
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
  echo "manual_gateway_domains_file=$MANUAL_GATEWAY_DOMAINS_FILE"
  echo "adaptive_domains_count=$(count_noncomment_lines "$ADAPTIVE_DOMAINS_FILE")"
  echo "manual_gateway_domains_count=$(count_noncomment_lines "$MANUAL_GATEWAY_DOMAINS_FILE")"
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
  echo "manual_gateway_domains_file=$MANUAL_GATEWAY_DOMAINS_FILE"
  echo "adaptive_domains_file=$ADAPTIVE_DOMAINS_FILE"
  echo "manual_gateway_domains_count=$(count_noncomment_lines "$MANUAL_GATEWAY_DOMAINS_FILE")"
  echo "adaptive_domains_count=$(count_noncomment_lines "$ADAPTIVE_DOMAINS_FILE")"
  echo "manual_gateway_domains:"
  if [[ -f "$MANUAL_GATEWAY_DOMAINS_FILE" ]]; then
    awk 'NF && $0 !~ /^[[:space:]]*#/' "$MANUAL_GATEWAY_DOMAINS_FILE"
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
    append_unique_line "$MANUAL_GATEWAY_DOMAINS_FILE" "$domain"
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
    remove_exact_line "$MANUAL_GATEWAY_DOMAINS_FILE" "$domain"
    removed=$((removed + 1))
  done
  site_auto_bootstrap_run >/dev/null 2>&1 || true
  echo "site_remove_status=ok count=$removed"
}

site_auto_watch_run_once() {
  site_auto_discover_run >/dev/null 2>&1 || true
  site_auto_bootstrap_run >/dev/null 2>&1 || true
  echo "site_auto_watch_run_once=ok"
}

site_auto_watch_loop() {
  while true; do
    site_auto_watch_run_once >/dev/null 2>&1 || true
    sleep "$SITE_AUTOWATCH_INTERVAL_SEC"
  done
}

site_auto_watch_status() {
  local pid="unknown"
  if [[ -f "$SITE_AUTOWATCH_PID_FILE" ]]; then
    pid="$(tr -d '[:space:]' <"$SITE_AUTOWATCH_PID_FILE")"
    if [[ -n "$pid" ]] && kill -0 "$pid" >/dev/null 2>&1; then
      echo "site_auto_watch_status=running pid=$pid interval_sec=$SITE_AUTOWATCH_INTERVAL_SEC"
      return 0
    fi
  fi
  echo "site_auto_watch_status=stopped"
}

site_auto_watch_start() {
  if [[ "${SITE_AUTOWATCH_ENABLED}" != "1" ]]; then
    echo "site_auto_watch_status=disabled"
    return 0
  fi
  if [[ -f "$SITE_AUTOWATCH_PID_FILE" ]]; then
    local pid
    pid="$(tr -d '[:space:]' <"$SITE_AUTOWATCH_PID_FILE" 2>/dev/null || true)"
    if [[ -n "$pid" ]] && kill -0 "$pid" >/dev/null 2>&1; then
      echo "site_auto_watch_status=running pid=$pid interval_sec=$SITE_AUTOWATCH_INTERVAL_SEC"
      return 0
    fi
  fi
  ensure_parent_dir "$SITE_AUTOWATCH_PID_FILE"
  nohup "$SCRIPT_PATH" __site-auto-watch-loop >/dev/null 2>&1 &
  local pid=$!
  printf '%s\n' "$pid" >"$SITE_AUTOWATCH_PID_FILE"
  echo "site_auto_watch_status=started pid=$pid interval_sec=$SITE_AUTOWATCH_INTERVAL_SEC"
}

site_auto_watch_stop() {
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

service_proxy_enable() {
  local ids=("$@")
  if [[ "${#ids[@]}" -eq 0 ]]; then
    mapfile -t ids < <(list_config_ids "service:")
  else
    mapfile -t ids < <(resolve_service_ids_for_args "${ids[@]}")
  fi
  local id
  for id in "${ids[@]}"; do
    [[ -n "$id" ]] || continue
    set_service_route_override "$id" "enabled"
  done
  echo "service_proxy_enable_status=ok count=${#ids[@]}"
}

service_proxy_disable() {
  local ids=("$@")
  if [[ "${#ids[@]}" -eq 0 ]]; then
    mapfile -t ids < <(list_config_ids "service:")
  else
    mapfile -t ids < <(resolve_service_ids_for_args "${ids[@]}")
  fi
  local id
  for id in "${ids[@]}"; do
    [[ -n "$id" ]] || continue
    delete_service_route_override "$id"
  done
  echo "service_proxy_disable_status=ok count=${#ids[@]}"
}

service_proxy_enable_running() {
  if systemd_user_ready; then
    mapfile -t running < <(systemctl --user list-units --type=service --state=running --no-legend 2>/dev/null | awk '{print $1}' | sed 's/\.service$//')
    if [[ "${#running[@]}" -gt 0 ]]; then
      service_proxy_enable "${running[@]}"
      return 0
    fi
  fi
  service_proxy_enable "$@"
}

verify_service() {
  local ids=("$@")
  if [[ "${#ids[@]}" -eq 0 ]]; then
    mapfile -t ids < <(list_config_ids "service:")
  else
    mapfile -t ids < <(resolve_service_ids_for_args "${ids[@]}")
  fi
  local failed=0
  local id svc_name state
  for id in "${ids[@]}"; do
    svc_name="$(resolve_service_name "$id")"
    state="$(service_route_override_state "$id")"
    if [[ -n "$svc_name" ]]; then
      echo "verify_service[$id]=pass service=$svc_name override=${state:-none}"
    else
      echo "verify_service[$id]=skip reason=service_not_installed"
    fi
  done
  [[ "$failed" -eq 0 ]]
}

run_app() {
  local app_id="${1:?app_id_required}"
  shift || true
  local command env_spec full_cmd
  command="$(resolve_app_command "$app_id")"
  if [[ -z "$command" ]]; then
    echo "run_app_status=fail reason=app_not_found app_id=$app_id" >&2
    return 2
  fi
  env_spec="$(resolve_app_env "$app_id")"
  full_cmd="$command"
  local arg
  for arg in "$@"; do
    full_cmd+=" $(printf '%q' "$arg")"
  done
  run_shell_command_with_env "$env_spec" "$full_cmd"
}

verify_app() {
  local app_id="${1:?app_id_required}"
  shift || true
  if run_app "$app_id" "$@"; then
    echo "verify_app_status=pass app_id=$app_id"
    return 0
  else
    local rc=$?
    echo "verify_app_status=fail app_id=$app_id exit=$rc" >&2
    return "$rc"
  fi
}

verify_cmd() {
  local full_cmd=""
  local arg
  for arg in "$@"; do
    full_cmd+=" $(printf '%q' "$arg")"
  done
  full_cmd="${full_cmd# }"
  if bash -lc "$full_cmd"; then
    echo "verify_cmd_status=pass"
    return 0
  else
    local rc=$?
    echo "verify_cmd_status=fail exit=$rc" >&2
    return "$rc"
  fi
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

logs_tail() {
  local lines="${1:-200}"
  if ! [[ "$lines" =~ ^[0-9]+$ ]]; then
    lines=200
  fi
  echo "=== gateway log: $GATEWAY_LOG ==="
  tail -n "$lines" "$GATEWAY_LOG" 2>/dev/null || true
  echo "=== client log: $CLIENT_LOG ==="
  tail -n "$lines" "$CLIENT_LOG" 2>/dev/null || true
  echo "=== autofix log: $AUTOFIX_LOG_FILE ==="
  tail -n "$lines" "$AUTOFIX_LOG_FILE" 2>/dev/null || true
}

doctor_run() {
  mkdir -p "$(dirname "$ROOT_DIR/docs/doctor_latest.json")" >/dev/null 2>&1 || true
  if run_chimera_cli doctor --config "$CLIENT_CONFIG_FILE" --json --out "$ROOT_DIR/docs/doctor_latest.json"; then
    echo "doctor_status=ok"
    return 0
  fi
  local rc=$?
  echo "doctor_status=fail exit=$rc" >&2
  return "$rc"
}

start_runtime() {
  ensure_base_path
  ensure_upstream_env_bootstrapped
  if [[ -x "$RUNTIME_BOOTSTRAP_SCRIPT" ]]; then
    "$RUNTIME_BOOTSTRAP_SCRIPT" ensure-singbox >/dev/null 2>&1 || true
  fi
  if systemd_user_ready; then
    systemctl --user daemon-reload >/dev/null 2>&1 || true
    systemctl --user start chimera-gateway.service >/dev/null 2>&1 || true
    if client_config_ready; then
      systemctl --user start chimera-client.service >/dev/null 2>&1 || true
      site_auto_watch_start >/dev/null 2>&1 || true
      echo "start_status=ok mode=systemd_user client=started"
    else
      systemctl --user stop chimera-client.service >/dev/null 2>&1 || true
      site_auto_watch_stop >/dev/null 2>&1 || true
      echo "start_status=ok mode=systemd_user client=skipped reason=no_endpoint"
    fi
    return 0
  fi
  run_chimera_gateway doctor --config "$ROOT_DIR/configs/gateway.example.conf" --json --out "$ROOT_DIR/docs/gateway_doctor_latest.json" >/dev/null 2>&1 || true
  local gateway_status="skipped"
  local client_status="skipped"
  if [[ -f "$PEER_EGRESS_ENV_FILE" ]]; then
    start_runner_background "peer_egress" "$(peer_egress_pid_path)" "$GATEWAY_LOG" "$PEER_EGRESS_ENV_FILE" "peer-egress" >/dev/null 2>&1 || true
    gateway_status="started"
    publish_mesh_discovery_snapshot >/dev/null 2>&1 || true
  fi
  if client_config_ready && [[ -f "$TRANSPARENT_RUNTIME_ENV_FILE" ]]; then
    start_runner_background "transparent_runtime" "$(transparent_runtime_pid_path)" "$CLIENT_LOG" "$TRANSPARENT_RUNTIME_ENV_FILE" "transparent-runtime" >/dev/null 2>&1 || true
    client_status="started"
    run_chimera_cli up \
      --config "$CLIENT_CONFIG_FILE" \
      --state-file "$STATE_FILE" \
      --apply-tun true \
      --apply-route true \
      --apply-dns true >/dev/null 2>&1 || true
    site_auto_watch_start >/dev/null 2>&1 || true
  else
    site_auto_watch_stop >/dev/null 2>&1 || true
  fi
  if client_config_ready; then
    echo "start_status=ok mode=direct gateway=$gateway_status client=$client_status"
  else
    echo "start_status=ok mode=direct gateway=$gateway_status client=skipped reason=no_endpoint"
  fi
}

stop_runtime() {
  site_auto_watch_stop >/dev/null 2>&1 || true
  if systemd_user_ready; then
    systemctl --user stop chimera-client.service >/dev/null 2>&1 || true
    systemctl --user stop chimera-gateway.service >/dev/null 2>&1 || true
    echo "stop_status=ok mode=systemd_user"
    return 0
  fi
  stop_runner_background "transparent_runtime" "$(transparent_runtime_pid_path)" >/dev/null 2>&1 || true
  stop_runner_background "peer_egress" "$(peer_egress_pid_path)" >/dev/null 2>&1 || true
  run_chimera_cli down \
    --config "$CLIENT_CONFIG_FILE" \
    --state-file "$STATE_FILE" \
    --apply-tun true \
    --apply-route true \
    --apply-dns true >/dev/null 2>&1 || true
  echo "stop_status=ok mode=direct"
}

restart_runtime() {
  stop_runtime >/dev/null 2>&1 || true
  start_runtime
}

runtime_status() {
  local gateway_state client_state proxy_url proxy_listener route_mode split_mode watch_status
  gateway_state="$(read_runtime_service_state chimera-gateway.service)"
  client_state="$(read_runtime_service_state chimera-client.service)"
  proxy_url="$(resolve_proxy_url)"
  proxy_listener="$(listener_state_for_proxy "$proxy_url")"
  route_mode="$(read_route_mode)"
  split_mode="$(read_split_list_mode)"
  watch_status="$(site_auto_watch_status)"
  echo "runtime_root=$ROOT_DIR"
  echo "gateway_service_state=$gateway_state"
  echo "client_service_state=$client_state"
  echo "peer_egress_state_file=$(peer_egress_state_path)"
  echo "chimera_proxy_url=$proxy_url"
  echo "chimera_proxy_listener=$proxy_listener"
  echo "$watch_status"
  if runtime_state_is_up; then
    echo "transparent_runtime=running"
    echo "runtime_state_status=up"
  else
    if systemd_user_ready; then
      if client_config_ready; then
        echo "transparent_runtime=$([[ "$client_state" == "active" ]] && echo running || echo stopped)"
      else
        echo "transparent_runtime=$([[ "$gateway_state" == "active" ]] && echo running || echo stopped)"
      fi
    else
      if client_config_ready; then
        if pidfile_running "$(peer_egress_pid_path)" && pidfile_running "$(transparent_runtime_pid_path)"; then
          echo "transparent_runtime=running"
        else
          echo "transparent_runtime=stopped"
        fi
      else
        if pidfile_running "$(peer_egress_pid_path)"; then
          echo "transparent_runtime=running"
        else
          echo "transparent_runtime=stopped"
        fi
      fi
    fi
    echo "runtime_state_status=unknown"
  fi
  echo "route_mode=$route_mode"
  echo "split_list_mode=$split_mode"
  if client_config_ready; then
    echo "client_config_ready=true"
  else
    echo "client_config_ready=false"
  fi
  if [[ -f "$STATE_FILE" ]]; then
    echo "state_file=$STATE_FILE"
    awk -F= '
      /^carrier\.addr[[:space:]]*=/ { print "carrier_addr=" $2 }
      /^selected_node[[:space:]]*=/ { print "selected_node=" $2 }
      /^mesh_node[[:space:]]*=/ { print "mesh_node=" $2 }
      /^autoconnect[[:space:]]*=/ { print "autoconnect=" $2 }
    ' "$STATE_FILE" 2>/dev/null || true
  fi
  if [[ -f "$(peer_egress_state_path)" ]]; then
    echo "peer_egress_state=$(peer_egress_state_path)"
    awk -F= '
      /^resolved_local_listen[[:space:]]*=/ { print "peer_egress_resolved_local_listen=" $2 }
      /^resolved_peer_listen[[:space:]]*=/ { print "peer_egress_resolved_peer_listen=" $2 }
      /^mode[[:space:]]*=/ { print "peer_egress_mode=" $2 }
    ' "$(peer_egress_state_path)" 2>/dev/null || true
  fi
}

proxy_status() {
  runtime_status
}

route_status() {
  local proxy_url proxy_listener app_routes_count service_routes_count manual_count adaptive_count
  proxy_url="$(resolve_proxy_url)"
  proxy_listener="$(listener_state_for_proxy "$proxy_url")"
  app_routes_count="$(count_config_prefix "$APP_ROUTES_FILE" "app:")"
  service_routes_count="$(count_config_prefix "$APP_ROUTES_FILE" "service:")"
  manual_count="$(count_noncomment_lines "$MANUAL_GATEWAY_DOMAINS_FILE")"
  adaptive_count="$(count_noncomment_lines "$ADAPTIVE_DOMAINS_FILE")"
  echo "chimera_proxy_url=$proxy_url"
  echo "chimera_proxy_listener=$proxy_listener"
  echo "route_mode=$(read_route_mode)"
  echo "split_list_mode=$(read_split_list_mode)"
  echo "app_routes_count=$app_routes_count"
  echo "service_routes_count=$service_routes_count"
  echo "manual_gateway_domains_count=$manual_count"
  echo "adaptive_domains_count=$adaptive_count"
  if [[ -f "$SERVICE_ROUTE_OVERRIDES_FILE" ]]; then
    awk -F= '/^service_route_override\[/{print}' "$SERVICE_ROUTE_OVERRIDES_FILE"
  fi
}

split_transparent_status() {
  proxy_status
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
  stop_runtime >/dev/null 2>&1 || true
  if systemd_user_ready; then
    systemctl --user disable --now chimera-gateway.service chimera-client.service >/dev/null 2>&1 || true
  fi
  rm -f "$STATE_FILE" "$GATEWAY_LOG" "$CLIENT_LOG" "$LAST_ENDPOINT_FILE" "$UPSTREAM_HEALTH_STATE_FILE" "$SITE_AUTOWATCH_PID_FILE" "$(peer_egress_pid_path)" "$(transparent_runtime_pid_path)"
  rm -f "$(peer_egress_state_path)"
  rm -f "${XDG_CONFIG_HOME:-$HOME/.config}/chimera/peer-egress.env" "${XDG_CONFIG_HOME:-$HOME/.config}/chimera/transparent-runtime.env"
  rm -f "$SERVICE_ROUTE_OVERRIDES_FILE"
  rm -f "${XDG_CONFIG_HOME:-$HOME/.config}/chimera/site_adaptive_routes.db"
  echo "uninstall_status=ok"
}

main() {
  local cmd="${1:-}"
  case "$cmd" in
    start)
      start_runtime
      ;;
    stop)
      stop_runtime
      ;;
    restart)
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
    proxy-status)
      proxy_status
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
    service-proxy-enable)
      shift || true
      service_proxy_enable "$@"
      ;;
    service-proxy-disable)
      shift || true
      service_proxy_disable "$@"
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
    apps-running)
      apps_running
      ;;
    services-running)
      services_running
      ;;
    service-proxy-enable-running)
      shift || true
      service_proxy_enable_running "$@"
      ;;
    ui-mode)
      shift || true
      ui_mode_dispatch "${1:-show}"
      ;;
    uninstall)
      uninstall_runtime
      ;;
    mesh)
      shift || true
      run_chimera_cli mesh "$@"
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
