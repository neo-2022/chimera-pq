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
    tun*|wg*|ppp*|tailscale*|zt*|utun*|tunnel*)
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

  local runtime_state transparent_runtime route_mode split_mode
  runtime_state="$(extract_route_status_value "$route_status_tmp" "runtime_state_status")"
  transparent_runtime="$(extract_route_status_value "$route_status_tmp" "transparent_runtime")"
  route_mode="$(extract_route_status_value "$route_status_tmp" "route_mode")"
  split_mode="$(extract_route_status_value "$route_status_tmp" "split_list_mode")"
  runtime_state="${runtime_state:-unknown}"
  transparent_runtime="${transparent_runtime:-unknown}"
  route_mode="${route_mode:-unknown}"
  split_mode="${split_mode:-unknown}"

  local app_routes_count service_routes_count service_overrides_enabled
  app_routes_count="$(extract_route_status_value "$app_status_tmp" "app_routes_count")"
  service_routes_count="$(extract_route_status_value "$app_status_tmp" "service_routes_count")"
  app_routes_count="${app_routes_count:-0}"
  service_routes_count="${service_routes_count:-0}"
  service_overrides_enabled="$(count_route_lines "$app_status_tmp" "service_route_override[")"

  local path_status path_reason path_direct_ip
  local path_targets_total path_targets_passed path_targets_failed
  if command -v jq >/dev/null 2>&1; then
    path_status="$(jq -r '.status // empty' "$PATH_PROOF_JSON" 2>/dev/null || true)"
    path_reason="$(jq -r '.reason // empty' "$PATH_PROOF_JSON" 2>/dev/null || true)"
    path_direct_ip="$(jq -r '.direct_baseline.remote_ip // empty' "$PATH_PROOF_JSON" 2>/dev/null || true)"
    path_targets_total="$(jq -r '.datapath.targets_total // 0' "$PATH_PROOF_JSON" 2>/dev/null || true)"
    path_targets_passed="$(jq -r '.datapath.targets_passed // 0' "$PATH_PROOF_JSON" 2>/dev/null || true)"
    path_targets_failed="$(jq -r '.datapath.targets_failed // 0' "$PATH_PROOF_JSON" 2>/dev/null || true)"
  else
    path_status="$(extract_json_string "$PATH_PROOF_JSON" "status")"
    path_reason="$(extract_json_string "$PATH_PROOF_JSON" "reason")"
    path_direct_ip="$(tr -d '\n' <"$PATH_PROOF_JSON" | sed -n 's/.*"direct_baseline":[[:space:]]*{[^}]*"remote_ip":[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)"
    path_targets_total="$(tr -d '\n' <"$PATH_PROOF_JSON" | sed -n 's/.*"targets_total":[[:space:]]*\([0-9][0-9]*\).*/\1/p' | head -n 1)"
    path_targets_passed="$(tr -d '\n' <"$PATH_PROOF_JSON" | sed -n 's/.*"targets_passed":[[:space:]]*\([0-9][0-9]*\).*/\1/p' | head -n 1)"
    path_targets_failed="$(tr -d '\n' <"$PATH_PROOF_JSON" | sed -n 's/.*"targets_failed":[[:space:]]*\([0-9][0-9]*\).*/\1/p' | head -n 1)"
  fi
  path_status="${path_status:-unknown}"
  path_reason="${path_reason:-unknown}"
  path_direct_ip="${path_direct_ip:-}"
  path_targets_total="${path_targets_total:-0}"
  path_targets_passed="${path_targets_passed:-0}"
  path_targets_failed="${path_targets_failed:-0}"

  local default_iface default_gateway default_iface_class
  default_iface="$(detect_default_iface)"
  default_gateway="$(detect_default_gateway)"
  default_iface_class="$(classify_default_iface "$default_iface")"

  local overall_status overall_reason
  if [[ "$transparent_runtime" != "running" && "$runtime_state" != "up" ]]; then
    overall_status="fail"
    overall_reason="transparent_datapath_not_running"
  elif [[ "$path_status" != "pass" ]]; then
    overall_status="fail"
    overall_reason="path_proof_${path_reason}"
  else
    overall_status="pass"
    overall_reason="transparent_datapath_observed"
  fi

  local finished_at
  finished_at="$(now_utc)"

  cat >"$AUDIT_JSON_OUT" <<EOF
{"kind":"chimera_channel_audit","status":"$overall_status","reason":"$overall_reason","started_at":"$started_at","finished_at":"$finished_at","network_state":"not_modified","chimera":{"runtime_state":"$(json_escape "$runtime_state")","transparent_runtime":"$(json_escape "$transparent_runtime")","route_mode":"$(json_escape "$route_mode")","split_list_mode":"$(json_escape "$split_mode")"},"path_proof":{"status":"$path_status","reason":"$(json_escape "$path_reason")","direct_remote_ip":"$(json_escape "$path_direct_ip")","datapath_mode":"transparent_datapath","targets_total":$path_targets_total,"targets_passed":$path_targets_passed,"targets_failed":$path_targets_failed},"selective_routing":{"app_routes_count":$app_routes_count,"service_routes_count":$service_routes_count,"service_override_rows":$service_overrides_enabled,"app_routes_file":"$(json_escape "$APP_ROUTES_FILE")"},"system_default_path":{"iface":"$(json_escape "$default_iface")","gateway":"$(json_escape "$default_gateway")","iface_class":"$default_iface_class"}}
EOF

  if [[ "$QUIET" != "1" ]]; then
    cat "$AUDIT_JSON_OUT"
  fi
  rm -f "$route_status_tmp" "$app_status_tmp"

  [[ "$overall_status" == "pass" ]]
}

main "$@"
