#!/usr/bin/env bash
set -euo pipefail

UPSTREAM_ENV_FILE="${UPSTREAM_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/upstream_proxy.env}"
SOCKS_HOST="${SOCKS_HOST:-127.0.0.1}"
SOCKS_PORT="${SOCKS_PORT:-11080}"
SOCKS_HEALTHCHECK_URL="${SOCKS_HEALTHCHECK_URL:-https://api.ipify.org}"
SOCKS_HEALTHCHECK_TIMEOUT_SEC="${SOCKS_HEALTHCHECK_TIMEOUT_SEC:-6}"
PID_FILE="${PID_FILE:-${XDG_RUNTIME_DIR:-/tmp}/chimera-socks-watchdog.pid}"
LOG_FILE="${LOG_FILE:-${XDG_CACHE_HOME:-$HOME/.cache}/chimera/socks-watchdog.log}"
UPSTREAM_STRATEGY="${CHIMERA_UPSTREAM_STRATEGY:-balanced}"
UPSTREAM_PROBE_TIMEOUT_SEC="${CHIMERA_UPSTREAM_PROBE_TIMEOUT_SEC:-2}"
UPSTREAM_STICKY_SEC="${CHIMERA_UPSTREAM_STICKY_SEC:-120}"
UPSTREAM_DEGRADE_FAILS="${CHIMERA_UPSTREAM_DEGRADE_FAILS:-3}"
LAST_ENDPOINT_FILE="${XDG_CACHE_HOME:-$HOME/.cache}/chimera/last_upstream_endpoint"
HEALTH_STATE_FILE="${XDG_CACHE_HOME:-$HOME/.cache}/chimera/upstream_health_state"
WATCHDOG_INTERVAL_SEC="${CHIMERA_WATCHDOG_INTERVAL_SEC:-5}"
WATCHDOG_INTERVAL_DEGRADED_SEC="${CHIMERA_WATCHDOG_INTERVAL_DEGRADED_SEC:-1}"
FAILOVER_SILENT="${CHIMERA_FAILOVER_SILENT:-1}"
last_transport="unknown"

mkdir -p "$(dirname "$LOG_FILE")"

pid_belongs_to_watchdog() {
  local pid="${1:-}"
  [[ -n "$pid" ]] || return 1
  [[ -r "/proc/$pid/cmdline" ]] || return 1
  tr '\0' ' ' <"/proc/$pid/cmdline" | grep -q "chimera-socks-watchdog\\.sh"
}

if [[ -f "$PID_FILE" ]]; then
  old_pid="$(cat "$PID_FILE" 2>/dev/null || true)"
  if [[ -n "$old_pid" ]] && kill -0 "$old_pid" 2>/dev/null; then
    if pid_belongs_to_watchdog "$old_pid"; then
      exit 0
    fi
    rm -f "$PID_FILE"
  fi
fi
echo "$$" >"$PID_FILE"
trap 'rm -f "$PID_FILE"' EXIT
degrade_fails=0
echo "$(date '+%F %T') watchdog start pid=$$ port=$SOCKS_PORT strategy=$UPSTREAM_STRATEGY" >>"$LOG_FILE"

write_health_state() {
  local listener_up="${1:-false}"
  local health_ok="${2:-false}"
  local reason="${3:-none}"
  local now
  now="$(date +%s)"
  mkdir -p "$(dirname "$HEALTH_STATE_FILE")"
  cat >"$HEALTH_STATE_FILE" <<EOF
ts=$now
listener_up=$listener_up
health_ok=$health_ok
degrade_fails=$degrade_fails
degrade_threshold=$UPSTREAM_DEGRADE_FAILS
strategy=$UPSTREAM_STRATEGY
last_reason=$reason
last_transport=$last_transport
EOF
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
    out+=("${CHIMERA_UPSTREAM_HOST}:${CHIMERA_UPSTREAM_PORT:-22}")
  fi
  printf '%s\n' "${out[@]}"
}

count_upstream_candidates() {
  local n=0 _
  while IFS= read -r _; do
    [[ -z "$_" ]] && continue
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

endpoint_latency_ms() {
  local parsed endpoint
  parsed="$(parse_transport_endpoint "${1:-}" || true)"
  endpoint="${parsed#*|}"
  local host="${endpoint%:*}"
  local port="${endpoint##*:}"
  [[ -z "$host" || -z "$port" ]] && echo 2147483647 && return 0
  local start end dur
  start="$(date +%s 2>/dev/null || echo 0)"
  if timeout "$UPSTREAM_PROBE_TIMEOUT_SEC" bash -lc "</dev/tcp/$host/$port" >/dev/null 2>&1; then
    end="$(date +%s 2>/dev/null || echo 0)"
    if [[ "$start" =~ ^[0-9]+$ && "$end" =~ ^[0-9]+$ && "$end" -ge "$start" ]]; then
      # Second-level timing is enough for ranking; keep ms units for consistency.
      dur=$(((end - start) * 1000))
      echo "$dur"
    else
      echo 1
    fi
  else
    echo 2147483647
  fi
}

select_best_endpoint() {
  local candidates=()
  local endpoint
  while IFS= read -r endpoint; do
    [[ -z "$endpoint" ]] && continue
    candidates+=("$endpoint")
  done < <(build_upstream_candidates)

  if [[ "${#candidates[@]}" -eq 0 ]]; then
    return 1
  fi

  local sticky_until=0 now=0 last=""
  now="$(date +%s)"
  if [[ -f "$LAST_ENDPOINT_FILE" ]]; then
    last="$(awk -F'|' 'NR==1{print $1}' "$LAST_ENDPOINT_FILE" 2>/dev/null || true)"
    sticky_until="$(awk -F'|' 'NR==1{print $2}' "$LAST_ENDPOINT_FILE" 2>/dev/null || echo 0)"
    if [[ -n "$last" && "$sticky_until" =~ ^[0-9]+$ && "$sticky_until" -gt "$now" ]]; then
      printf '%s\n' "$last"
      return 0
    fi
  fi

  local best="" best_score=2147483647 score lat
  local best_transport="ssh"
  local probe_trace=""
  for endpoint in "${candidates[@]}"; do
    local parsed transport endpoint_only
    parsed="$(parse_transport_endpoint "$endpoint" || true)"
    transport="${parsed%%|*}"
    endpoint_only="${parsed#*|}"
    lat="$(endpoint_latency_ms "$endpoint")"
    probe_trace+="${transport}@${endpoint_only}:${lat},"
    score="$lat"
    case "$UPSTREAM_STRATEGY" in
      resilience)
        score="$lat"
        ;;
      balanced|throughput|*)
        score="$lat"
        ;;
    esac
    if [[ "$score" =~ ^[0-9]+$ ]] && [[ "$score" -lt "$best_score" ]]; then
      best_score="$score"
      best="$endpoint_only"
      best_transport="${transport:-ssh}"
    fi
  done

  if [[ -z "$best" ]]; then
    local parsed
    parsed="$(parse_transport_endpoint "${candidates[0]}" || true)"
    best="${parsed#*|}"
    best_transport="${parsed%%|*}"
  fi
  mkdir -p "$(dirname "$LAST_ENDPOINT_FILE")"
  printf '%s|%s|%s\n' "$best" "$((now + UPSTREAM_STICKY_SEC))" "$best_transport" >"$LAST_ENDPOINT_FILE"
  echo "$(date '+%F %T') endpoint_probe strategy=${UPSTREAM_STRATEGY} probes=${probe_trace%?} best=${best} best_transport=${best_transport} score=${best_score}" >>"$LOG_FILE"
  printf '%s@%s\n' "$best_transport" "$best"
}

launch_for_endpoint() {
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
    *) return 1 ;;
  esac
  last_transport="${transport:-ssh}"
  SSHPASS="$CHIMERA_UPSTREAM_PASS" nohup sshpass -e ssh \
    -o StrictHostKeyChecking=no \
    -o ExitOnForwardFailure=yes \
    -o ServerAliveInterval=30 \
    -o ServerAliveCountMax=3 \
    -N -D "$SOCKS_HOST:$SOCKS_PORT" \
    -p "$port" \
    "$CHIMERA_UPSTREAM_USER@$host" >>"$LOG_FILE" 2>&1 &
  local ssh_pid=$!
  for _ in 1 2 3 4 5 6; do
    sleep 1
    if ss -ltn "( sport = :$SOCKS_PORT )" 2>/dev/null | grep -q "$SOCKS_HOST:$SOCKS_PORT"; then
      if proxy_healthcheck_ok; then
        return 0
      fi
      kill "$ssh_pid" 2>/dev/null || true
      pkill -f "ssh .* -D $SOCKS_HOST:$SOCKS_PORT .*@.*" 2>/dev/null || true
      return 1
    fi
  done
  kill "$ssh_pid" 2>/dev/null || true
  return 1
}

while true; do
  loop_sleep="$WATCHDOG_INTERVAL_SEC"
  if [[ -f "$UPSTREAM_ENV_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$UPSTREAM_ENV_FILE"
    if [[ -n "${CHIMERA_SOCKS_PORT:-}" ]] && [[ "${CHIMERA_SOCKS_PORT}" =~ ^[0-9]+$ ]]; then
      SOCKS_PORT="$CHIMERA_SOCKS_PORT"
    fi
    if [[ -n "${CHIMERA_UPSTREAM_USER:-}" && -n "${CHIMERA_UPSTREAM_HOST:-}" && -n "${CHIMERA_UPSTREAM_PASS:-}" ]]; then
      listener_up=false
      if ss -ltn "( sport = :$SOCKS_PORT )" 2>/dev/null | grep -q "$SOCKS_HOST:$SOCKS_PORT"; then
        listener_up=true
      fi
      health_ok=false
      if proxy_healthcheck_ok; then
        health_ok=true
      fi

      if [[ "$listener_up" == true && "$health_ok" == false ]]; then
        degrade_fails=$((degrade_fails + 1))
        loop_sleep="$WATCHDOG_INTERVAL_DEGRADED_SEC"
      elif [[ "$listener_up" == true && "$health_ok" == true ]]; then
        degrade_fails=0
      fi
      write_health_state "$listener_up" "$health_ok" "steady"

      if [[ "$listener_up" == false || "$health_ok" == false && "$degrade_fails" -ge "$UPSTREAM_DEGRADE_FAILS" ]]; then
        loop_sleep="$WATCHDOG_INTERVAL_DEGRADED_SEC"
        pkill -f "ssh .* -D $SOCKS_HOST:$SOCKS_PORT .*@.*" 2>/dev/null || true
        candidates_total="$(count_upstream_candidates)"
        if [[ "$listener_up" == false ]]; then
          echo "$(date '+%F %T') failover begin reason=listener_down" >>"$LOG_FILE"
          write_health_state "$listener_up" "$health_ok" "restart_listener_down"
        else
          echo "$(date '+%F %T') failover begin reason=degraded fails=$degrade_fails threshold=$UPSTREAM_DEGRADE_FAILS" >>"$LOG_FILE"
          write_health_state "$listener_up" "$health_ok" "restart_degraded"
        fi
        if [[ "$candidates_total" =~ ^[0-9]+$ ]]; then
          if [[ "$candidates_total" -eq 0 ]]; then
            echo "$(date '+%F %T') failover note reason=no_candidate_configured" >>"$LOG_FILE"
          elif [[ "$candidates_total" -eq 1 ]]; then
            echo "$(date '+%F %T') failover note reason=single_candidate" >>"$LOG_FILE"
          fi
        fi
        endpoint_ok=false
        best_endpoint="$(select_best_endpoint || true)"
        if [[ -n "${best_endpoint:-}" ]]; then
          if launch_for_endpoint "$best_endpoint"; then
            endpoint_ok=true
            echo "$(date '+%F %T') failover success endpoint=${best_endpoint} transport=${last_transport} strategy=${UPSTREAM_STRATEGY}" >>"$LOG_FILE"
          fi
        fi
        if [[ "$endpoint_ok" != true ]]; then
          while IFS= read -r endpoint; do
            [[ -z "$endpoint" ]] && continue
            if [[ -n "${best_endpoint:-}" && "$endpoint" == "$best_endpoint" ]]; then
              continue
            fi
            if launch_for_endpoint "$endpoint"; then
              endpoint_ok=true
              echo "$(date '+%F %T') failover success_fallback endpoint=${endpoint} transport=${last_transport}" >>"$LOG_FILE"
              break
            fi
          done < <(build_upstream_candidates)
        fi
        if [[ "$endpoint_ok" != true ]]; then
          echo "$(date '+%F %T') failover note reason=no_alternative_candidate" >>"$LOG_FILE"
          echo "$(date '+%F %T') failover failed" >>"$LOG_FILE"
          write_health_state "false" "false" "restart_failed"
        else
          degrade_fails=0
          write_health_state "true" "true" "restart_ok"
        fi
      fi
    fi
  fi
  sleep "$loop_sleep"
done
