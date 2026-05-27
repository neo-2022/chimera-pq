#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONTROL="$ROOT_DIR/scripts/chimera-control.sh"
APP_ROUTES_FILE="${APP_ROUTES_FILE:-$ROOT_DIR/configs/chimera-app-routes.conf}"
PATH_PROOF_JSON="${PATH_PROOF_JSON:-$ROOT_DIR/docs/CHIMERA_PATH_PROOF.json}"
CHANNEL_AUDIT_JSON="${CHANNEL_AUDIT_JSON:-$ROOT_DIR/docs/CHIMERA_CHANNEL_AUDIT.json}"
QUIET="${CHIMERA_QUIET:-0}"

cd "$ROOT_DIR"

if [[ ! -f "$APP_ROUTES_FILE" ]]; then
  if [[ -f "$ROOT_DIR/configs/chimera-app-routes.example.conf" ]]; then
    cp "$ROOT_DIR/configs/chimera-app-routes.example.conf" "$APP_ROUTES_FILE"
  fi
fi

if [[ "$QUIET" != "1" ]]; then echo "chimera runtime verification: start"; fi
if systemctl --user list-unit-files 2>/dev/null | rg -q '^chimera-gateway\.service|^chimera-client\.service'; then
  bash "$CONTROL" start || true
else
  if [[ "$QUIET" != "1" ]]; then
    echo "chimera runtime verification: systemd user units missing (chimera-gateway.service/chimera-client.service)"
  fi
fi
sleep 2

if [[ "$QUIET" != "1" ]]; then echo "chimera runtime verification: route-status"; fi
APP_ROUTES_FILE="$APP_ROUTES_FILE" bash "$CONTROL" route-status

if [[ "$QUIET" != "1" ]]; then echo "chimera runtime verification: path-proof"; fi
if [[ "$QUIET" == "1" ]]; then
  CHIMERA_QUIET=1 bash "$ROOT_DIR/scripts/chimera-path-proof.sh" "$PATH_PROOF_JSON" >/dev/null 2>&1 || true
else
  bash "$ROOT_DIR/scripts/chimera-path-proof.sh" "$PATH_PROOF_JSON" || true
fi

if [[ "$QUIET" != "1" ]]; then echo "chimera runtime verification: channel-audit"; fi
if [[ "$QUIET" == "1" ]]; then
  CHIMERA_QUIET=1 CHIMERA_CHANNEL_AUDIT_SKIP_PATH_PROOF=1 bash "$ROOT_DIR/scripts/chimera_channel_audit.sh" "$CHANNEL_AUDIT_JSON" >/dev/null 2>&1 || true
else
  CHIMERA_CHANNEL_AUDIT_SKIP_PATH_PROOF=1 bash "$ROOT_DIR/scripts/chimera_channel_audit.sh" "$CHANNEL_AUDIT_JSON" || true
fi

if [[ "$QUIET" != "1" ]]; then echo "chimera runtime verification: summary"; fi
if [[ -f "$PATH_PROOF_JSON" ]]; then
  if command -v jq >/dev/null 2>&1; then
    jq -r '"path_proof.status=\(.status // "unknown")",
           "path_proof.reason=\(.reason // "unknown")",
           "path_proof.listener_ok=\(.listener.ok // false)",
           "path_proof.direct_ip=\(.observed_public_ip.direct.ip // "")",
           "path_proof.chimera_ip=\(.observed_public_ip.chimera.ip // "")",
           "path_proof.targets_passed=\(.totals.passed // 0)/\(.totals.targets // 0)"' "$PATH_PROOF_JSON"
  else
    echo "path_proof.status=$(awk -F'\"' '/\"status\":/ {print $4; exit}' "$PATH_PROOF_JSON")"
    echo "path_proof.reason=$(awk -F'\"' '/\"reason\":/ {print $4; exit}' "$PATH_PROOF_JSON")"
  fi
else
  echo "path_proof_json_missing=$PATH_PROOF_JSON"
fi

if [[ -f "$CHANNEL_AUDIT_JSON" ]]; then
  if command -v jq >/dev/null 2>&1; then
    jq -r '"channel_audit.status=\(.status // "unknown")",
           "channel_audit.reason=\(.reason // "unknown")",
           "channel_audit.listener=\(.chimera.proxy_listener // "unknown")",
           "channel_audit.path_status=\(.path_proof.status // "unknown")",
           "channel_audit.direct_ip=\(.path_proof.public_ip_direct // "")",
           "channel_audit.chimera_ip=\(.path_proof.public_ip_via_chimera // "")",
           "channel_audit.app_routes=\(.selective_routing.app_routes_count // 0)",
           "channel_audit.service_routes=\(.selective_routing.service_routes_count // 0)",
           "channel_audit.iface_class=\(.system_default_path.iface_class // "unknown")"' "$CHANNEL_AUDIT_JSON"
  else
    echo "channel_audit.status=$(awk -F'\"' '/\"status\":/ {print $4; exit}' "$CHANNEL_AUDIT_JSON")"
    echo "channel_audit.reason=$(awk -F'\"' '/\"reason\":/ {print $4; exit}' "$CHANNEL_AUDIT_JSON")"
  fi
else
  echo "channel_audit_json_missing=$CHANNEL_AUDIT_JSON"
fi

if [[ "$QUIET" != "1" ]]; then echo "chimera runtime verification: done"; fi
