#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${1:-$ROOT_DIR/docs/CHIMERA_AUTONOMOUS_NAT_GUARD.json}"
CONTROL="$ROOT_DIR/scripts/chimera-control.sh"

now_utc() { date -u +"%Y-%m-%dT%H:%M:%SZ"; }
esc() { printf '%s' "${1:-}" | sed 's/\\/\\\\/g; s/"/\\"/g'; }

mkdir -p "$(dirname "$OUT")"

started_at="$(now_utc)"

# 1) Local autonomy preflight: must not require manual route toggles or app proxy setup.
"$CONTROL" start >/tmp/chimera_autonomous_start.log 2>&1 || true
datapath_status="$("$CONTROL" datapath-status 2>/dev/null || true)"
route_status="$("$CONTROL" route-status 2>/dev/null || true)"

datapath_up="false"
if printf '%s\n' "$datapath_status" | grep -q '^runtime_state_status=up$'; then
  datapath_up="true"
fi

# 2) Path proof as autonomous reachability signal.
CHIMERA_QUIET=1 bash "$ROOT_DIR/scripts/chimera-path-proof.sh" "$ROOT_DIR/docs/CHIMERA_PATH_PROOF.json" >/dev/null 2>&1 || true
path_status="$(jq -r '.status // "unknown"' "$ROOT_DIR/docs/CHIMERA_PATH_PROOF.json" 2>/dev/null || echo unknown)"
path_reason="$(jq -r '.reason // "unknown"' "$ROOT_DIR/docs/CHIMERA_PATH_PROOF.json" 2>/dev/null || echo unknown)"
path_mode="$(jq -r '.mode // "unknown"' "$ROOT_DIR/docs/CHIMERA_PATH_PROOF.json" 2>/dev/null || echo unknown)"
path_chimera_evidence="$(jq -r '.chimera_datapath_evidence // false' "$ROOT_DIR/docs/CHIMERA_PATH_PROOF.json" 2>/dev/null || echo false)"
path_datapath_attempted="$(jq -r '.datapath.attempted // false' "$ROOT_DIR/docs/CHIMERA_PATH_PROOF.json" 2>/dev/null || echo false)"
path_datapath_ok="$(jq -r '.datapath.ok // false' "$ROOT_DIR/docs/CHIMERA_PATH_PROOF.json" 2>/dev/null || echo false)"
path_datapath_targets_total="$(jq -r '.datapath.targets_total // 0' "$ROOT_DIR/docs/CHIMERA_PATH_PROOF.json" 2>/dev/null || echo 0)"
path_datapath_targets_passed="$(jq -r '.datapath.targets_passed // 0' "$ROOT_DIR/docs/CHIMERA_PATH_PROOF.json" 2>/dev/null || echo 0)"
path_datapath_targets_failed="$(jq -r '.datapath.targets_failed // 0' "$ROOT_DIR/docs/CHIMERA_PATH_PROOF.json" 2>/dev/null || echo 0)"
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

# 3) Egress adaptation capability signal (must be >=2 candidates for autonomous geo adaptation).
upstream_audit="$("$CONTROL" upstream-audit 20 2>/dev/null || true)"
candidates_total="$(printf '%s\n' "$upstream_audit" | awk -F= '/^upstream_candidates_total=/{print $2; exit}')"
adaptation_possible="$(printf '%s\n' "$upstream_audit" | awk -F= '/^upstream_adaptation_possible=/{print $2; exit}')"
upstream_source="$(printf '%s\n' "$upstream_audit" | awk -F= '/^upstream_source=/{print $2; exit}')"
legacy_upstream_source_used="$(printf '%s\n' "$upstream_audit" | awk -F= '/^legacy_upstream_source_used=/{print $2; exit}')"
upstream_product_datapath_evidence="$(printf '%s\n' "$upstream_audit" | awk -F= '/^upstream_product_datapath_evidence=/{print $2; exit}')"

if [[ -z "${candidates_total:-}" ]]; then candidates_total="0"; fi
if [[ -z "${adaptation_possible:-}" ]]; then adaptation_possible="false"; fi
if [[ -z "${upstream_source:-}" ]]; then upstream_source="unknown"; fi
if [[ -z "${legacy_upstream_source_used:-}" ]]; then legacy_upstream_source_used="false"; fi
if [[ -z "${upstream_product_datapath_evidence:-}" ]]; then upstream_product_datapath_evidence="unknown"; fi

status="fail"
reason="autonomy_guard_failed"
if [[ "$legacy_upstream_source_used" == "true" ]]; then
  status="fail"
  reason="legacy_upstream_source_not_allowed"
elif [[ "$upstream_product_datapath_evidence" != "false" ]]; then
  status="fail"
  reason="upstream_product_datapath_evidence_not_false"
elif [[ "$datapath_up" == "true" && "$path_contract_ok" == "true" && "$adaptation_possible" == "true" ]]; then
  status="pass"
  reason="autonomous_path_and_adaptation_ready"
elif [[ "$datapath_up" == "true" && "$path_status" == "pass" && "$path_contract_ok" != "true" ]]; then
  status="fail"
  reason="path_proof_contract_failed"
elif [[ "$datapath_up" == "true" && "$path_contract_ok" == "true" ]]; then
  status="partial"
  reason="path_ready_but_multi_egress_missing"
fi

finished_at="$(now_utc)"

cat >"$OUT" <<EOF
{"kind":"chimera_autonomous_nat_guard","status":"$status","reason":"$reason","started_at":"$started_at","finished_at":"$finished_at","network_state":"not_modified","signals":{"datapath_up":$datapath_up,"path_status":"$(esc "$path_status")","path_reason":"$(esc "$path_reason")","path_mode":"$(esc "$path_mode")","path_chimera_datapath_evidence":$path_chimera_evidence,"path_datapath_attempted":$path_datapath_attempted,"path_datapath_ok":$path_datapath_ok,"path_datapath_targets_total":$path_datapath_targets_total,"path_datapath_targets_passed":$path_datapath_targets_passed,"path_datapath_targets_failed":$path_datapath_targets_failed,"path_contract_ok":$path_contract_ok,"upstream_candidates_total":$candidates_total,"upstream_adaptation_possible":"$(esc "$adaptation_possible")","upstream_source":"$(esc "$upstream_source")","legacy_upstream_source_used":"$(esc "$legacy_upstream_source_used")","upstream_product_datapath_evidence":"$(esc "$upstream_product_datapath_evidence")"}}
EOF

cat "$OUT"
[[ "$status" == "pass" ]]
