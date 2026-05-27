#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONTROL_SCRIPT="${ROOT_DIR}/scripts/chimera-control.sh"
PATH_PROOF_SCRIPT="${ROOT_DIR}/scripts/chimera-path-proof.sh"
APP_ROUTES_FILE="${APP_ROUTES_FILE:-$ROOT_DIR/configs/chimera-app-routes.conf}"
PATH_PROOF_JSON="${PATH_PROOF_JSON:-$ROOT_DIR/docs/CHIMERA_PATH_PROOF.json}"
AUDIT_JSON_OUT="${1:-${CHIMERA_CHANNEL_AUDIT_JSON_OUT:-$ROOT_DIR/docs/CHIMERA_CHANNEL_AUDIT.json}}"
QUIET="${CHIMERA_QUIET:-0}"

now_utc() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

json_escape() {
  local s="${1:-}"
  s=${s//\\/\\\\}
  s=${s//\"/\\\"}
  s=${s//$'\n'/ }
  s=${s//$'\r'/ }
  s=${s//$'\t'/ }
  printf '%s' "$s"
}

extract_json_string() {
  local file="$1"
  local key="$2"
  if command -v jq >/dev/null 2>&1; then
    jq -r --arg k "$key" '.[$k] // empty' "$file" 2>/dev/null | head -n 1
    return 0
  fi
  tr -d '\n' <"$file" | sed -n "s/.*\"${key}\":[[:space:]]*\"\\([^\"]*\\)\".*/\\1/p" | head -n 1
}

extract_json_number() {
  local file="$1"
  local key="$2"
  if command -v jq >/dev/null 2>&1; then
    jq -r --arg k "$key" '.[$k] // empty' "$file" 2>/dev/null | head -n 1
    return 0
  fi
  tr -d '\n' <"$file" | sed -n "s/.*\"${key}\":[[:space:]]*\\([0-9][0-9]*\\).*/\\1/p" | head -n 1
}

extract_route_status_value() {
  local file="$1"
  local key="$2"
  awk -F= -v k="$key" '$1==k {print substr($0, index($0,$2)); exit}' "$file"
}

count_route_lines() {
  local file="$1"
  local prefix="$2"
  awk -v pfx="$prefix" 'index($0, pfx)==1 {n++} END {print n+0}' "$file"
}

detect_default_iface() {
  ip route show default 2>/dev/null | awk '/default/ {for(i=1;i<=NF;i++) if ($i=="dev") {print $(i+1); exit}}'
}

detect_default_gateway() {
  ip route show default 2>/dev/null | awk '/default/ {for(i=1;i<=NF;i++) if ($i=="via") {print $(i+1); exit}}'
}

classify_default_iface() {
  local iface="${1:-}"
  if [[ -z "$iface" ]]; then
    echo "unknown"
    return
  fi
  case "$iface" in
    tun*|wg*|ppp*|tailscale*|zt*|utun*|vpn*)
      echo "vpn_or_tunnel"
      ;;
    *)
      echo "regular_interface"
      ;;
  esac
}

main() {
  mkdir -p "$(dirname "$AUDIT_JSON_OUT")"

  local started_at
  started_at="$(now_utc)"

  local route_status_tmp app_status_tmp
  route_status_tmp="$(mktemp)"
  app_status_tmp="$(mktemp)"

  APP_ROUTES_FILE="$APP_ROUTES_FILE" bash "$CONTROL_SCRIPT" route-status >"$route_status_tmp" || true
  APP_ROUTES_FILE="$APP_ROUTES_FILE" bash "$CONTROL_SCRIPT" app-routes-status >"$app_status_tmp" || true

  if [[ "${CHIMERA_CHANNEL_AUDIT_SKIP_PATH_PROOF:-0}" != "1" ]]; then
    bash "$PATH_PROOF_SCRIPT" "$PATH_PROOF_JSON" >/dev/null 2>&1 || true
  fi

  local proxy_listener route_proxy_url
  proxy_listener="$(extract_route_status_value "$route_status_tmp" "chimera_proxy_listener")"
  route_proxy_url="$(extract_route_status_value "$route_status_tmp" "chimera_proxy_url")"
  proxy_listener="${proxy_listener:-unknown}"
  route_proxy_url="${route_proxy_url:-unknown}"

  local app_routes_count service_routes_count service_overrides_enabled
  app_routes_count="$(extract_route_status_value "$app_status_tmp" "app_routes_count")"
  service_routes_count="$(extract_route_status_value "$app_status_tmp" "service_routes_count")"
  app_routes_count="${app_routes_count:-0}"
  service_routes_count="${service_routes_count:-0}"
  service_overrides_enabled="$(count_route_lines "$app_status_tmp" "service_route_override[")"

  local path_status path_reason path_direct_ip path_chimera_ip
  local path_targets_total path_targets_passed path_targets_failed
  if command -v jq >/dev/null 2>&1; then
    path_status="$(jq -r '.status // empty' "$PATH_PROOF_JSON" 2>/dev/null || true)"
    path_reason="$(jq -r '.reason // empty' "$PATH_PROOF_JSON" 2>/dev/null || true)"
    path_direct_ip="$(jq -r '.observed_public_ip.direct.ip // empty' "$PATH_PROOF_JSON" 2>/dev/null || true)"
    path_chimera_ip="$(jq -r '.observed_public_ip.chimera.ip // empty' "$PATH_PROOF_JSON" 2>/dev/null || true)"
    path_targets_total="$(jq -r '.totals.targets // 0' "$PATH_PROOF_JSON" 2>/dev/null || true)"
    path_targets_passed="$(jq -r '.totals.passed // 0' "$PATH_PROOF_JSON" 2>/dev/null || true)"
    path_targets_failed="$(jq -r '.totals.failed // 0' "$PATH_PROOF_JSON" 2>/dev/null || true)"
  else
    path_status="$(extract_json_string "$PATH_PROOF_JSON" "status")"
    path_reason="$(extract_json_string "$PATH_PROOF_JSON" "reason")"
    path_direct_ip="$(extract_json_string "$PATH_PROOF_JSON" "ip")"
    path_chimera_ip="$(tr -d '\n' <"$PATH_PROOF_JSON" | sed -n 's/.*"chimera":[[:space:]]*{[^}]*"ip":[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)"
    path_targets_total="$(extract_json_number "$PATH_PROOF_JSON" "targets")"
    path_targets_passed="$(extract_json_number "$PATH_PROOF_JSON" "passed")"
    path_targets_failed="$(extract_json_number "$PATH_PROOF_JSON" "failed")"
  fi
  path_status="${path_status:-unknown}"
  path_reason="${path_reason:-unknown}"
  path_direct_ip="${path_direct_ip:-}"
  path_chimera_ip="${path_chimera_ip:-}"
  path_targets_total="${path_targets_total:-0}"
  path_targets_passed="${path_targets_passed:-0}"
  path_targets_failed="${path_targets_failed:-0}"

  local default_iface default_gateway default_iface_class
  default_iface="$(detect_default_iface)"
  default_gateway="$(detect_default_gateway)"
  default_iface_class="$(classify_default_iface "$default_iface")"

  local overall_status overall_reason
  if [[ "$proxy_listener" != "up" ]]; then
    overall_status="fail"
    overall_reason="chimera_listener_down"
  elif [[ "$path_status" != "pass" ]]; then
    overall_status="fail"
    overall_reason="path_proof_${path_reason}"
  elif [[ "$path_direct_ip" == "$path_chimera_ip" ]]; then
    overall_status="warn"
    overall_reason="same_public_ip_direct_and_chimera"
  else
    overall_status="pass"
    overall_reason="channel_separation_observed"
  fi

  local finished_at
  finished_at="$(now_utc)"

  cat >"$AUDIT_JSON_OUT" <<EOF
{"kind":"chimera_channel_audit","status":"$overall_status","reason":"$overall_reason","started_at":"$started_at","finished_at":"$finished_at","network_state":"not_modified","chimera":{"proxy_listener":"$proxy_listener","proxy_url":"$(json_escape "$route_proxy_url")"},"path_proof":{"status":"$path_status","reason":"$(json_escape "$path_reason")","public_ip_direct":"$(json_escape "$path_direct_ip")","public_ip_via_chimera":"$(json_escape "$path_chimera_ip")","targets_total":$path_targets_total,"targets_passed":$path_targets_passed,"targets_failed":$path_targets_failed},"selective_routing":{"app_routes_count":$app_routes_count,"service_routes_count":$service_routes_count,"service_override_rows":$service_overrides_enabled,"app_routes_file":"$(json_escape "$APP_ROUTES_FILE")"},"system_default_path":{"iface":"$(json_escape "$default_iface")","gateway":"$(json_escape "$default_gateway")","iface_class":"$default_iface_class"}}
EOF

  if [[ "$QUIET" != "1" ]]; then
    cat "$AUDIT_JSON_OUT"
  fi
  rm -f "$route_status_tmp" "$app_status_tmp"

  [[ "$overall_status" == "pass" ]]
}

main "$@"
