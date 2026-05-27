#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

gate_json="${1:-docs/CHIMERA_E2E_CHANNEL_GATE.json}"
max_age_sec="${CHIMERA_E2E_GATE_MAX_AGE_SEC:-1800}"

if [[ ! -f "$gate_json" ]]; then
  echo "chimera e2e channel gate guard: missing artifact: $gate_json"
  exit 1
fi

if ! [[ "$max_age_sec" =~ ^[0-9]+$ ]] || (( max_age_sec < 1 )); then
  echo "chimera e2e channel gate guard: CHIMERA_E2E_GATE_MAX_AGE_SEC must be positive integer"
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "chimera e2e channel gate guard: jq is required"
  exit 1
fi

status="$(jq -r '.status // empty' "$gate_json")"
reason="$(jq -r '.reason // empty' "$gate_json")"
path_status="$(jq -r '.path_proof.status // empty' "$gate_json")"
audit_status="$(jq -r '.channel_audit.status // empty' "$gate_json")"
run_app_ok="$(jq -r '.selected_route_checks.run_app_curl_example_ok // false' "$gate_json")"
svc_checked="$(jq -r '.selected_route_checks.service_override_checked // false' "$gate_json")"
svc_after="$(jq -r '.selected_route_checks.service_override_after // empty' "$gate_json")"
network_state="$(jq -r '.network_state // empty' "$gate_json")"

if [[ "$status" != "pass" ]]; then
  echo "chimera e2e channel gate guard: status must be pass (got: $status)"
  exit 1
fi
if [[ "$path_status" != "pass" || "$audit_status" != "pass" ]]; then
  echo "chimera e2e channel gate guard: path/channel status mismatch: path=$path_status channel=$audit_status"
  exit 1
fi
if [[ "$run_app_ok" != "true" || "$svc_checked" != "true" || "$svc_after" != "enabled" ]]; then
  echo "chimera e2e channel gate guard: selected-route checks failed: run_app=$run_app_ok checked=$svc_checked after=$svc_after"
  exit 1
fi
if [[ "$network_state" != "not_modified" ]]; then
  echo "chimera e2e channel gate guard: network_state must be not_modified (got: $network_state)"
  exit 1
fi
if [[ -z "$reason" ]]; then
  echo "chimera e2e channel gate guard: reason must be non-empty"
  exit 1
fi

now_epoch="$(date +%s)"
mtime="$(stat -c %Y "$gate_json")"
age=$(( now_epoch - mtime ))
if (( age < 0 )); then
  echo "chimera e2e channel gate guard: artifact mtime is in the future"
  exit 1
fi
if (( age > max_age_sec )); then
  echo "chimera e2e channel gate guard: stale artifact ($age sec): $gate_json"
  exit 1
fi

echo "chimera e2e channel gate guard: PASS"
