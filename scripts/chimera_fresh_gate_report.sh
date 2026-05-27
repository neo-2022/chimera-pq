#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_JSON="${1:-$ROOT_DIR/docs/CHIMERA_FRESH_GATE_REPORT.json}"
OUT_MD="${OUT_JSON%.json}.md"
PATH_JSON="${CHIMERA_PATH_PROOF_JSON:-$ROOT_DIR/docs/CHIMERA_PATH_PROOF.json}"
AUDIT_JSON="${CHIMERA_CHANNEL_AUDIT_JSON:-$ROOT_DIR/docs/CHIMERA_CHANNEL_AUDIT.json}"
E2E_JSON="${CHIMERA_E2E_GATE_JSON:-$ROOT_DIR/docs/CHIMERA_E2E_CHANNEL_GATE.json}"
LOAD_JSON="${CHIMERA_LOAD_GATE_JSON:-$ROOT_DIR/docs/CHIMERA_LOAD_GATE_LAPTOP.json}"

if ! command -v jq >/dev/null 2>&1; then
  echo "chimera-fresh-gate-report: jq is required" >&2
  exit 1
fi

for f in "$PATH_JSON" "$AUDIT_JSON" "$E2E_JSON" "$LOAD_JSON"; do
  if [[ ! -f "$f" ]]; then
    echo "chimera-fresh-gate-report: missing artifact: $f" >&2
    exit 1
  fi
done

path_status="$(jq -r '.status // "unknown"' "$PATH_JSON")"
audit_status="$(jq -r '.status // "unknown"' "$AUDIT_JSON")"
e2e_status="$(jq -r '.status // "unknown"' "$E2E_JSON")"
load_status="$(jq -r '.status // "unknown"' "$LOAD_JSON")"

overall="fail"
overall_reason="one_or_more_gates_failed"
if [[ "$path_status" == "pass" && "$audit_status" == "pass" && "$e2e_status" == "pass" && "$load_status" == "pass" ]]; then
  overall="pass"
  overall_reason="all_fresh_gates_passed"
fi

mkdir -p "$(dirname "$OUT_JSON")"

jq -n \
  --arg kind "chimera_fresh_gate_report" \
  --arg status "$overall" \
  --arg reason "$overall_reason" \
  --arg path_json "$PATH_JSON" \
  --arg audit_json "$AUDIT_JSON" \
  --arg e2e_json "$E2E_JSON" \
  --arg load_json "$LOAD_JSON" \
  --arg path_status "$path_status" \
  --arg audit_status "$audit_status" \
  --arg e2e_status "$e2e_status" \
  --arg load_status "$load_status" \
  --arg ts "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
  '{
    kind: $kind,
    status: $status,
    reason: $reason,
    generated_at_utc: $ts,
    artifacts: {
      path_proof: $path_json,
      channel_audit: $audit_json,
      e2e_gate: $e2e_json,
      load_gate: $load_json
    },
    checks: {
      path_proof_status: $path_status,
      channel_audit_status: $audit_status,
      e2e_gate_status: $e2e_status,
      load_gate_status: $load_status
    }
  }' >"$OUT_JSON"

cat >"$OUT_MD" <<EOF
# CHIMERA Fresh Gate Report

- status: $overall
- reason: $overall_reason

Checks:
- path_proof: $path_status
- channel_audit: $audit_status
- e2e_gate: $e2e_status
- load_gate: $load_status

Artifacts:
- $PATH_JSON
- $AUDIT_JSON
- $E2E_JSON
- $LOAD_JSON
EOF

cat "$OUT_JSON"
[[ "$overall" == "pass" ]]
