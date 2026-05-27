#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONTROL_SCRIPT="$ROOT_DIR/scripts/chimera-control.sh"
RUNTIME_VERIFY_SCRIPT="$ROOT_DIR/scripts/chimera_runtime_verification.sh"
APP_ROUTES_FILE="${APP_ROUTES_FILE:-$ROOT_DIR/configs/chimera-app-routes.conf}"
PATH_PROOF_JSON="${PATH_PROOF_JSON:-$ROOT_DIR/docs/CHIMERA_PATH_PROOF.json}"
CHANNEL_AUDIT_JSON="${CHANNEL_AUDIT_JSON:-$ROOT_DIR/docs/CHIMERA_CHANNEL_AUDIT.json}"
E2E_JSON_OUT="${1:-${CHIMERA_E2E_CHANNEL_JSON_OUT:-$ROOT_DIR/docs/CHIMERA_E2E_CHANNEL_GATE.json}}"
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

main() {
  mkdir -p "$(dirname "$E2E_JSON_OUT")"

  local started_at
  started_at="$(now_utc)"
  local allow_warn_audit="${CHIMERA_E2E_ALLOW_WARN_AUDIT:-0}"

  APP_ROUTES_FILE="$APP_ROUTES_FILE" bash "$RUNTIME_VERIFY_SCRIPT" >/tmp/chimera_e2e_runtime_verify.log 2>&1 || true

  local path_status path_reason audit_status audit_reason
  path_status="$(jq -r '.status // "unknown"' "$PATH_PROOF_JSON" 2>/dev/null || echo "unknown")"
  path_reason="$(jq -r '.reason // "unknown"' "$PATH_PROOF_JSON" 2>/dev/null || echo "unknown")"
  audit_status="$(jq -r '.status // "unknown"' "$CHANNEL_AUDIT_JSON" 2>/dev/null || echo "unknown")"
  audit_reason="$(jq -r '.reason // "unknown"' "$CHANNEL_AUDIT_JSON" 2>/dev/null || echo "unknown")"

  local run_app_ok=false
  local run_try
  : > /tmp/chimera_e2e_run_app.log
  for run_try in 1 2 3; do
    if APP_ROUTES_FILE="$APP_ROUTES_FILE" bash "$CONTROL_SCRIPT" run-app curl_example >>/tmp/chimera_e2e_run_app.log 2>&1; then
      run_app_ok=true
      break
    fi
    sleep 1
  done

  local service_override_before service_override_after service_override_checked
  service_override_before="unknown"
  service_override_after="unknown"
  service_override_checked=false
  if APP_ROUTES_FILE="$APP_ROUTES_FILE" bash "$CONTROL_SCRIPT" app-routes-status >/tmp/chimera_e2e_app_status_before.log 2>&1; then
    service_override_before="$(awk -F= '/^service_route_override\[example\]=/ {print $2; exit}' /tmp/chimera_e2e_app_status_before.log)"
  fi
  if APP_ROUTES_FILE="$APP_ROUTES_FILE" bash "$CONTROL_SCRIPT" service-proxy-enable example >/tmp/chimera_e2e_service_enable.log 2>&1; then
    if APP_ROUTES_FILE="$APP_ROUTES_FILE" bash "$CONTROL_SCRIPT" app-routes-status >/tmp/chimera_e2e_app_status_after.log 2>&1; then
      service_override_after="$(awk -F= '/^service_route_override\[example\]=/ {print $2; exit}' /tmp/chimera_e2e_app_status_after.log)"
      service_override_checked=true
    fi
  fi

  local gate_status="fail"
  local gate_reason="unknown"
  local audit_ok_for_gate="false"
  if [[ "$audit_status" == "pass" ]]; then
    audit_ok_for_gate="true"
  elif [[ "$audit_status" == "warn" && "$allow_warn_audit" == "1" ]]; then
    audit_ok_for_gate="true"
  fi

  if [[ "$path_status" == "pass" && "$audit_ok_for_gate" == "true" && "$run_app_ok" == "true" ]]; then
    if [[ "$service_override_checked" == "true" && "$service_override_after" == "enabled" ]]; then
      gate_status="pass"
      gate_reason="channel_audit_and_selected_routes_ok"
    else
      gate_status="warn"
      gate_reason="channel_ok_service_override_not_confirmed"
    fi
  else
    gate_status="fail"
    gate_reason="channel_or_route_gate_failed"
  fi

  local finished_at
  finished_at="$(now_utc)"

  cat >"$E2E_JSON_OUT" <<EOF
{"kind":"chimera_e2e_channel_gate","status":"$gate_status","reason":"$gate_reason","started_at":"$started_at","finished_at":"$finished_at","network_state":"not_modified","path_proof":{"status":"$path_status","reason":"$(json_escape "$path_reason")"},"channel_audit":{"status":"$audit_status","reason":"$(json_escape "$audit_reason")"},"selected_route_checks":{"run_app_curl_example_ok":$run_app_ok,"service_override_checked":$service_override_checked,"service_override_before":"$(json_escape "$service_override_before")","service_override_after":"$(json_escape "$service_override_after")"},"artifacts":{"path_proof_json":"$(json_escape "$PATH_PROOF_JSON")","channel_audit_json":"$(json_escape "$CHANNEL_AUDIT_JSON")"}}
EOF

  if [[ "$QUIET" != "1" ]]; then
    cat "$E2E_JSON_OUT"
  fi
  [[ "$gate_status" == "pass" ]]
}

main "$@"
