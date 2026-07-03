#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_JSON="${1:-$ROOT_DIR/docs/CHIMERA_FRESH_GATE_REPORT.json}"
OUT_MD="${OUT_JSON%.json}.md"
PATH_JSON="${CHIMERA_PATH_PROOF_JSON:-$ROOT_DIR/docs/CHIMERA_PATH_PROOF.json}"
AUDIT_JSON="${CHIMERA_CHANNEL_AUDIT_JSON:-$ROOT_DIR/docs/CHIMERA_CHANNEL_AUDIT.json}"
E2E_JSON="${CHIMERA_E2E_GATE_JSON:-$ROOT_DIR/docs/CHIMERA_E2E_CHANNEL_GATE.json}"
LOAD_JSON="${CHIMERA_LOAD_GATE_JSON:-$ROOT_DIR/docs/CHIMERA_LOAD_GATE_SIDE_B.json}"

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
path_mode="$(jq -r '.mode // "unknown"' "$PATH_JSON")"
path_chimera_evidence="$(jq -r '.chimera_datapath_evidence // false' "$PATH_JSON")"
path_datapath_attempted="$(jq -r '.datapath.attempted // false' "$PATH_JSON")"
path_datapath_ok="$(jq -r '.datapath.ok // false' "$PATH_JSON")"
path_datapath_targets_total="$(jq -r '.datapath.targets_total // 0' "$PATH_JSON")"
path_datapath_targets_passed="$(jq -r '.datapath.targets_passed // 0' "$PATH_JSON")"
path_datapath_targets_failed="$(jq -r '.datapath.targets_failed // 0' "$PATH_JSON")"
audit_status="$(jq -r '.status // "unknown"' "$AUDIT_JSON")"
e2e_status="$(jq -r '.status // "unknown"' "$E2E_JSON")"
load_status="$(jq -r '.status // "unknown"' "$LOAD_JSON")"

path_contract_ok="false"
if [[ "$path_status" == "pass" \
  && "$path_mode" == "chimera_transparent_datapath" \
  && "$path_chimera_evidence" == "true" \
  && "$path_datapath_attempted" == "true" \
  && "$path_datapath_ok" == "true" \
  && "$path_datapath_targets_total" =~ ^[0-9]+$ \
  && "$path_datapath_targets_passed" =~ ^[0-9]+$ \
  && "$path_datapath_targets_failed" =~ ^[0-9]+$ \
  && "$path_datapath_targets_total" -gt 0 \
  && "$path_datapath_targets_passed" -eq "$path_datapath_targets_total" \
  && "$path_datapath_targets_failed" -eq 0 ]]; then
  path_contract_ok="true"
fi

overall="fail"
overall_reason="one_or_more_gates_failed"
if [[ "$path_contract_ok" != "true" ]]; then
  overall_reason="path_proof_contract_failed"
elif [[ "$path_contract_ok" == "true" && "$audit_status" == "pass" && "$e2e_status" == "pass" && "$load_status" == "pass" ]]; then
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
  --arg path_mode "$path_mode" \
  --arg path_chimera_evidence "$path_chimera_evidence" \
  --arg path_datapath_attempted "$path_datapath_attempted" \
  --arg path_datapath_ok "$path_datapath_ok" \
  --argjson path_datapath_targets_total "$path_datapath_targets_total" \
  --argjson path_datapath_targets_passed "$path_datapath_targets_passed" \
  --argjson path_datapath_targets_failed "$path_datapath_targets_failed" \
  --arg path_contract_ok "$path_contract_ok" \
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
      path_proof_mode: $path_mode,
      path_proof_chimera_datapath_evidence: ($path_chimera_evidence == "true"),
      path_proof_datapath_attempted: ($path_datapath_attempted == "true"),
      path_proof_datapath_ok: ($path_datapath_ok == "true"),
      path_proof_datapath_targets_total: $path_datapath_targets_total,
      path_proof_datapath_targets_passed: $path_datapath_targets_passed,
      path_proof_datapath_targets_failed: $path_datapath_targets_failed,
      path_proof_contract_ok: ($path_contract_ok == "true"),
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
- path_proof_mode: $path_mode
- path_proof_chimera_datapath_evidence: $path_chimera_evidence
- path_proof_datapath_attempted: $path_datapath_attempted
- path_proof_datapath_ok: $path_datapath_ok
- path_proof_datapath_targets: $path_datapath_targets_passed/$path_datapath_targets_total failed=$path_datapath_targets_failed
- path_proof_contract_ok: $path_contract_ok
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
