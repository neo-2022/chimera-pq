#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
STATE_FILE="${STATE_FILE:-$ROOT_DIR/docs/runtime_state_latest.json}"
GATEWAY_LOG="${GATEWAY_LOG:-$ROOT_DIR/docs/chimera_gateway.service.log}"
CLIENT_LOG="${CLIENT_LOG:-$ROOT_DIR/docs/chimera_client.service.log}"
UI_MODE_FILE="${UI_MODE_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/ui_mode}"
UPSTREAM_ENV_FILE="${UPSTREAM_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/upstream_proxy.env}"
SOCKS_HOST="${SOCKS_HOST:-127.0.0.1}"
SOCKS_PORT="${SOCKS_PORT:-11080}"
SOCKS_PORT_MIN="${SOCKS_PORT_MIN:-12080}"
SOCKS_PORT_MAX="${SOCKS_PORT_MAX:-12180}"
CHIMERA_PROTECTED_PORTS_CSV="${CHIMERA_PROTECTED_PORTS_CSV:-11080,22180}"
CHIMERA_SAFE_HOST_LOCK="${CHIMERA_SAFE_HOST_LOCK:-1}"
CHIMERA_ALLOW_LOCAL_NETWORK_MUTATION="${CHIMERA_ALLOW_LOCAL_NETWORK_MUTATION:-0}"
SOCKS_HEALTHCHECK_URL="${SOCKS_HEALTHCHECK_URL:-https://api.ipify.org}"
SOCKS_HEALTHCHECK_TIMEOUT_SEC="${SOCKS_HEALTHCHECK_TIMEOUT_SEC:-6}"
POLICY_FILE="${POLICY_FILE:-$ROOT_DIR/configs/policy.runtime.conf}"
MANUAL_GATEWAY_DOMAINS_FILE="${MANUAL_GATEWAY_DOMAINS_FILE:-$ROOT_DIR/configs/manual_gateway_domains.txt}"
APP_ROUTES_FILE="${APP_ROUTES_FILE:-$ROOT_DIR/configs/chimera-app-routes.conf}"
PAC_FILE="${PAC_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/chimera-proxy.pac}"
ROUTE_MODE_FILE="${ROUTE_MODE_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/route_mode}"
SPLIT_LIST_MODE_FILE="${SPLIT_LIST_MODE_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/split_list_mode}"
AUTOFIX_SCRIPT="$ROOT_DIR/scripts/chimera-autofix.sh"
WATCHDOG_SCRIPT="$ROOT_DIR/scripts/chimera-socks-watchdog.sh"
UPSTREAM_AUTOBOOTSTRAP_SCRIPT="${UPSTREAM_AUTOBOOTSTRAP_SCRIPT:-$ROOT_DIR/scripts/chimera_upstream_autobootstrap.sh}"
AUTOFIX_TIMEOUT="${CHIMERA_AUTOFIX_MAX_TIME:-25}"
WATCHDOG_PID_FILE="${XDG_RUNTIME_DIR:-/tmp}/chimera-socks-watchdog.pid"
UPSTREAM_SSH_KEY_FILE="${CHIMERA_UPSTREAM_SSH_KEY_FILE:-$HOME/.ssh/id_ed25519}"
SOCKS_TUNNEL_PID_FILE="${XDG_RUNTIME_DIR:-/tmp}/chimera-socks-tunnel.pid"
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
CHIMERA_ALLOW_VPN_COEXIST_MUTATION="${CHIMERA_ALLOW_VPN_COEXIST_MUTATION:-0}"
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

is_port_busy() {
  local port="${1:-}"
  [[ -z "$port" ]] && return 1
  ss -ltn "( sport = :$port )" 2>/dev/null | grep -q "LISTEN"
}

pid_cmdline_contains() {
  local pid="${1:-}"
  local needle="${2:-}"
  [[ -n "$pid" && -n "$needle" ]] || return 1
  [[ -r "/proc/$pid/cmdline" ]] || return 1
  tr '\0' ' ' <"/proc/$pid/cmdline" | grep -Fq -- "$needle"
}

kill_owned_socks_tunnel() {
  local pid=""
  if [[ -f "$SOCKS_TUNNEL_PID_FILE" ]]; then
    pid="$(cat "$SOCKS_TUNNEL_PID_FILE" 2>/dev/null || true)"
  fi
  if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
    if pid_cmdline_contains "$pid" "-D $SOCKS_HOST:$SOCKS_PORT"; then
      kill "$pid" 2>/dev/null || true
    fi
  fi
  rm -f "$SOCKS_TUNNEL_PID_FILE"
}

choose_free_socks_port() {
  local start_port="${1:-11080}"
  local end_port="${2:-11180}"
  local p
  for ((p=start_port; p<=end_port; p++)); do
    if is_protected_port "$p"; then
      continue
    fi
    if ! is_port_busy "$p"; then
      echo "$p"
      return 0
    fi
  done
  return 1
}

refresh_socks_port_from_upstream_env() {
  if [[ ! -f "$UPSTREAM_ENV_FILE" ]]; then
    return 0
  fi
  # shellcheck disable=SC1090
  source "$UPSTREAM_ENV_FILE"
  if [[ -n "${CHIMERA_SOCKS_PORT:-}" ]] && [[ "${CHIMERA_SOCKS_PORT}" =~ ^[0-9]+$ ]]; then
    SOCKS_PORT="$CHIMERA_SOCKS_PORT"
  fi
  if [[ -z "${CHIMERA_UPSTREAM_USER:-}" || -z "${CHIMERA_UPSTREAM_HOST:-}" ]]; then
    return 0
  fi
  local detected_port=""
  detected_port="$(find_matching_tunnel_port "${CHIMERA_UPSTREAM_HOST}" "${CHIMERA_UPSTREAM_USER}" || true)"
  if [[ -n "$detected_port" ]]; then
    SOCKS_PORT="$detected_port"
  fi
}

ensure_socks_port_isolated() {
  refresh_socks_port_from_upstream_env
  if is_protected_port "$SOCKS_PORT"; then
    local remap_port=""
    remap_port="$(choose_free_socks_port "$SOCKS_PORT_MIN" "$SOCKS_PORT_MAX" || true)"
    if [[ -n "$remap_port" ]]; then
      SOCKS_PORT="$remap_port"
      upsert_env_kv "$UPSTREAM_ENV_FILE" "CHIMERA_SOCKS_PORT" "$SOCKS_PORT"
      echo "chimera_port_isolation=protected_port_remap port=$SOCKS_PORT"
      return 0
    fi
  fi
  if is_port_busy "$SOCKS_PORT"; then
    local dynamic_port=""
    dynamic_port="$(choose_free_socks_port "$SOCKS_PORT_MIN" "$SOCKS_PORT_MAX" || true)"
    if [[ -z "$dynamic_port" ]]; then
      dynamic_port="$(choose_free_socks_port 11080 12180 || true)"
    fi
    if [[ -n "$dynamic_port" ]]; then
      SOCKS_PORT="$dynamic_port"
      upsert_env_kv "$UPSTREAM_ENV_FILE" "CHIMERA_SOCKS_PORT" "$SOCKS_PORT"
      echo "chimera_port_isolation=selected port=$SOCKS_PORT"
    fi
  else
    upsert_env_kv "$UPSTREAM_ENV_FILE" "CHIMERA_SOCKS_PORT" "$SOCKS_PORT"
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
  proxy-status   Show SOCKS tunnel and desktop proxy status
  app-routes-status  Show parsed app/service routing config
  route-status       Show split routing runtime status
  run-app <app_id> [args...]
                Run selected app through CHIMERA proxy env only
  verify-app <app_id> [args...]
                Verify app run with strict CHIMERA proxy channel checks
  verify-cmd <command...>
                Verify any command/binary through CHIMERA proxy env
  service-proxy-enable [service...]
                Enable CHIMERA proxy env for selected user services
  service-proxy-disable [service...]
                Disable CHIMERA proxy env override for selected user services
  verify-service <service...>
                Verify user service proxy override and active environment
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
                System-level split capture via sing-box TUN:
                blocked domains -> CHIMERA SOCKS, other traffic -> direct
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
  addr="$(awk -F'= ' '$1=="carrier.addr" {print $2; exit}' "$config_path" 2>/dev/null || true)"
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

proxy_env_url() {
  local runtime_port="$SOCKS_PORT"
  if [[ -f "$UPSTREAM_ENV_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$UPSTREAM_ENV_FILE"
    if [[ -n "${CHIMERA_UPSTREAM_HOST:-}" && -n "${CHIMERA_UPSTREAM_USER:-}" ]]; then
      local detected_port=""
      detected_port="$(find_matching_tunnel_port "${CHIMERA_UPSTREAM_HOST}" "${CHIMERA_UPSTREAM_USER}" || true)"
      if [[ -n "$detected_port" ]]; then
        runtime_port="$detected_port"
      fi
    fi
  fi
  echo "socks5h://$SOCKS_HOST:$runtime_port"
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
  # If host already has non-CHIMERA VPN stack, avoid route/tun mutations by default.
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
  if [[ "$CHIMERA_ALLOW_VPN_COEXIST_MUTATION" == "1" ]]; then
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

force_desktop_proxy_none() {
  if [[ "$CHIMERA_FORCE_PROXY_NONE" != "1" ]]; then
    return 0
  fi
  if ! desktop_proxy_supported; then
    return 0
  fi
  gsettings set org.gnome.system.proxy mode 'none' 2>/dev/null || true
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

force_desktop_proxy_auto() {
  if ! desktop_proxy_supported; then
    return 0
  fi
  build_pac_file
  gsettings set org.gnome.system.proxy mode 'auto' 2>/dev/null || true
  gsettings set org.gnome.system.proxy autoconfig-url "file://$PAC_FILE" 2>/dev/null || true
  gsettings set org.gnome.system.proxy.socks host "$SOCKS_HOST" 2>/dev/null || true
  gsettings set org.gnome.system.proxy.socks port "$SOCKS_PORT" 2>/dev/null || true
}

proxy_healthcheck_ok() {
  env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
    curl -sS \
      --proxy "socks5h://$SOCKS_HOST:$SOCKS_PORT" \
      --connect-timeout "$SOCKS_HEALTHCHECK_TIMEOUT_SEC" \
      --max-time "$SOCKS_HEALTHCHECK_TIMEOUT_SEC" \
      "$SOCKS_HEALTHCHECK_URL" >/dev/null 2>&1
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
  if [[ -f "${XDG_CACHE_HOME:-$HOME/.cache}/chimera/socks-watchdog.log" ]]; then
    echo "upstream_recent_events:"
    tail -n "$lines" "${XDG_CACHE_HOME:-$HOME/.cache}/chimera/socks-watchdog.log" | grep -E 'endpoint_probe|failover|reason=' || true
  fi
  echo "upstream_audit_end"
}

upstream_failover_smoke() {
  local wait_sec="${1:-10}"
  if ! [[ "$wait_sec" =~ ^[0-9]+$ ]]; then
    wait_sec=10
  fi
  start_socks_watchdog || true
  local before="down"
  local after="down"
  if ss -ltn "( sport = :$SOCKS_PORT )" 2>/dev/null | grep -q "$SOCKS_HOST:$SOCKS_PORT"; then
    before="up"
  fi
  kill_owned_socks_tunnel
  if [[ "$CHIMERA_ALLOW_PGREP_KILL" == "1" ]] && ! is_protected_port "$SOCKS_PORT"; then
    pkill -f "ssh .* -D $SOCKS_HOST:$SOCKS_PORT .*@.*" 2>/dev/null || true
    pkill -f "ssh .* -D $SOCKS_PORT .*@.*" 2>/dev/null || true
  fi
  sleep 1
  if ss -ltn "( sport = :$SOCKS_PORT )" 2>/dev/null | grep -q "$SOCKS_HOST:$SOCKS_PORT"; then
    after="up"
  fi
  echo "upstream_failover_smoke_drop before=$before after=$after"
  sleep "$wait_sec"
  upstream_audit 200
}

launch_socks_tunnel_for_endpoint() {
  local parsed transport endpoint
  parsed="$(parse_transport_endpoint "${1:-}" || true)"
  transport="${parsed%%|*}"
  endpoint="${parsed#*|}"
  [[ -z "$endpoint" ]] && return 1
  local host="${endpoint%:*}"
  local port="${endpoint##*:}"
  [[ -z "$host" || -z "$port" ]] && return 1
  [[ "$port" =~ ^[0-9]+$ ]] || return 1
  case "$transport" in
    ""|ssh|ssh22|ssh-22|ssh443|ssh-443|ssh8443|ssh-8443|tls|tcp443|tcp8443) ;;
    *)
      echo "upstream_skip transport=$transport reason=unsupported"
      return 1
      ;;
  esac
  local ssh_cmd=()
  if [[ -r "$UPSTREAM_SSH_KEY_FILE" ]]; then
    ssh_cmd=(ssh -i "$UPSTREAM_SSH_KEY_FILE" -o BatchMode=yes)
  elif [[ -n "${CHIMERA_UPSTREAM_PASS:-}" ]]; then
    ssh_cmd=(sshpass -e ssh)
    export SSHPASS="$CHIMERA_UPSTREAM_PASS"
  else
    echo "upstream_skip reason=no_ssh_key_or_password"
    return 1
  fi
  nohup "${ssh_cmd[@]}" \
    -o StrictHostKeyChecking=no \
    -o ExitOnForwardFailure=yes \
    -o ServerAliveInterval=30 \
    -o ServerAliveCountMax=3 \
    -N -D "$SOCKS_HOST:$SOCKS_PORT" \
    -p "$port" \
      "$CHIMERA_UPSTREAM_USER@$host" >/dev/null 2>&1 &
  local ssh_pid=$!
  printf '%s\n' "$ssh_pid" >"$SOCKS_TUNNEL_PID_FILE"
  for _ in 1 2 3 4 5 6; do
    sleep 1
    if ss -ltn "( sport = :$SOCKS_PORT )" 2>/dev/null | grep -q "$SOCKS_HOST:$SOCKS_PORT"; then
      if proxy_healthcheck_ok; then
        return 0
      fi
      kill "$ssh_pid" 2>/dev/null || true
      if [[ "$CHIMERA_ALLOW_PGREP_KILL" == "1" ]]; then
        pkill -f "ssh .* -D $SOCKS_HOST:$SOCKS_PORT .*@.*" 2>/dev/null || true
      fi
      return 1
    fi
  done
  kill "$ssh_pid" 2>/dev/null || true
  rm -f "$SOCKS_TUNNEL_PID_FILE"
  return 1
}

parse_env_pairs() {
  local raw="${1:-}"
  local out=()
  local pair trimmed
  IFS=';' read -r -a out <<<"$raw"
  for pair in "${out[@]}"; do
    trimmed="$(trim_ascii "$pair")"
    [[ -z "$trimmed" ]] && continue
    if [[ "$trimmed" != *=* ]]; then
      continue
    fi
    printf '%s\n' "$trimmed"
  done
}

service_override_dir() {
  local svc="${1:?service_name_required}"
  echo "${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/${svc}.d"
}

service_override_file() {
  local svc="${1:?service_name_required}"
  echo "$(service_override_dir "$svc")/90-chimera-proxy.conf"
}

parse_app_routes_file() {
  local line key value
  [[ -f "$APP_ROUTES_FILE" ]] || return 0
  while IFS= read -r line; do
    line="${line%%#*}"
    line="$(trim_ascii "$line")"
    [[ -z "$line" ]] && continue
    key="${line%%=*}"
    value="${line#*=}"
    key="$(trim_ascii "$key")"
    value="$(trim_ascii "$value")"
    if [[ -z "$key" || -z "$value" ]]; then
      continue
    fi
    case "$key" in
      app:*)
        local app_id="${key#app:}"
        APP_ROUTE_IDS+=("$app_id")
        APP_ROUTE_CMDS["$app_id"]="$value"
        ;;
      app-env:*)
        local app_env_id="${key#app-env:}"
        APP_ROUTE_ENV["$app_env_id"]="$value"
        ;;
      service:*)
        local svc_id="${key#service:}"
        SERVICE_ROUTE_IDS+=("$svc_id")
        SERVICE_ROUTE_NAMES["$svc_id"]="$value"
        ;;
      service-env:*)
        local svc_env_id="${key#service-env:}"
        SERVICE_ROUTE_ENV["$svc_env_id"]="$value"
        ;;
      *)
        ;;
    esac
  done <"$APP_ROUTES_FILE"
}

upsert_route_line() {
  local file="$1"
  local key="$2"
  local value="$3"
  mkdir -p "$(dirname "$file")"
  touch "$file"
  local esc_key esc_val
  esc_key="$(printf '%s' "$key" | sed 's/[^^]/[&]/g; s/\^/\\^/g')"
  esc_val="$(printf '%s' "$value" | sed 's/[&/\]/\\&/g')"
  if rg -n "^${esc_key}=" "$file" >/dev/null 2>&1; then
    sed -i "s/^${esc_key}=.*/${key}=${esc_val}/" "$file"
  else
    printf '%s=%s\n' "$key" "$value" >> "$file"
  fi
}

normalize_app_id() {
  local raw="${1:-}"
  raw="$(trim_ascii "$raw")"
  raw="${raw,,}"
  raw="$(printf '%s' "$raw" | sed 's/[^a-z0-9._-]/_/g')"
  raw="${raw##_}"
  raw="${raw%%_}"
  printf '%s' "$raw"
}

print_app_routes_status() {
  echo "app_routes_file=$APP_ROUTES_FILE"
  if [[ ! -f "$APP_ROUTES_FILE" ]]; then
    echo "app_routes_config=missing"
    return 0
  fi
  parse_app_routes_file
  echo "app_routes_count=${#APP_ROUTE_IDS[@]}"
  for id in "${APP_ROUTE_IDS[@]}"; do
    echo "app_route[$id]=${APP_ROUTE_CMDS[$id]}"
    if [[ -n "${APP_ROUTE_ENV[$id]:-}" ]]; then
      echo "app_route_env[$id]=${APP_ROUTE_ENV[$id]}"
    fi
  done
  echo "service_routes_count=${#SERVICE_ROUTE_IDS[@]}"
  for id in "${SERVICE_ROUTE_IDS[@]}"; do
    echo "service_route[$id]=${SERVICE_ROUTE_NAMES[$id]}"
    if [[ -n "${SERVICE_ROUTE_ENV[$id]:-}" ]]; then
      echo "service_route_env[$id]=${SERVICE_ROUTE_ENV[$id]}"
    fi
    if [[ -f "$(service_override_file "${SERVICE_ROUTE_NAMES[$id]}")" ]]; then
      echo "service_route_override[$id]=enabled"
    else
      echo "service_route_override[$id]=disabled"
    fi
  done
}

run_app_via_proxy() {
  local app_id="${1:-}"
  shift || true
  if [[ -z "$app_id" ]]; then
    echo "error: run-app requires <app_id>" >&2
    exit 1
  fi
  parse_app_routes_file
  local cmd="${APP_ROUTE_CMDS[$app_id]:-}"
  if [[ -z "$cmd" ]]; then
    echo "error: app route not found for id: $app_id" >&2
    exit 1
  fi
  local proxy_url
  proxy_url="$(proxy_env_url)"
  echo "run-app: id=$app_id cmd=$cmd proxy=$proxy_url"
  local full_cmd="$cmd"
  if [[ "$#" -gt 0 ]]; then
    local arg
    for arg in "$@"; do
      full_cmd+=" $(printf '%q' "$arg")"
    done
  fi
  local env_pairs=()
  local pair
  while IFS= read -r pair; do
    env_pairs+=("$pair")
  done < <(parse_env_pairs "${APP_ROUTE_ENV[$app_id]:-}")
  env -u CURL_CA_BUNDLE -u SSL_CERT_FILE \
    "HTTP_PROXY=$proxy_url" \
    "HTTPS_PROXY=$proxy_url" \
    "ALL_PROXY=$proxy_url" \
    "NO_PROXY=localhost,127.0.0.1,::1" \
    "http_proxy=$proxy_url" \
    "https_proxy=$proxy_url" \
    "all_proxy=$proxy_url" \
    "no_proxy=localhost,127.0.0.1,::1" \
    "${env_pairs[@]}" \
    bash -lc "$full_cmd"
}

ensure_proxy_channel_ready() {
  refresh_socks_port_from_upstream_env
  if ! ss -ltn "( sport = :$SOCKS_PORT )" 2>/dev/null | grep -q "$SOCKS_HOST:$SOCKS_PORT"; then
    echo "proxy_channel_ready=false reason=listener_down host=$SOCKS_HOST port=$SOCKS_PORT"
    return 2
  fi
  if ! proxy_healthcheck_ok; then
    echo "proxy_channel_ready=false reason=healthcheck_failed host=$SOCKS_HOST port=$SOCKS_PORT url=$SOCKS_HEALTHCHECK_URL"
    return 2
  fi
  echo "proxy_channel_ready=true host=$SOCKS_HOST port=$SOCKS_PORT"
  return 0
}

verify_app_via_proxy() {
  local app_id="${1:-}"
  shift || true
  if [[ -z "$app_id" ]]; then
    echo "error: verify-app requires <app_id>" >&2
    exit 1
  fi
  if ! ensure_proxy_channel_ready; then
    return 2
  fi
  local out_file=""
  out_file="$(mktemp)"
  set +e
  run_app_via_proxy "$app_id" "$@" >"$out_file" 2>&1
  local rc=$?
  set -e
  echo "verify_app_id=$app_id"
  echo "verify_app_exit_code=$rc"
  sed -n '1,120p' "$out_file"
  rm -f "$out_file"
  if [[ "$rc" -eq 0 ]]; then
    echo "verify_app_status=pass"
    return 0
  fi
  echo "verify_app_status=fail"
  return "$rc"
}

verify_cmd_via_proxy() {
  if [[ "$#" -eq 0 ]]; then
    echo "error: verify-cmd requires <command...>" >&2
    exit 1
  fi
  if ! ensure_proxy_channel_ready; then
    return 2
  fi
  local proxy_url
  proxy_url="$(proxy_env_url)"
  local out_file
  out_file="$(mktemp)"
  set +e
  env -u CURL_CA_BUNDLE -u SSL_CERT_FILE \
    "HTTP_PROXY=$proxy_url" \
    "HTTPS_PROXY=$proxy_url" \
    "ALL_PROXY=$proxy_url" \
    "NO_PROXY=localhost,127.0.0.1,::1" \
    "http_proxy=$proxy_url" \
    "https_proxy=$proxy_url" \
    "all_proxy=$proxy_url" \
    "no_proxy=localhost,127.0.0.1,::1" \
    "$@" >"$out_file" 2>&1
  local rc=$?
  set -e
  echo "verify_cmd=$*"
  echo "verify_cmd_exit_code=$rc"
  sed -n '1,120p' "$out_file"
  rm -f "$out_file"
  if [[ "$rc" -eq 0 ]]; then
    echo "verify_cmd_status=pass"
    return 0
  fi
  echo "verify_cmd_status=fail"
  return "$rc"
}

service_proxy_enable() {
  parse_app_routes_file
  local proxy_url
  proxy_url="$(proxy_env_url)"
  local services=("$@")
  if [[ "${#services[@]}" -eq 0 ]]; then
    services=("${SERVICE_ROUTE_NAMES[@]}")
  else
    local resolved=()
    local svc
    for svc in "${services[@]}"; do
      if [[ -n "${SERVICE_ROUTE_NAMES[$svc]:-}" ]]; then
        resolved+=("${SERVICE_ROUTE_NAMES[$svc]}")
      else
        resolved+=("$svc")
      fi
    done
    services=("${resolved[@]}")
  fi
  if [[ "${#services[@]}" -eq 0 ]]; then
    echo "service-proxy-enable: no services configured"
    return 0
  fi
  require_cmd systemctl
  if ! ensure_proxy_channel_ready; then
    return 2
  fi
  for svc in "${services[@]}"; do
    local matched_id=""
    local id
    for id in "${SERVICE_ROUTE_IDS[@]}"; do
      if [[ "${SERVICE_ROUTE_NAMES[$id]}" == "$svc" ]]; then
        matched_id="$id"
        break
      fi
    done
    local override
    override="$(service_override_file "$svc")"
    mkdir -p "$(dirname "$override")"
    {
      echo "[Service]"
      echo "Environment=HTTP_PROXY=$proxy_url"
      echo "Environment=HTTPS_PROXY=$proxy_url"
      echo "Environment=ALL_PROXY=$proxy_url"
      echo "Environment=NO_PROXY=localhost,127.0.0.1,::1"
      echo "Environment=http_proxy=$proxy_url"
      echo "Environment=https_proxy=$proxy_url"
      echo "Environment=all_proxy=$proxy_url"
      echo "Environment=no_proxy=localhost,127.0.0.1,::1"
      if [[ -n "$matched_id" && -n "${SERVICE_ROUTE_ENV[$matched_id]:-}" ]]; then
        local pair
        while IFS= read -r pair; do
          echo "Environment=$pair"
        done < <(parse_env_pairs "${SERVICE_ROUTE_ENV[$matched_id]}")
      fi
    } >"$override"
    systemctl --user daemon-reload
    systemctl --user restart "$svc" || true
    echo "service_proxy_enabled=$svc proxy=$proxy_url override=$override"
  done
}

verify_service_proxy() {
  local services=("$@")
  parse_app_routes_file
  if [[ "${#services[@]}" -eq 0 ]]; then
    services=("${SERVICE_ROUTE_NAMES[@]}")
  fi
  if [[ "${#services[@]}" -eq 0 ]]; then
    echo "verify_service_status=skip reason=no_services_configured"
    return 0
  fi
  require_cmd systemctl
  local proxy_url
  proxy_url="$(proxy_env_url)"
  local failed=0
  local svc
  for svc in "${services[@]}"; do
    local load_state
    load_state="$(systemctl --user show "$svc" -p LoadState --value 2>/dev/null || true)"
    if [[ "$load_state" == "not-found" || -z "$load_state" ]]; then
      echo "verify_service[$svc]=skip reason=service_not_installed"
      continue
    fi
    local override
    override="$(service_override_file "$svc")"
    if [[ ! -f "$override" ]]; then
      echo "verify_service[$svc]=fail reason=override_missing file=$override"
      failed=1
      continue
    fi
    local env_line
    env_line="$(systemctl --user show "$svc" -p Environment --value 2>/dev/null || true)"
    if [[ "$env_line" == *"HTTP_PROXY=$proxy_url"* && "$env_line" == *"ALL_PROXY=$proxy_url"* ]]; then
      echo "verify_service[$svc]=pass proxy=$proxy_url"
    else
      echo "verify_service[$svc]=fail reason=proxy_env_not_active proxy=$proxy_url"
      failed=1
    fi
  done
  if [[ "$failed" -eq 0 ]]; then
    echo "verify_service_status=pass"
    return 0
  fi
  echo "verify_service_status=fail"
  return 2
}

service_proxy_disable() {
  parse_app_routes_file
  local services=("$@")
  if [[ "${#services[@]}" -eq 0 ]]; then
    services=("${SERVICE_ROUTE_NAMES[@]}")
  else
    local resolved=()
    local svc
    for svc in "${services[@]}"; do
      if [[ -n "${SERVICE_ROUTE_NAMES[$svc]:-}" ]]; then
        resolved+=("${SERVICE_ROUTE_NAMES[$svc]}")
      else
        resolved+=("$svc")
      fi
    done
    services=("${resolved[@]}")
  fi
  if [[ "${#services[@]}" -eq 0 ]]; then
    echo "service-proxy-disable: no services configured"
    return 0
  fi
  require_cmd systemctl
  for svc in "${services[@]}"; do
    local override
    override="$(service_override_file "$svc")"
    rm -f "$override"
    rmdir "$(dirname "$override")" 2>/dev/null || true
    systemctl --user daemon-reload
    systemctl --user restart "$svc" || true
    echo "service_proxy_disabled=$svc override_removed=$override"
  done
}

print_route_status() {
  refresh_socks_port_from_upstream_env
  local proxy_url
  proxy_url="$(proxy_env_url)"
  echo "chimera_proxy_url=$proxy_url"
  echo "route_mode=$(read_route_mode)"
  echo "split_list_mode=$(read_split_list_mode)"
  if ss -ltn "( sport = :$SOCKS_PORT )" 2>/dev/null | grep -q "$SOCKS_HOST:$SOCKS_PORT"; then
    echo "chimera_proxy_listener=up"
  else
    echo "chimera_proxy_listener=down"
  fi
  echo "upstream_strategy=$UPSTREAM_STRATEGY"
  echo "upstream_sticky_sec=$UPSTREAM_STICKY_SEC"
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
  print_app_routes_status
}

apply_desktop_proxy() {
  if [[ "$CHIMERA_SYSTEM_INTEGRATION" != "1" ]]; then
    return 0
  fi
  local mode="$1"
  if ! desktop_proxy_supported; then
    return 0
  fi
  if [[ "$mode" == "manual" ]]; then
    gsettings set org.gnome.system.proxy mode 'manual' 2>/dev/null || true
    gsettings set org.gnome.system.proxy.socks host "$SOCKS_HOST" 2>/dev/null || true
    gsettings set org.gnome.system.proxy.socks port "$SOCKS_PORT" 2>/dev/null || true
  elif [[ "$mode" == "auto" ]]; then
    gsettings set org.gnome.system.proxy mode 'auto' 2>/dev/null || true
    gsettings set org.gnome.system.proxy autoconfig-url "file://$PAC_FILE" 2>/dev/null || true
  else
    gsettings set org.gnome.system.proxy mode 'none' 2>/dev/null || true
  fi
}

apply_route_mode() {
  local mode
  mode="$(read_route_mode)"
  case "$mode" in
    full)
      apply_desktop_proxy manual
      ;;
    split)
      build_pac_file
      apply_desktop_proxy auto
      ;;
    off)
      apply_desktop_proxy none
      ;;
    *)
      apply_desktop_proxy auto
      ;;
  esac
}

auto_sync_desktop_proxy_port() {
  if [[ "$CHIMERA_SYSTEM_INTEGRATION" != "1" ]]; then
    return 0
  fi
  refresh_socks_port_from_upstream_env
  if [[ "$PROXY_AUTOSYNC" != "1" ]]; then
    return 0
  fi
  if ! desktop_proxy_supported; then
    return 0
  fi

  local mode host port
  mode="$(gsettings get org.gnome.system.proxy mode 2>/dev/null || echo "'unknown'")"
  host="$(gsettings get org.gnome.system.proxy.socks host 2>/dev/null || echo "'unknown'")"
  port="$(gsettings get org.gnome.system.proxy.socks port 2>/dev/null || echo "0")"
  host="${host//\'}"

  if [[ "$mode" == "'manual'" ]]; then
    if [[ "$host" != "$SOCKS_HOST" ]] || [[ "$port" != "$SOCKS_PORT" ]]; then
      gsettings set org.gnome.system.proxy.socks host "$SOCKS_HOST" || true
      gsettings set org.gnome.system.proxy.socks port "$SOCKS_PORT" || true
      echo "chimera_proxy_autosync=applied mode=manual host=$SOCKS_HOST port=$SOCKS_PORT"
    fi
  elif [[ "$mode" == "'auto'" ]]; then
    # Keep PAC mode, but make SOCKS tuple consistent for apps that still read socks settings.
    if [[ "$host" != "$SOCKS_HOST" ]] || [[ "$port" != "$SOCKS_PORT" ]]; then
      gsettings set org.gnome.system.proxy.socks host "$SOCKS_HOST" || true
      gsettings set org.gnome.system.proxy.socks port "$SOCKS_PORT" || true
      echo "chimera_proxy_autosync=applied mode=auto host=$SOCKS_HOST port=$SOCKS_PORT"
    fi
  fi
}

build_pac_file() {
  mkdir -p "$(dirname "$PAC_FILE")"
  local tmp_pac
  tmp_pac="$(mktemp)"
  local mode
  mode="$(read_route_mode)"
  local split_mode
  split_mode="$(read_split_list_mode)"
  local has_manual_domains=false
  if [[ -f "$MANUAL_GATEWAY_DOMAINS_FILE" ]] && rg -n '^[[:space:]]*[^#[:space:]].*$' "$MANUAL_GATEWAY_DOMAINS_FILE" >/dev/null 2>&1; then
    has_manual_domains=true
  fi
  {
    echo "function FindProxyForURL(url, host) {"
    echo "  var h = host.toLowerCase();"
    echo "  function m(s) { return dnsDomainIs(h, \".\" + s) || h == s; }"
    if [[ "$mode" == "full" ]]; then
      echo "  return \"SOCKS5 $SOCKS_HOST:$SOCKS_PORT\";"
      echo "}"
      mv "$tmp_pac" "$PAC_FILE"
      return 0
    fi

    # Split mode: domain list controls direction with allow/deny policy.
    if [[ "$mode" == "split" ]]; then
      if [[ "$has_manual_domains" == true ]]; then
        while IFS= read -r raw; do
          local_domain="$(echo "$raw" | tr -d '[:space:]')"
          [[ -z "$local_domain" ]] && continue
          [[ "$local_domain" == \#* ]] && continue
          local_domain="${local_domain#.}"
          if [[ "$split_mode" == "allow" ]]; then
            echo "  if (m(\"$local_domain\")) return \"SOCKS5 $SOCKS_HOST:$SOCKS_PORT\";"
          else
            echo "  if (m(\"$local_domain\")) return \"DIRECT\";"
          fi
        done <"$MANUAL_GATEWAY_DOMAINS_FILE"
      fi
      if [[ "$split_mode" == "allow" ]]; then
        echo "  return \"DIRECT\";"
      else
        echo "  return \"SOCKS5 $SOCKS_HOST:$SOCKS_PORT\";"
      fi
      echo "}"
      mv "$tmp_pac" "$PAC_FILE"
      return 0
    fi

    if [[ -f "$POLICY_FILE" ]]; then
      awk -F'suffix:| => ' '/suffix:.*=> gateway/ {print $2}' "$POLICY_FILE" | while IFS= read -r suffix; do
        suffix="$(echo "$suffix" | xargs)"
        [[ -z "$suffix" ]] && continue
        suffix="${suffix#.}"
        echo "  if (m(\"$suffix\")) return \"SOCKS5 $SOCKS_HOST:$SOCKS_PORT\";"
      done
    fi
    if [[ -f "$MANUAL_GATEWAY_DOMAINS_FILE" ]]; then
      while IFS= read -r raw; do
        domain="$(echo "$raw" | tr -d '[:space:]')"
        [[ -z "$domain" ]] && continue
        [[ "$domain" == \#* ]] && continue
        domain="${domain#.}"
        echo "  if (m(\"$domain\")) return \"SOCKS5 $SOCKS_HOST:$SOCKS_PORT\";"
      done <"$MANUAL_GATEWAY_DOMAINS_FILE"
    fi
    echo "  return \"DIRECT\";"
    echo "}"
  } >"$tmp_pac"
  mv "$tmp_pac" "$PAC_FILE"
}

normalize_domain() {
  local raw="${1:-}"
  raw="$(trim_ascii "$raw")"
  raw="${raw,,}"
  raw="${raw#http://}"
  raw="${raw#https://}"
  raw="${raw%%/*}"
  raw="${raw%%:*}"
  raw="${raw#.}"
  raw="${raw%.}"
  printf '%s' "$raw"
}

site_adaptive_upsert() {
  local domain="$1"
  local decision="$2"
  local direct_ok_streak="${3:-0}"
  local proxy_ok_streak="${4:-0}"
  local reason="${5:-none}"
  mkdir -p "$(dirname "$SITE_ADAPTIVE_DB_FILE")"
  touch "$SITE_ADAPTIVE_DB_FILE"
  local now
  now="$(date +%s)"
  local tmp
  tmp="$(mktemp)"
  awk -F'|' -v d="$domain" '$1 != d {print $0}' "$SITE_ADAPTIVE_DB_FILE" >"$tmp" || true
  printf '%s|%s|%s|%s|%s|%s\n' "$domain" "$decision" "$now" "$direct_ok_streak" "$proxy_ok_streak" "$reason" >>"$tmp"
  mv "$tmp" "$SITE_ADAPTIVE_DB_FILE"
}

site_adaptive_status() {
  if [[ ! -f "$SITE_ADAPTIVE_DB_FILE" ]]; then
    echo "site_adaptive_db=$SITE_ADAPTIVE_DB_FILE (missing)"
    return 0
  fi
  echo "site_adaptive_db=$SITE_ADAPTIVE_DB_FILE"
  awk -F'|' '
    NF>=3{
      d=$1; dec=$2; ts=$3; ds=($4==""?0:$4); ps=($5==""?0:$5); rs=($6==""?"none":$6);
      printf "domain=%s decision=%s ts=%s direct_ok_streak=%s proxy_ok_streak=%s reason=%s\n",d,dec,ts,ds,ps,rs
    }' "$SITE_ADAPTIVE_DB_FILE"
}

site_adaptive_get_decision() {
  local domain="$1"
  [[ -f "$SITE_ADAPTIVE_DB_FILE" ]] || return 1
  awk -F'|' -v d="$domain" '$1==d{dec=$2} END{if(dec!="") print dec}' "$SITE_ADAPTIVE_DB_FILE"
}

site_adaptive_get_record() {
  local domain="$1"
  [[ -f "$SITE_ADAPTIVE_DB_FILE" ]] || return 1
  awk -F'|' -v d="$domain" '
    $1==d{
      dec=$2
      ts=($3==""?0:$3)
      ds=($4==""?0:$4)
      ps=($5==""?0:$5)
      rs=($6==""?"none":$6)
    }
    END{
      if (dec!="") printf "%s|%s|%s|%s|%s\n",dec,ts,ds,ps,rs
    }' "$SITE_ADAPTIVE_DB_FILE"
}

site_adaptive_prune_expired() {
  [[ -f "$SITE_ADAPTIVE_DB_FILE" ]] || return 0
  [[ "$SITE_ADAPTIVE_ENTRY_TTL_SEC" =~ ^[0-9]+$ ]] || return 0
  local now cutoff tmp
  now="$(date +%s)"
  cutoff=$((now - SITE_ADAPTIVE_ENTRY_TTL_SEC))
  tmp="$(mktemp)"
  awk -F'|' -v c="$cutoff" '
    {
      ts=($3==""?0:$3)
      if (ts >= c) print $0
    }' "$SITE_ADAPTIVE_DB_FILE" >"$tmp" || true
  mv "$tmp" "$SITE_ADAPTIVE_DB_FILE"
}

http_code_via_path() {
  local domain="$1"
  local via="$2"
  local code=""
  local ua
  ua="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"
  if [[ "$via" == "proxy" ]]; then
    code="$(curl -sS -o /dev/null -w '%{http_code}' --proxy "socks5h://$SOCKS_HOST:$SOCKS_PORT" -A "$ua" --connect-timeout 12 --max-time 25 "https://$domain" 2>/dev/null || true)"
  else
    code="$(env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
      curl -sS -o /dev/null -w '%{http_code}' -A "$ua" --connect-timeout 12 --max-time 25 "https://$domain" 2>/dev/null || true)"
  fi
  [[ -z "$code" ]] && code="000"
  echo "$code"
}

is_success_http_code() {
  local code="${1:-000}"
  [[ "$code" =~ ^2[0-9][0-9]$ || "$code" =~ ^3[0-9][0-9]$ ]]
}

apply_domain_decision_to_split_list() {
  local domain="$1"
  local decision="$2"
  local split_mode
  split_mode="$(read_split_list_mode)"
  case "$split_mode" in
    allow)
      if [[ "$decision" == "proxy" ]]; then
        site_add "$domain" >/dev/null
      else
        site_remove "$domain" >/dev/null || true
      fi
      ;;
    deny)
      if [[ "$decision" == "direct" ]]; then
        site_add "$domain" >/dev/null
      else
        site_remove "$domain" >/dev/null || true
      fi
      ;;
  esac
}

site_auto_resolve_one() {
  local domain_raw="$1"
  local domain
  domain="$(normalize_domain "$domain_raw")"
  [[ -z "$domain" ]] && return 0

  # Baseline policy in split mode: allow=>default direct, deny=>default proxy.
  local split_mode
  split_mode="$(read_split_list_mode)"
  local first second
  if [[ "$split_mode" == "allow" ]]; then
    first="direct"; second="proxy"
  else
    first="proxy"; second="direct"
  fi

  local first_code second_code decision prev_decision prev_ts prev_direct_streak prev_proxy_streak prev_reason
  prev_decision=""; prev_ts="0"; prev_direct_streak="0"; prev_proxy_streak="0"; prev_reason="none"
  record="$(site_adaptive_get_record "$domain" || true)"
  if [[ -n "$record" ]]; then
    IFS='|' read -r prev_decision prev_ts prev_direct_streak prev_proxy_streak prev_reason <<<"$record"
  fi

  local direct_ok_streak="$prev_direct_streak"
  local proxy_ok_streak="$prev_proxy_streak"
  local decision_reason="fallback_keep"

  first_code="$(http_code_via_path "$domain" "$first")"
  if is_success_http_code "$first_code"; then
    if [[ "$first" == "direct" ]]; then
      direct_ok_streak=$((direct_ok_streak + 1))
      proxy_ok_streak=0
      if [[ "$prev_decision" == "proxy" && "$direct_ok_streak" -lt "$SITE_FAILBACK_DIRECT_THRESHOLD" ]]; then
        decision="proxy"
        decision_reason="hold_proxy_hysteresis"
      else
        decision="$first"
        decision_reason="direct_ok"
      fi
    else
      proxy_ok_streak=$((proxy_ok_streak + 1))
      direct_ok_streak=0
      if [[ "$proxy_ok_streak" -ge "$SITE_FAILOVER_PROXY_THRESHOLD" ]]; then
        decision="$first"
        decision_reason="proxy_ok"
      else
        decision="${prev_decision:-$second}"
        decision_reason="hold_before_proxy_threshold"
      fi
    fi
  else
    second_code="$(http_code_via_path "$domain" "$second")"
    if is_success_http_code "$second_code"; then
      if [[ "$second" == "proxy" ]]; then
        proxy_ok_streak=$((proxy_ok_streak + 1))
        direct_ok_streak=0
        if [[ "$proxy_ok_streak" -ge "$SITE_FAILOVER_PROXY_THRESHOLD" ]]; then
          decision="proxy"
          decision_reason="switch_to_proxy_after_direct_fail"
        else
          decision="${prev_decision:-$first}"
          decision_reason="hold_before_proxy_threshold"
        fi
      else
        direct_ok_streak=$((direct_ok_streak + 1))
        proxy_ok_streak=0
        if [[ "$prev_decision" == "proxy" && "$direct_ok_streak" -lt "$SITE_FAILBACK_DIRECT_THRESHOLD" ]]; then
          decision="proxy"
          decision_reason="hold_proxy_hysteresis"
        else
          decision="direct"
          decision_reason="switch_back_direct"
        fi
      fi
    else
      # If both paths failed transiently, keep last known-good decision when possible.
      if [[ "$prev_decision" == "proxy" || "$prev_decision" == "direct" ]]; then
        decision="$prev_decision"
        decision_reason="both_failed_keep_previous"
      else
        decision="$first"
        decision_reason="both_failed_default"
      fi
      direct_ok_streak=0
      proxy_ok_streak=0
    fi
  fi

  apply_domain_decision_to_split_list "$domain" "$decision"
  site_adaptive_upsert "$domain" "$decision" "$direct_ok_streak" "$proxy_ok_streak" "$decision_reason"
  apply_route_mode
  echo "site_auto_resolve domain=$domain first=$first code_first=${first_code:-000} second=$second code_second=${second_code:-000} decision=$decision reason=$decision_reason direct_ok_streak=$direct_ok_streak proxy_ok_streak=$proxy_ok_streak"
}

site_auto_resolve_many() {
  if [[ "$#" -eq 0 ]]; then
    echo "error: site-auto-resolve requires at least one domain" >&2
    exit 1
  fi
  local d
  for d in "$@"; do
    site_auto_resolve_one "$d"
  done
}

collect_seed_domains() {
  local tmp
  tmp="$(mktemp)"

  if [[ -f "$SITE_AUTO_SEEDS_FILE" ]]; then
    awk '
      { line=$0; sub(/[[:space:]]*#.*/, "", line); gsub(/^[[:space:]]+|[[:space:]]+$/, "", line); if (line!="") print line; }
    ' "$SITE_AUTO_SEEDS_FILE" >>"$tmp" || true
  fi

  if [[ -f "$MANUAL_GATEWAY_DOMAINS_FILE" ]]; then
    awk '
      { line=$0; sub(/[[:space:]]*#.*/, "", line); gsub(/^[[:space:]]+|[[:space:]]+$/, "", line); if (line!="") print line; }
    ' "$MANUAL_GATEWAY_DOMAINS_FILE" >>"$tmp" || true
  fi

  if [[ -f "$SITE_ADAPTIVE_DB_FILE" ]]; then
    awk -F'|' 'NF>=1 && $1!="" {print $1}' "$SITE_ADAPTIVE_DB_FILE" >>"$tmp" || true
  fi

  if [[ -f "$APP_ROUTES_FILE" ]]; then
    # Extract URL-like tokens from app commands and keep host/domain part.
    awk -F'=' '
      /^[[:space:]]*#/ {next}
      /^[[:space:]]*$/ {next}
      NF>=2 {
        rhs=$2
        while (match(rhs, /https?:\/\/[^[:space:]"'\''<>]+/)) {
          u=substr(rhs, RSTART, RLENGTH)
          sub(/^https?:\/\//, "", u)
          sub(/\/.*$/, "", u)
          sub(/:.*/, "", u)
          gsub(/^[.]+|[.]+$/, "", u)
          if (u != "") print u
          rhs=substr(rhs, RSTART+RLENGTH)
        }
      }
    ' "$APP_ROUTES_FILE" >>"$tmp" || true
  fi

  awk '
    {
      d=tolower($0)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", d)
      if (d == "") next
      if (index(d, ".") == 0) next
      print d
    }' "$tmp" | sort -u
  rm -f "$tmp"
}

site_auto_bootstrap() {
  local domains=()
  local d
  while IFS= read -r d; do
    d="$(normalize_domain "$d")"
    [[ -z "$d" ]] && continue
    domains+=("$d")
  done < <(collect_seed_domains)

  if [[ "${#domains[@]}" -eq 0 ]]; then
    echo "site_auto_bootstrap=skip reason=no_seed_domains"
    return 0
  fi

  echo "site_auto_bootstrap=run domains=${#domains[@]}"
  site_auto_resolve_many "${domains[@]}"
}

site_discovery_status() {
  if [[ ! -f "$SITE_DISCOVERY_DOMAINS_FILE" ]]; then
    echo "site_auto_discovery_file=$SITE_DISCOVERY_DOMAINS_FILE (missing)"
    return 0
  fi
  local count
  count="$(awk 'NF{n++} END{print n+0}' "$SITE_DISCOVERY_DOMAINS_FILE")"
  echo "site_auto_discovery_file=$SITE_DISCOVERY_DOMAINS_FILE"
  echo "site_auto_discovery_count=$count"
  sed -n '1,80p' "$SITE_DISCOVERY_DOMAINS_FILE"
}

site_discovery_clear() {
  rm -f "$SITE_DISCOVERY_DOMAINS_FILE"
  echo "site_auto_discovery=cleared"
}

site_discovery_from_resolved_journal() {
  local out_file="$1"
  local since_arg
  since_arg="$(date -u -d "-${SITE_AUTO_DISCOVERY_LOOKBACK_SEC} seconds" '+%Y-%m-%d %H:%M:%S' 2>/dev/null || true)"
  [[ -n "$since_arg" ]] || return 0
  command -v journalctl >/dev/null 2>&1 || return 0
  # shellcheck disable=SC2016
  journalctl --no-pager -u systemd-resolved --since "$since_arg" 2>/dev/null \
    | awk '
      {
        line=$0
        if (match(line, /[A-Za-z0-9._-]+\.[A-Za-z]{2,}/)) {
          d=substr(line, RSTART, RLENGTH)
          gsub(/^[.]+|[.]+$/, "", d)
          print tolower(d)
        }
      }
    ' >>"$out_file" || true
}

site_discovery_from_resolvectl_stats() {
  local out_file="$1"
  command -v resolvectl >/dev/null 2>&1 || return 0
  # resolvectl query only works with explicit domains, so keep as no-op fallback.
  # This hook is reserved for future resolvectl cache parsing support.
  : >>"$out_file"
}

site_auto_discover_run() {
  mkdir -p "$(dirname "$SITE_DISCOVERY_DOMAINS_FILE")"
  local tmp
  tmp="$(mktemp)"
  site_discovery_from_resolved_journal "$tmp"
  site_discovery_from_resolvectl_stats "$tmp"
  awk '
    {
      d=$0
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", d)
      if (d == "") next
      if (d ~ /^localhost$/) next
      if (d ~ /^[0-9.]+$/) next
      if (index(d, ".") == 0) next
      print d
    }
  ' "$tmp" | sort -u >"${tmp}.norm"
  rm -f "$tmp"

  if [[ ! -s "${tmp}.norm" ]]; then
    rm -f "${tmp}.norm"
    echo "site_auto_discovery=skip reason=no_recent_domains"
    return 0
  fi

  if [[ -f "$SITE_DISCOVERY_DOMAINS_FILE" ]]; then
    cat "$SITE_DISCOVERY_DOMAINS_FILE" "${tmp}.norm" | sort -u >"${tmp}.all"
    mv "${tmp}.all" "$SITE_DISCOVERY_DOMAINS_FILE"
  else
    mv "${tmp}.norm" "$SITE_DISCOVERY_DOMAINS_FILE"
  fi
  rm -f "${tmp}.norm"

  local domains=()
  local d
  while IFS= read -r d; do
    d="$(normalize_domain "$d")"
    [[ -z "$d" ]] && continue
    domains+=("$d")
  done <"$SITE_DISCOVERY_DOMAINS_FILE"

  if [[ "${#domains[@]}" -eq 0 ]]; then
    echo "site_auto_discovery=skip reason=normalized_empty"
    return 0
  fi
  echo "site_auto_discovery=run domains=${#domains[@]} lookback_sec=$SITE_AUTO_DISCOVERY_LOOKBACK_SEC"
  site_auto_resolve_many "${domains[@]}"
}

site_auto_watch_run_once() {
  site_adaptive_prune_expired
  if [[ "$SITE_AUTO_DISCOVERY_ENABLED" == "1" ]]; then
    site_auto_discover_run >/dev/null 2>&1 || true
  fi
  if [[ ! -f "$SITE_ADAPTIVE_DB_FILE" ]]; then
    site_auto_bootstrap >/dev/null 2>&1 || true
  fi
  local domains=()
  while IFS='|' read -r d _; do
    [[ -z "$d" ]] && continue
    domains+=("$d")
  done <"$SITE_ADAPTIVE_DB_FILE"
  if [[ "${#domains[@]}" -eq 0 ]]; then
    echo "site_auto_watch=skip reason=empty_db"
    return 0
  fi
  site_auto_resolve_many "${domains[@]}"
  # Enforce persisted decisions into active split list each cycle.
  local d dec
  while IFS='|' read -r d dec _; do
    [[ -z "$d" || -z "$dec" ]] && continue
    if [[ "$dec" == "proxy" || "$dec" == "direct" ]]; then
      apply_domain_decision_to_split_list "$d" "$dec"
    fi
  done <"$SITE_ADAPTIVE_DB_FILE"
  apply_route_mode
}

site_auto_watch_start() {
  if [[ -f "$SITE_AUTOWATCH_PID_FILE" ]]; then
    local old_pid
    old_pid="$(cat "$SITE_AUTOWATCH_PID_FILE" 2>/dev/null || true)"
    if [[ -n "$old_pid" ]] && kill -0 "$old_pid" 2>/dev/null; then
      echo "site_auto_watch=already_running pid=$old_pid"
      return 0
    fi
  fi
  nohup bash -lc "
    echo \$\$ > '$SITE_AUTOWATCH_PID_FILE'
    trap 'rm -f \"$SITE_AUTOWATCH_PID_FILE\"' EXIT
    while true; do
      '$SCRIPT_PATH' site-auto-watch run-once >/dev/null 2>&1 || true
      sleep '$SITE_AUTOWATCH_INTERVAL_SEC'
    done
  " >/dev/null 2>&1 &
  echo "site_auto_watch=started interval_sec=$SITE_AUTOWATCH_INTERVAL_SEC"
}

site_auto_watch_stop() {
  if [[ -f "$SITE_AUTOWATCH_PID_FILE" ]]; then
    local pid
    pid="$(cat "$SITE_AUTOWATCH_PID_FILE" 2>/dev/null || true)"
    if [[ -n "$pid" ]]; then
      kill "$pid" 2>/dev/null || true
    fi
    rm -f "$SITE_AUTOWATCH_PID_FILE"
  fi
  echo "site_auto_watch=stopped"
}

site_auto_watch_status() {
  if [[ -f "$SITE_AUTOWATCH_PID_FILE" ]]; then
    local pid
    pid="$(cat "$SITE_AUTOWATCH_PID_FILE" 2>/dev/null || true)"
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      echo "site_auto_watch=running pid=$pid interval_sec=$SITE_AUTOWATCH_INTERVAL_SEC"
      return 0
    fi
  fi
  echo "site_auto_watch=stopped"
}

site_list() {
  if [[ ! -f "$MANUAL_GATEWAY_DOMAINS_FILE" ]]; then
    echo "site_list_file=$MANUAL_GATEWAY_DOMAINS_FILE (missing)"
    return 0
  fi
  echo "site_list_file=$MANUAL_GATEWAY_DOMAINS_FILE"
  awk 'NF && $1 !~ /^#/{print}' "$MANUAL_GATEWAY_DOMAINS_FILE" | sed 's/[[:space:]]//g'
}

apps_running() {
  ps -eo comm= 2>/dev/null | tr -s ' ' | sed '/^$/d' | sort -u
}

services_running() {
  if command -v systemctl >/dev/null 2>&1; then
    systemctl --user list-units --type=service --state=running --no-legend --no-pager 2>/dev/null | awk '{print $1}'
  fi
}

app_route_add() {
  local app_id="${1:-}"
  shift || true
  local cmd="${*:-}"
  if [[ -z "$app_id" || -z "$cmd" ]]; then
    echo "error: app-route-add requires <app_id> <command>" >&2
    exit 1
  fi
  app_id="$(normalize_app_id "$app_id")"
  if [[ -z "$app_id" ]]; then
    echo "error: normalized app_id is empty" >&2
    exit 1
  fi
  upsert_route_line "$APP_ROUTES_FILE" "app:${app_id}" "$cmd"
  echo "app_route_added id=$app_id cmd=$cmd file=$APP_ROUTES_FILE"
}

app_route_add_running() {
  if [[ "$#" -eq 0 ]]; then
    echo "error: app-route-add-running requires at least one process_name" >&2
    exit 1
  fi
  local p id
  for p in "$@"; do
    p="$(trim_ascii "$p")"
    [[ -z "$p" ]] && continue
    id="$(normalize_app_id "$p")"
    [[ -z "$id" ]] && continue
    upsert_route_line "$APP_ROUTES_FILE" "app:${id}" "$p"
    echo "app_route_added_from_running id=$id cmd=$p"
  done
}

service_proxy_enable_running() {
  local running=()
  local svc
  while IFS= read -r svc; do
    [[ -z "$svc" ]] && continue
    running+=("$svc")
  done < <(services_running)

  if [[ "$#" -gt 0 ]]; then
    service_proxy_enable "$@"
    return 0
  fi
  if [[ "${#running[@]}" -eq 0 ]]; then
    echo "service-proxy-enable-running: no running user services"
    return 0
  fi
  service_proxy_enable "${running[@]}"
}

site_add() {
  if [[ "$#" -eq 0 ]]; then
    echo "error: site-add requires at least one domain" >&2
    exit 1
  fi
  mkdir -p "$(dirname "$MANUAL_GATEWAY_DOMAINS_FILE")"
  touch "$MANUAL_GATEWAY_DOMAINS_FILE"
  local d norm
  for d in "$@"; do
    norm="$(normalize_domain "$d")"
    [[ -z "$norm" ]] && continue
    if ! rg -n "^${norm}$" "$MANUAL_GATEWAY_DOMAINS_FILE" >/dev/null 2>&1; then
      echo "$norm" >> "$MANUAL_GATEWAY_DOMAINS_FILE"
      echo "site_added=$norm"
    fi
  done
  apply_route_mode
}

site_remove() {
  if [[ "$#" -eq 0 ]]; then
    echo "error: site-remove requires at least one domain" >&2
    exit 1
  fi
  [[ -f "$MANUAL_GATEWAY_DOMAINS_FILE" ]] || return 0
  local tmp d norm
  tmp="$(mktemp)"
  cp "$MANUAL_GATEWAY_DOMAINS_FILE" "$tmp"
  for d in "$@"; do
    norm="$(normalize_domain "$d")"
    [[ -z "$norm" ]] && continue
    sed -i "/^${norm}$/d" "$tmp"
    echo "site_removed=$norm"
  done
  mv "$tmp" "$MANUAL_GATEWAY_DOMAINS_FILE"
  apply_route_mode
}

start_socks_tunnel() {
  ensure_upstream_env_bootstrapped
  if [[ ! -f "$UPSTREAM_ENV_FILE" ]]; then
    return 0
  fi
  # shellcheck disable=SC1090
  source "$UPSTREAM_ENV_FILE"
  if [[ -z "${CHIMERA_UPSTREAM_USER:-}" || -z "${CHIMERA_UPSTREAM_HOST:-}" ]] || { [[ -z "${CHIMERA_UPSTREAM_PASS:-}" ]] && [[ ! -r "$UPSTREAM_SSH_KEY_FILE" ]]; }; then
    return 0
  fi
  local detected_port=""
  detected_port="$(find_matching_tunnel_port "${CHIMERA_UPSTREAM_HOST}" "${CHIMERA_UPSTREAM_USER}" || true)"
  if [[ -n "$detected_port" ]]; then
    SOCKS_PORT="$detected_port"
    upsert_env_kv "$UPSTREAM_ENV_FILE" "CHIMERA_SOCKS_PORT" "$SOCKS_PORT"
  fi
  if ss -ltn "( sport = :$SOCKS_PORT )" 2>/dev/null | grep -q "$SOCKS_HOST:$SOCKS_PORT"; then
    if proxy_healthcheck_ok; then
      build_pac_file
      apply_desktop_proxy auto
      return 0
    fi
    pkill -f "ssh .* -D $SOCKS_HOST:$SOCKS_PORT .*@.*" 2>/dev/null || true
  fi
  ensure_socks_port_isolated
  local endpoint
  while IFS= read -r endpoint; do
    [[ -z "$endpoint" ]] && continue
    if launch_socks_tunnel_for_endpoint "$endpoint"; then
      build_pac_file
      apply_desktop_proxy auto
      return 0
    fi
  done < <(build_upstream_candidates)
  # Tunnel didn't come up. In isolated mode we must not touch desktop proxy.
  if [[ "$CHIMERA_SYSTEM_INTEGRATION" == "1" ]]; then
    apply_desktop_proxy none
  fi
}

failover_upstream_gate_ok() {
  case "${CHIMERA_REQUIRE_UPSTREAM_FOR_FAILOVER,,}" in
    0|false|no|off) return 0 ;;
  esac
  refresh_socks_port_from_upstream_env
  if ss -ltn "( sport = :$SOCKS_PORT )" 2>/dev/null | grep -q "$SOCKS_HOST:$SOCKS_PORT"; then
    echo "failover_gate=ok source=listener host=$SOCKS_HOST port=$SOCKS_PORT"
    return 0
  fi
  if [[ -f "$UPSTREAM_ENV_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$UPSTREAM_ENV_FILE"
  fi
  if [[ -z "${CHIMERA_UPSTREAM_USER:-}" || -z "${CHIMERA_UPSTREAM_HOST:-}" ]] || { [[ -z "${CHIMERA_UPSTREAM_PASS:-}" ]] && [[ ! -r "$UPSTREAM_SSH_KEY_FILE" ]]; }; then
    echo "failover_gate=blocked reason=upstream_env_missing path=$UPSTREAM_ENV_FILE"
    return 1
  fi
  echo "failover_gate=ok source=upstream_env host=${CHIMERA_UPSTREAM_HOST} port=${CHIMERA_UPSTREAM_PORT:-22}"
  return 0
}

start_socks_watchdog() {
  if [[ ! -x "$WATCHDOG_SCRIPT" ]]; then
    return 0
  fi
  watchdog_pid_belongs() {
    local pid="${1:-}"
    [[ -n "$pid" ]] || return 1
    [[ -r "/proc/$pid/cmdline" ]] || return 1
    tr '\0' ' ' <"/proc/$pid/cmdline" | rg -q "chimera-socks-watchdog\\.sh"
  }
  if [[ -f "$WATCHDOG_PID_FILE" ]]; then
    local wpid
    wpid="$(cat "$WATCHDOG_PID_FILE" 2>/dev/null || true)"
    if [[ -n "$wpid" ]] && kill -0 "$wpid" 2>/dev/null; then
      if watchdog_pid_belongs "$wpid"; then
        return 0
      fi
      rm -f "$WATCHDOG_PID_FILE"
    fi
  fi
  ensure_socks_port_isolated
  export SOCKS_HOST SOCKS_PORT
  nohup "$WATCHDOG_SCRIPT" >/dev/null 2>&1 &
}

collect_proxy_domains() {
  local tmp out d dec
  tmp="$(mktemp)"
  if [[ -f "$MANUAL_GATEWAY_DOMAINS_FILE" ]]; then
    while IFS= read -r d; do
      d="${d%%#*}"
      d="$(normalize_domain "$d")"
      [[ -z "$d" ]] && continue
      [[ "$d" =~ ^[a-z0-9]([a-z0-9-]*\.)+[a-z0-9-]+$ ]] || continue
      printf '%s\n' "$d" >>"$tmp"
    done <"$MANUAL_GATEWAY_DOMAINS_FILE"
  fi
  if [[ -f "$SITE_ADAPTIVE_DB_FILE" ]]; then
    while IFS='|' read -r d dec _rest; do
      d="$(normalize_domain "$d")"
      [[ -z "$d" ]] && continue
      [[ "$d" =~ ^[a-z0-9]([a-z0-9-]*\.)+[a-z0-9-]+$ ]] || continue
      if [[ "$dec" == "proxy" ]]; then
        printf '%s\n' "$d" >>"$tmp"
      fi
    done <"$SITE_ADAPTIVE_DB_FILE"
  fi
  out="$(sort -u "$tmp" | sed '/^[[:space:]]*$/d')"
  rm -f "$tmp"
  printf '%s\n' "$out"
}

build_split_transparent_config() {
  mkdir -p "$(dirname "$SPLIT_TRANSPARENT_CONFIG_FILE")" "$(dirname "$SPLIT_TRANSPARENT_LOG_FILE")"
  local domains_json=""
  local domains_exact_json=""
  local auto_redirect_json="true"
  case "${SPLIT_TRANSPARENT_AUTO_REDIRECT,,}" in
    0|false|no|off) auto_redirect_json="false" ;;
  esac
  local d
  while IFS= read -r d; do
    d="$(trim_ascii "$d")"
    [[ -z "$d" ]] && continue
    if [[ -n "$domains_json" ]]; then
      domains_json+=","
    fi
    domains_json+="\"$d\""
    if [[ -n "$domains_exact_json" ]]; then
      domains_exact_json+=","
    fi
    domains_exact_json+="\"$d\""
  done < <(collect_proxy_domains)
  if [[ -z "$domains_json" ]]; then
    domains_json="\"www.youtube.com\",\"youtube.com\",\"chatgpt.com\""
    domains_exact_json="$domains_json"
  fi

  cat >"$SPLIT_TRANSPARENT_CONFIG_FILE" <<EOF
{
  "log": {
    "level": "$SPLIT_TRANSPARENT_LOG_LEVEL",
    "timestamp": true
  },
  "dns": {
    "servers": [
      { "tag": "local", "address": "local" }
    ],
    "final": "local",
    "strategy": "$SPLIT_TRANSPARENT_DNS_STRATEGY"
  },
  "inbounds": [
    {
      "type": "tun",
      "tag": "tun-in",
      "interface_name": "$SPLIT_TRANSPARENT_TUN_NAME",
      "address": ["$SPLIT_TRANSPARENT_TUN_ADDR", "$SPLIT_TRANSPARENT_TUN_ADDR6"],
      "auto_route": true,
      "auto_redirect": $auto_redirect_json,
      "strict_route": false,
      "route_exclude_address": [
        "127.0.0.0/8",
        "::1/128",
        "fc00::/7",
        "fe80::/10",
        "10.0.0.0/8",
        "172.16.0.0/12",
        "192.168.0.0/16"
      ],
      "sniff": true
    }
  ],
  "outbounds": [
    { "type": "direct", "tag": "direct" },
    {
      "type": "socks",
      "tag": "proxy",
      "server": "$SOCKS_HOST",
      "server_port": $SOCKS_PORT,
      "version": "5"
    }
  ],
  "route": {
    "auto_detect_interface": true,
    "rules": [
      { "ip_is_private": true, "outbound": "direct" },
      { "network": ["tcp"], "domain_suffix": [$domains_json], "outbound": "proxy" },
      { "network": ["tcp"], "domain": [$domains_exact_json], "outbound": "proxy" }
    ],
    "final": "direct"
  }
}
EOF
}

split_transparent_start() {
  if [[ "$SPLIT_TRANSPARENT_ENABLED" != "1" ]]; then
    echo "split_transparent=disabled"
    return 0
  fi
  if [[ ! -x "$SINGBOX_BIN" ]]; then
    if [[ -x "$RUNTIME_BOOTSTRAP_SCRIPT" ]]; then
      "$RUNTIME_BOOTSTRAP_SCRIPT" ensure-singbox >/dev/null 2>&1 || true
    fi
  fi
  if [[ ! -x "$SINGBOX_BIN" ]]; then
    echo "split_transparent=failed reason=singbox_unavailable_after_bootstrap bin=$SINGBOX_BIN"
    return 1
  fi
  if ! failover_upstream_gate_ok; then
    echo "split_transparent=failed reason=failover_gate_blocked"
    return 1
  fi
  start_socks_tunnel
  if ! ss -ltn "( sport = :$SOCKS_PORT )" 2>/dev/null | grep -q "$SOCKS_HOST:$SOCKS_PORT"; then
    echo "split_transparent=failed reason=socks_not_running"
    return 1
  fi
  build_split_transparent_config
  if ! sudo -n "$SINGBOX_BIN" check -c "$SPLIT_TRANSPARENT_CONFIG_FILE" >/dev/null 2>&1; then
    echo "split_transparent=failed reason=config_check_failed"
    return 1
  fi
  split_transparent_stop >/dev/null 2>&1 || true
  sudo -n nohup "$SINGBOX_BIN" run -c "$SPLIT_TRANSPARENT_CONFIG_FILE" >"$SPLIT_TRANSPARENT_LOG_FILE" 2>&1 &
  sleep 2
  local spid=""
  spid="$(split_transparent_find_pid || true)"
  if [[ -n "$spid" ]]; then
    mkdir -p "$(dirname "$SPLIT_TRANSPARENT_PID_FILE")"
    printf '%s\n' "$spid" >"$SPLIT_TRANSPARENT_PID_FILE"
    echo "split_transparent=running pid=$spid tun=$SPLIT_TRANSPARENT_TUN_NAME config=$SPLIT_TRANSPARENT_CONFIG_FILE"
  else
    echo "split_transparent=failed reason=process_not_alive"
    return 1
  fi
}

split_transparent_stop() {
  local spid=""
  if [[ -f "$SPLIT_TRANSPARENT_PID_FILE" ]]; then
    spid="$(cat "$SPLIT_TRANSPARENT_PID_FILE" 2>/dev/null || true)"
  fi
  if [[ -n "$spid" ]]; then
    sudo -n kill "$spid" 2>/dev/null || true
  fi
  sudo -n pkill -f "sing-box run -c $SPLIT_TRANSPARENT_CONFIG_FILE" 2>/dev/null || true
  rm -f "$SPLIT_TRANSPARENT_PID_FILE"
  echo "split_transparent=stopped"
}

split_transparent_status() {
  local running="false" spid=""
  if [[ -f "$SPLIT_TRANSPARENT_PID_FILE" ]]; then
    spid="$(cat "$SPLIT_TRANSPARENT_PID_FILE" 2>/dev/null || true)"
    if [[ -n "$spid" ]] && sudo -n test -e "/proc/$spid" && pid_cmdline_contains "$spid" "$SPLIT_TRANSPARENT_CONFIG_FILE"; then
      running="true"
    fi
  fi
  if [[ "$running" != "true" ]]; then
    spid="$(split_transparent_find_pid || true)"
    if [[ -n "$spid" ]] && sudo -n test -e "/proc/$spid" && pid_cmdline_contains "$spid" "$SPLIT_TRANSPARENT_CONFIG_FILE"; then
      running="true"
      printf '%s\n' "$spid" >"$SPLIT_TRANSPARENT_PID_FILE"
    fi
  fi
  echo "split_transparent_running=$running"
  echo "split_transparent_pid=${spid:-none}"
  echo "split_transparent_tun=$SPLIT_TRANSPARENT_TUN_NAME"
  echo "split_transparent_config=$SPLIT_TRANSPARENT_CONFIG_FILE"
  echo "split_transparent_log=$SPLIT_TRANSPARENT_LOG_FILE"
}

split_transparent_find_pid() {
  pgrep -af "sing-box run -c $SPLIT_TRANSPARENT_CONFIG_FILE" 2>/dev/null \
    | awk '!/chimera-control.sh|rg -n|pgrep -af/ {print $1; exit}'
}

ensure_split_transparent_running() {
  [[ "$SPLIT_TRANSPARENT_ENABLED" == "1" ]] || return 0
  split_transparent_start || true
  local pid=""
  pid="$(split_transparent_find_pid || true)"
  if [[ -z "$pid" ]]; then
    sleep 2
    split_transparent_start || true
    pid="$(split_transparent_find_pid || true)"
  fi
  if [[ -z "$pid" ]]; then
    echo "split_transparent=failed reason=not_running_after_retry"
    return 1
  fi
  printf '%s\n' "$pid" >"$SPLIT_TRANSPARENT_PID_FILE"
  echo "split_transparent=ready pid=$pid"
  return 0
}

start_split_transparent_watchdog() {
  [[ "$SPLIT_TRANSPARENT_ENABLED" == "1" ]] || return 0
  if [[ -f "$SPLIT_TRANSPARENT_WATCHDOG_PID_FILE" ]]; then
    local old_pid=""
    old_pid="$(cat "$SPLIT_TRANSPARENT_WATCHDOG_PID_FILE" 2>/dev/null || true)"
    if [[ -n "$old_pid" ]] && kill -0 "$old_pid" 2>/dev/null; then
      return 0
    fi
  fi
  (
    while true; do
      if [[ ! -f "$SPLIT_TRANSPARENT_PID_FILE" ]] || [[ "$("$SCRIPT_PATH" split-transparent status | awk -F= '/split_transparent_running/{print $2}')" != "true" ]]; then
        "$SCRIPT_PATH" split-transparent start >/dev/null 2>&1 || true
        if [[ "$("$SCRIPT_PATH" split-transparent status | awk -F= '/split_transparent_running/{print $2}')" != "true" ]]; then
          echo "chimera_split_watchdog=status=degraded reason=split_restart_failed" >>"$SPLIT_TRANSPARENT_LOG_FILE"
        fi
      fi
      sleep 8
    done
  ) >/dev/null 2>&1 &
  printf '%s\n' "$!" >"$SPLIT_TRANSPARENT_WATCHDOG_PID_FILE"
}

stop_split_transparent_watchdog() {
  if [[ -f "$SPLIT_TRANSPARENT_WATCHDOG_PID_FILE" ]]; then
    local pid=""
    pid="$(cat "$SPLIT_TRANSPARENT_WATCHDOG_PID_FILE" 2>/dev/null || true)"
    if [[ -n "$pid" ]]; then
      kill "$pid" 2>/dev/null || true
    fi
    rm -f "$SPLIT_TRANSPARENT_WATCHDOG_PID_FILE"
  fi
}

restart_chromium_with_pac() {
  if [[ "$CHIMERA_SYSTEM_INTEGRATION" != "1" ]]; then
    return 0
  fi
  if [[ "$AUTO_RESTART_CHROMIUM" != "1" ]]; then
    return 0
  fi
  local chromium_bin="/usr/lib64/chromium/chromium"
  if [[ ! -x "$chromium_bin" ]]; then
    return 0
  fi
  if ! pgrep -f "$chromium_bin" >/dev/null 2>&1; then
    return 0
  fi
  pkill -f "$chromium_bin" 2>/dev/null || true
  sleep 1
  DISPLAY="${DISPLAY:-:0}" WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}" XDG_SESSION_TYPE="${XDG_SESSION_TYPE:-wayland}" \
    nohup "$chromium_bin" \
      --ozone-platform=wayland \
      --proxy-pac-url="file://$PAC_FILE" \
      --no-default-browser-check >/dev/null 2>&1 &
}

detect_browser_bin() {
  local bins=(
    /usr/bin/google-chrome-stable
    /usr/bin/google-chrome
    /usr/bin/chromium-browser
    /usr/bin/chromium
    /usr/bin/brave-browser
    /usr/bin/yandex-browser
    /usr/lib64/chromium/chromium
  )
  local b
  for b in "${bins[@]}"; do
    if [[ -x "$b" ]]; then
      printf '%s\n' "$b"
      return 0
    fi
  done
  return 1
}

restart_browser_with_split_proxy() {
  if [[ "$CHIMERA_SYSTEM_INTEGRATION" != "1" ]]; then
    return 0
  fi
  if [[ "$AUTO_RESTART_CHROMIUM" != "1" ]]; then
    return 0
  fi
  local browser_bin=""
  browser_bin="$(detect_browser_bin || true)"
  if [[ -z "$browser_bin" ]]; then
    return 0
  fi
  local pname
  pname="$(basename "$browser_bin")"
  if ! pgrep -x "$pname" >/dev/null 2>&1 && ! pgrep -f "$browser_bin" >/dev/null 2>&1; then
    return 0
  fi
  pkill -x "$pname" 2>/dev/null || true
  pkill -f "$browser_bin" 2>/dev/null || true
  sleep 1
  DISPLAY="${DISPLAY:-:0}" WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}" XDG_SESSION_TYPE="${XDG_SESSION_TYPE:-wayland}" \
    nohup "$browser_bin" \
      --proxy-pac-url="file://$PAC_FILE" \
      --proxy-bypass-list="<-loopback>" \
      --disable-quic \
      --disable-features=UseDnsHttpsSvcb,AsyncDns,DnsHttpssvc \
      --no-default-browser-check >/dev/null 2>&1 &
}

stop_socks_tunnel() {
  if [[ -f "$WATCHDOG_PID_FILE" ]]; then
    wpid="$(cat "$WATCHDOG_PID_FILE" 2>/dev/null || true)"
    if [[ -n "${wpid:-}" ]]; then
      kill "$wpid" 2>/dev/null || true
    fi
    rm -f "$WATCHDOG_PID_FILE"
  fi
  kill_owned_socks_tunnel
  if [[ "$CHIMERA_ALLOW_PGREP_KILL" == "1" ]] && ! is_protected_port "$SOCKS_PORT"; then
    pkill -f "ssh .* -D $SOCKS_HOST:$SOCKS_PORT .*@.*" 2>/dev/null || true
  fi
  if [[ "$CHIMERA_SYSTEM_INTEGRATION" == "1" ]]; then
    apply_desktop_proxy none
  fi
}

uninstall_full() {
  ensure_base_path

  # 1) Stop runtime pieces (best-effort).
  site_auto_watch_stop || true
  stop_socks_tunnel || true
  if command -v systemctl >/dev/null 2>&1; then
    systemctl --user disable --now chimera-client.service chimera-gateway.service 2>/dev/null || true
  fi

  # 2) Rollback runtime state if CLI state file exists.
  if [[ -f "$STATE_FILE" ]]; then
    (
      run_chimera_cli rollback recover --state-file "$STATE_FILE" >/dev/null 2>&1 || true
      run_chimera_cli down --state-file "$STATE_FILE" >/dev/null 2>&1 || true
    ) || true
  fi

  # 3) Remove proxy overrides for configured services.
  service_proxy_disable >/dev/null 2>&1 || true
  if command -v systemctl >/dev/null 2>&1; then
    rm -rf "${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/"*.service.d 2>/dev/null || true
    systemctl --user daemon-reload 2>/dev/null || true
  fi

  # 4) Reset desktop proxy settings.
  if [[ "$CHIMERA_SYSTEM_INTEGRATION" == "1" ]] && desktop_proxy_supported; then
    gsettings set org.gnome.system.proxy mode 'none' 2>/dev/null || true
    gsettings set org.gnome.system.proxy autoconfig-url '' 2>/dev/null || true
  fi

  # 5) Remove installed user units/desktop entries.
  local systemd_user_dir="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
  local applications_dir="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
  rm -f "$systemd_user_dir/chimera-gateway.service" "$systemd_user_dir/chimera-client.service" 2>/dev/null || true
  rm -f "$applications_dir/chimera-control-gui.desktop" "$applications_dir/chimera-control.desktop" 2>/dev/null || true
  if command -v systemctl >/dev/null 2>&1; then
    systemctl --user daemon-reload 2>/dev/null || true
  fi

  # 6) Remove CHIMERA user state/cache settings.
  rm -f "$PAC_FILE" "$UI_MODE_FILE" "$ROUTE_MODE_FILE" "$SPLIT_LIST_MODE_FILE" 2>/dev/null || true
  rm -f "$WATCHDOG_PID_FILE" "$SITE_AUTOWATCH_PID_FILE" "$LAST_ENDPOINT_FILE" "$UPSTREAM_HEALTH_STATE_FILE" 2>/dev/null || true
  rm -f "$SITE_ADAPTIVE_DB_FILE" 2>/dev/null || true
  rm -rf "${XDG_CONFIG_HOME:-$HOME/.config}/chimera" 2>/dev/null || true
  rm -rf "${XDG_CACHE_HOME:-$HOME/.cache}/chimera" 2>/dev/null || true

  # 7) Remove local runtime artifacts created by control/runtime checks.
  rm -f "$STATE_FILE" "$ROOT_DIR/docs/runtime_state_latest.json" 2>/dev/null || true
  rm -f "$ROOT_DIR/docs/CHIMERA_PATH_PROOF.json" "$ROOT_DIR/docs/CHIMERA_CHANNEL_AUDIT.json" "$ROOT_DIR/docs/CHIMERA_E2E_CHANNEL_GATE.json" 2>/dev/null || true
  rm -f "$ROOT_DIR/docs/CHIMERA_LOAD_GATE_LAPTOP.json" "$ROOT_DIR/docs/CHIMERA_FRESH_GATE_REPORT.json" 2>/dev/null || true

  echo "uninstall_status=ok"
}

cmd="${1:-}"
declare -a APP_ROUTE_IDS=()
declare -a SERVICE_ROUTE_IDS=()
declare -A APP_ROUTE_CMDS=()
declare -A SERVICE_ROUTE_NAMES=()
declare -A APP_ROUTE_ENV=()
declare -A SERVICE_ROUTE_ENV=()
case "$cmd" in
  start)
    ensure_safe_local_host_guard
    ensure_base_path
    ensure_vpn_coexist_guard
    client_expected="1"
    if ! client_config_ready; then
      client_expected="0"
    fi
    if systemd_user_ready && systemd_chimera_units_present; then
      systemctl --user daemon-reload
      systemctl --user enable chimera-gateway.service
      if [[ "$client_expected" == "1" ]]; then
        systemctl --user enable chimera-client.service
      else
        systemctl --user disable --now chimera-client.service >/dev/null 2>&1 || true
      fi
      if [[ "$client_expected" == "1" ]]; then
        if ! systemctl --user restart chimera-gateway.service chimera-client.service; then
          sleep 1
          systemctl --user start chimera-gateway.service chimera-client.service || true
        fi
      else
        if ! systemctl --user restart chimera-gateway.service; then
          sleep 1
          systemctl --user start chimera-gateway.service || true
        fi
      fi
      if ! systemd_units_active_ok "$client_expected"; then
        echo "error: chimera user services failed to become active" >&2
        if [[ "$client_expected" == "1" ]]; then
          systemctl --user --no-pager --full status chimera-gateway.service chimera-client.service || true
        else
          systemctl --user --no-pager --full status chimera-gateway.service || true
        fi
        exit 1
      fi
    else
      (
        cd "$ROOT_DIR"
        if systemd_user_ready && ! systemd_chimera_units_present; then
          echo "chimera-control: systemd user detected, but CHIMERA units are missing; using direct-binary mode"
        fi
        run_chimera_gateway doctor --config configs/gateway.example.conf --json --out docs/gateway_doctor_latest.json
        if [[ "$client_expected" == "1" ]]; then
          run_chimera_cli up --config "$(client_config_path)" --state-file docs/runtime_state_latest.json
        else
          echo "client_config_carrier_addr=skipped reason=missing_endpoint"
        fi
      )
    fi
    if [[ -x "$AUTOFIX_SCRIPT" ]]; then
      mkdir -p "$(dirname "$AUTOFIX_LOG_FILE")"
      if command -v timeout >/dev/null 2>&1; then
        timeout "$AUTOFIX_TIMEOUT" "$AUTOFIX_SCRIPT" >>"$AUTOFIX_LOG_FILE" 2>&1 || true
      else
        "$AUTOFIX_SCRIPT" >>"$AUTOFIX_LOG_FILE" 2>&1 || true
      fi
    fi
    start_socks_tunnel
    apply_route_mode
    auto_sync_desktop_proxy_port
    force_desktop_proxy_none
    start_socks_watchdog
    if [[ "$SITE_AUTOWATCH_ENABLED" == "1" ]]; then
      site_auto_watch_start
    fi
    if [[ "$SPLIT_TRANSPARENT_ENABLED" == "1" && "$client_expected" == "1" ]]; then
      if ! ensure_split_transparent_running; then
        if [[ "$CHIMERA_STRICT_FAILOVER_GATE" == "1" ]]; then
          echo "error: split transparent/failover gate failed; refusing partial runtime start" >&2
          exit 1
        fi
      fi
      start_split_transparent_watchdog || true
    elif [[ "$SPLIT_TRANSPARENT_ENABLED" == "1" ]]; then
      echo "split_transparent=skipped reason=missing_endpoint"
    fi
    restart_chromium_with_pac
    restart_browser_with_split_proxy
    ;;
  stop)
    ensure_safe_local_host_guard
    ensure_base_path
    if systemd_user_ready; then
      systemctl --user disable --now chimera-client.service chimera-gateway.service || true
    else
      (
        cd "$ROOT_DIR"
        run_chimera_cli down --config "$(client_config_path)" --state-file docs/runtime_state_latest.json || true
      )
    fi
    site_auto_watch_stop
    stop_split_transparent_watchdog || true
    split_transparent_stop || true
    stop_socks_tunnel
    ;;
  restart)
    ensure_safe_local_host_guard
    ensure_base_path
    ensure_vpn_coexist_guard
    client_expected="1"
    if ! client_config_ready; then
      client_expected="0"
    fi
    if systemd_user_ready && systemd_chimera_units_present; then
      systemctl --user daemon-reload
      if [[ "$client_expected" == "1" ]]; then
        systemctl --user restart chimera-gateway.service chimera-client.service
      else
        systemctl --user disable --now chimera-client.service >/dev/null 2>&1 || true
        systemctl --user restart chimera-gateway.service
      fi
    else
      (
        cd "$ROOT_DIR"
        if systemd_user_ready && ! systemd_chimera_units_present; then
          echo "chimera-control: systemd user detected, but CHIMERA units are missing; using direct-binary mode"
        fi
        run_chimera_cli down --config "$(client_config_path)" --state-file docs/runtime_state_latest.json || true
        run_chimera_gateway doctor --config configs/gateway.example.conf --json --out docs/gateway_doctor_latest.json
        if [[ "$client_expected" == "1" ]]; then
          run_chimera_cli up --config "$(client_config_path)" --state-file docs/runtime_state_latest.json
        else
          echo "client_config_carrier_addr=skipped reason=missing_endpoint"
        fi
      )
    fi
    start_socks_tunnel
    apply_route_mode
    auto_sync_desktop_proxy_port
    force_desktop_proxy_none
    if [[ "$SPLIT_TRANSPARENT_ENABLED" == "1" && "$client_expected" == "1" ]]; then
      if ! ensure_split_transparent_running; then
        if [[ "$CHIMERA_STRICT_FAILOVER_GATE" == "1" ]]; then
          echo "error: split transparent/failover gate failed; refusing partial runtime restart" >&2
          exit 1
        fi
      fi
      start_split_transparent_watchdog || true
    elif [[ "$SPLIT_TRANSPARENT_ENABLED" == "1" ]]; then
      echo "split_transparent=skipped reason=missing_endpoint"
    fi
    restart_browser_with_split_proxy
    ;;
  status)
    ensure_base_path
    if systemd_user_ready; then
      if client_config_ready; then
        systemctl --user --no-pager --full status chimera-gateway.service chimera-client.service || true
      else
        systemctl --user --no-pager --full status chimera-gateway.service || true
      fi
    else
      echo "systemd_user=unavailable (direct-binary mode)"
    fi
    if [[ -f "$STATE_FILE" ]]; then
      echo
      echo "Runtime state file: $STATE_FILE"
      ls -l "$STATE_FILE"
    else
      if client_config_ready; then
        echo
        echo "Runtime state file is missing: $STATE_FILE"
      fi
    fi
    ;;
  doctor)
    ensure_base_path
    run_chimera_cli doctor --config configs/client.example.conf --json --out docs/doctor_latest.json
    ;;
  logs)
    echo "Gateway log: $GATEWAY_LOG"
    tail -n 80 "$GATEWAY_LOG" 2>/dev/null || true
    echo
    echo "Client log: $CLIENT_LOG"
    tail -n 80 "$CLIENT_LOG" 2>/dev/null || true
    ;;
  proxy-status)
    refresh_socks_port_from_upstream_env
    auto_sync_desktop_proxy_port
    if ss -ltn "( sport = :$SOCKS_PORT )" 2>/dev/null | grep -q "$SOCKS_HOST:$SOCKS_PORT"; then
      echo "socks_tunnel=running $SOCKS_HOST:$SOCKS_PORT"
    else
      echo "socks_tunnel=stopped"
    fi
    echo "upstream_strategy=$UPSTREAM_STRATEGY"
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
    if [[ "$CHIMERA_SYSTEM_INTEGRATION" == "1" ]] && desktop_proxy_supported; then
      echo "proxy_mode=$(gsettings get org.gnome.system.proxy mode)"
      echo "proxy_autoconfig_url=$(gsettings get org.gnome.system.proxy autoconfig-url)"
      echo "proxy_socks_host=$(gsettings get org.gnome.system.proxy.socks host)"
      echo "proxy_socks_port=$(gsettings get org.gnome.system.proxy.socks port)"
    else
      echo "proxy_mode=isolated (system integration disabled)"
      if [[ "$CHIMERA_COEXIST_TRANSPARENT_CAPTURE" == "1" ]]; then
        echo "coexist_transparent_capture=enabled"
        echo "coexist_capture_mode=kernel_tun_split"
      fi
    fi
    ;;
  app-routes-status)
    print_app_routes_status
    ;;
  route-status)
    print_route_status
    ;;
  route-mode)
    mode="${2:-show}"
    case "$mode" in
      show)
        echo "route_mode=$(read_route_mode)"
        ;;
      full|split|selective|off)
        write_route_mode "$mode"
        apply_route_mode
        auto_sync_desktop_proxy_port
        restart_browser_with_split_proxy
        echo "route_mode_set=$(read_route_mode)"
        ;;
      *)
        echo "error: route-mode must be show|full|split|off" >&2
        exit 1
        ;;
    esac
    ;;
  split-list-mode)
    mode="${2:-show}"
    case "$mode" in
      show)
        echo "split_list_mode=$(read_split_list_mode)"
        ;;
      allow|deny)
        write_split_list_mode "$mode"
        apply_route_mode
        restart_browser_with_split_proxy
        echo "split_list_mode_set=$(read_split_list_mode)"
        ;;
      *)
        echo "error: split-list-mode must be show|allow|deny" >&2
        exit 1
        ;;
    esac
    ;;
  site-add)
    shift || true
    site_add "$@"
    ;;
  site-remove)
    shift || true
    site_remove "$@"
    ;;
  site-list)
    site_list
    ;;
  site-auto-resolve)
    shift || true
    site_auto_resolve_many "$@"
    ;;
  site-auto-status)
    site_adaptive_status
    ;;
  site-auto-bootstrap)
    site_auto_bootstrap
    ;;
  site-auto-discover)
    sub="${2:-run}"
    case "$sub" in
      run) site_auto_discover_run ;;
      status) site_discovery_status ;;
      clear) site_discovery_clear ;;
      *)
        echo "error: site-auto-discover must be run|status|clear" >&2
        exit 1
        ;;
    esac
    ;;
  site-auto-watch)
    sub="${2:-status}"
    case "$sub" in
      start) site_auto_watch_start ;;
      stop) site_auto_watch_stop ;;
      status) site_auto_watch_status ;;
      run-once) site_auto_watch_run_once ;;
      *)
        echo "error: site-auto-watch must be start|stop|status|run-once" >&2
        exit 1
        ;;
    esac
    ;;
  split-transparent)
    sub="${2:-status}"
    case "$sub" in
      start) split_transparent_start ;;
      stop) split_transparent_stop ;;
      status) split_transparent_status ;;
      refresh)
        build_split_transparent_config
        split_transparent_start
        ;;
      *)
        echo "error: split-transparent must be start|stop|status|refresh" >&2
        exit 1
        ;;
    esac
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
    lines="${2:-30}"
    upstream_audit "$lines"
    ;;
  upstream-failover-smoke)
    wait_sec="${2:-10}"
    upstream_failover_smoke "$wait_sec"
    ;;
  apps-running)
    apps_running
    ;;
  services-running)
    services_running
    ;;
  app-route-add)
    shift || true
    app_id="${1:-}"
    shift || true
    app_route_add "$app_id" "$@"
    ;;
  app-route-add-running)
    shift || true
    app_route_add_running "$@"
    ;;
  service-proxy-enable-running)
    shift || true
    service_proxy_enable_running "$@"
    ;;
  uninstall)
    ensure_safe_local_host_guard
    uninstall_full
    ;;
  run-app)
    shift || true
    run_app_via_proxy "$@"
    ;;
  verify-app)
    shift || true
    verify_app_via_proxy "$@"
    ;;
  verify-cmd)
    shift || true
    verify_cmd_via_proxy "$@"
    ;;
  service-proxy-enable)
    shift || true
    service_proxy_enable "$@"
    ;;
  service-proxy-disable)
    shift || true
    service_proxy_disable "$@"
    ;;
  mesh)
    shift || true
    run_chimera_cli mesh "$@"
    ;;
  verify-service)
    shift || true
    verify_service_proxy "$@"
    ;;
  ui-mode)
    mode="${2:-show}"
    case "$mode" in
      show)
        if [[ -f "$UI_MODE_FILE" ]]; then
          echo "ui_mode=$(cat "$UI_MODE_FILE")"
        else
          echo "ui_mode=auto"
        fi
        ;;
      auto|tray|dialog|cli)
        mkdir -p "$(dirname "$UI_MODE_FILE")"
        printf '%s\n' "$mode" > "$UI_MODE_FILE"
        echo "ui_mode set to: $mode"
        ;;
      *)
        echo "error: ui-mode must be one of auto|tray|dialog|cli|show" >&2
        exit 1
        ;;
    esac
    ;;
  *)
    usage
    exit 1
    ;;
esac
