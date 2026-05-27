#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

out_path="${1:-docs/CEF_TRACK_REPORT.json}"
md_path="${2:-docs/CEF_TRACK_REPORT.md}"

has_any_path() {
  local found=false
  for p in "$@"; do
    if [[ -e "$p" ]]; then
      found=true
      break
    fi
  done
  if [[ "$found" == true ]]; then
    echo true
  else
    echo false
  fi
}

has_public_api() {
  local file="$1"
  if [[ -f "$file" ]] && rg -q '^pub ' "$file"; then
    echo true
  else
    echo false
  fi
}

has_tests() {
  local file="$1"
  if [[ -f "$file" ]] && rg -q '#\[test\]' "$file"; then
    echo true
  else
    echo false
  fi
}

has_runtime_wiring() {
  local file="$1"
  local pattern="$2"
  if [[ -f "$file" ]] && rg -q "$pattern" "$file"; then
    echo true
  else
    echo false
  fi
}

block_evidence() {
  local label="$1"
  shift
  local first=true
  local buf=""
  for p in "$@"; do
    if [[ -e "$p" ]]; then
      if [[ "$first" == true ]]; then
        buf="$p"
        first=false
      else
        buf="$buf, $p"
      fi
    fi
  done
  if [[ "$first" == true ]]; then
    echo "$label: not found in current workspace"
  else
    echo "$label: found paths -> $buf"
  fi
}

# Use structural checks to avoid false positives from comments/help text.
dht_code="$(has_any_path crates/chimera-dht)"
dht_api="$(has_public_api crates/chimera-dht/src/lib.rs)"
dht_tests="$(has_tests crates/chimera-dht/src/lib.rs)"
dht_runtime_wired="$(has_runtime_wiring crates/chimera-lab/src/cef_phase1_mesh.rs 'chimera_dht::|SignedDiscoveryRecord')"
dht_impl=false
if [[ "$dht_code" == true && "$dht_api" == true && "$dht_tests" == true ]]; then
  dht_impl=true
fi

dps_code="$(has_any_path crates/chimera-dps)"
dps_api="$(has_public_api crates/chimera-dps/src/lib.rs)"
dps_tests="$(has_tests crates/chimera-dps/src/lib.rs)"
dps_runtime_wired="$(has_runtime_wiring crates/chimera-lab/src/cef_phase1_mesh.rs 'chimera_dps|SignedPolicyFragment')"
dps_impl=false
if [[ "$dps_code" == true && "$dps_api" == true && "$dps_tests" == true ]]; then
  dps_impl=true
fi

mesh_code="$(has_any_path crates/chimera-mesh)"
mesh_api="$(has_public_api crates/chimera-mesh/src/lib.rs)"
mesh_tests=false
if [[ "$(has_tests crates/chimera-mesh/src/lib.rs)" == true || "$(has_tests crates/chimera-mesh/src/tests.rs)" == true ]]; then
  mesh_tests=true
elif [[ -d crates/chimera-mesh/src/tests ]] && rg -q '#\[test\]' crates/chimera-mesh/src/tests; then
  mesh_tests=true
elif [[ -f crates/chimera-mesh/src/tests/mod.rs ]] && rg -q '#\[test\]' crates/chimera-mesh/src/tests/mod.rs; then
  mesh_tests=true
fi
mesh_runtime_wired="$(has_runtime_wiring crates/chimera-lab/src/cef_phase1_mesh.rs 'evaluate_join_mode|mesh_join_mode_resolved')"
mesh_impl=false
if [[ "$mesh_code" == true && "$mesh_api" == true && "$mesh_tests" == true ]]; then
  mesh_impl=true
fi

relay_code="$(has_any_path crates/chimera-relay crates/chimera-coop-relay)"
relay_api="$(has_public_api crates/chimera-relay/src/lib.rs)"
relay_tests="$(has_tests crates/chimera-relay/src/lib.rs)"
relay_runtime_wired="$(has_runtime_wiring crates/chimera-lab/src/cef_phase1_mesh.rs 'chimera_relay::|RelayConsentMode|RelayPolicy')"
relay_impl=false
if [[ "$relay_code" == true && "$relay_api" == true && "$relay_tests" == true ]]; then
  relay_impl=true
fi

emergency_code="$(has_any_path crates/chimera-emergency crates/chimera-carrier-emergency)"
emergency_api="$(has_public_api crates/chimera-emergency/src/lib.rs)"
emergency_tests="$(has_tests crates/chimera-emergency/src/lib.rs)"
emergency_runtime_wired="$(has_runtime_wiring crates/chimera-lab/src/cef_phase1_mesh.rs 'EmergencyOffer|EmergencyCarrierKind')"
emergency_impl=false
if [[ "$emergency_code" == true && "$emergency_api" == true && "$emergency_tests" == true ]]; then
  emergency_impl=true
fi

roaming_code="$(has_any_path crates/chimera-roaming-cache crates/chimera-roaming)"
roaming_api="$(has_public_api crates/chimera-roaming/src/lib.rs)"
roaming_tests="$(has_tests crates/chimera-roaming/src/lib.rs)"
roaming_runtime_wired="$(has_runtime_wiring crates/chimera-lab/src/cef_phase1_mesh.rs 'RoamingCache|RoamingEntry')"
roaming_impl=false
if [[ "$roaming_code" == true && "$roaming_api" == true && "$roaming_tests" == true ]]; then
  roaming_impl=true
fi

reputation_code="$(has_any_path crates/chimera-reputation crates/chimera-complaint)"
reputation_api="$(has_public_api crates/chimera-reputation/src/lib.rs)"
reputation_tests="$(has_tests crates/chimera-reputation/src/lib.rs)"
reputation_runtime_wired="$(has_runtime_wiring crates/chimera-lab/src/cef_phase1_mesh.rs 'ComplaintEvidence|apply_penalty|ReputationState')"
reputation_impl=false
if [[ "$reputation_code" == true && "$reputation_api" == true && "$reputation_tests" == true ]]; then
  reputation_impl=true
fi

phase1_closed=false
if [[ "$mesh_runtime_wired" == true && "$dht_runtime_wired" == true && "$dps_runtime_wired" == true && "$relay_runtime_wired" == true && "$emergency_runtime_wired" == true && "$roaming_runtime_wired" == true && "$reputation_runtime_wired" == true ]]; then
  phase1_closed=true
fi

gate_status_for() {
  local runtime_wired="$1"
  if [[ "$runtime_wired" == true ]]; then
    echo ready
  else
    echo partial
  fi
}

mesh_gate_status="$(gate_status_for "$mesh_runtime_wired")"
dht_gate_status="$(gate_status_for "$dht_runtime_wired")"
dps_gate_status="$(gate_status_for "$dps_runtime_wired")"
relay_gate_status="$(gate_status_for "$relay_runtime_wired")"
emergency_gate_status="$(gate_status_for "$emergency_runtime_wired")"
roaming_gate_status="$(gate_status_for "$roaming_runtime_wired")"
reputation_gate_status="$(gate_status_for "$reputation_runtime_wired")"

mesh_evidence="$(block_evidence mesh_runtime crates/chimera-mesh)"
dht_evidence="$(block_evidence dht_discovery crates/chimera-dht)"
dps_evidence="$(block_evidence distributed_policy_store crates/chimera-dps)"
relay_evidence="$(block_evidence cooperative_relay_model crates/chimera-relay crates/chimera-coop-relay)"
emergency_evidence="$(block_evidence emergency_oob_carriers crates/chimera-emergency crates/chimera-carrier-emergency)"
roaming_evidence="$(block_evidence roaming_cache crates/chimera-roaming-cache crates/chimera-roaming)"
reputation_evidence="$(block_evidence reputation_complaint_credit crates/chimera-reputation crates/chimera-complaint)"

cat > "$out_path" <<JSON
{"status":"ok","kind":"cef_track_report","message_en":"Full CEF track snapshot generated.","message_ru":"Снимок трека Full CEF сформирован.","truth_boundary":{"mvp_lab_ready":true,"full_cef_closed":false},"phase1_closed":${phase1_closed},"blocks":{"mesh_runtime":{"implemented":${mesh_impl},"code":${mesh_code},"api":${mesh_api},"tests":${mesh_tests},"runtime_wired":${mesh_runtime_wired},"gate_status":"${mesh_gate_status}"},"dht_discovery":{"implemented":${dht_impl},"code":${dht_code},"api":${dht_api},"tests":${dht_tests},"runtime_wired":${dht_runtime_wired},"gate_status":"${dht_gate_status}"},"distributed_policy_store":{"implemented":${dps_impl},"code":${dps_code},"api":${dps_api},"tests":${dps_tests},"runtime_wired":${dps_runtime_wired},"gate_status":"${dps_gate_status}"},"cooperative_relay_model":{"implemented":${relay_impl},"code":${relay_code},"api":${relay_api},"tests":${relay_tests},"runtime_wired":${relay_runtime_wired},"gate_status":"${relay_gate_status}"},"emergency_oob_carriers":{"implemented":${emergency_impl},"code":${emergency_code},"api":${emergency_api},"tests":${emergency_tests},"runtime_wired":${emergency_runtime_wired},"gate_status":"${emergency_gate_status}"},"roaming_cache":{"implemented":${roaming_impl},"code":${roaming_code},"api":${roaming_api},"tests":${roaming_tests},"runtime_wired":${roaming_runtime_wired},"gate_status":"${roaming_gate_status}"},"reputation_complaint_credit":{"implemented":${reputation_impl},"code":${reputation_code},"api":${reputation_api},"tests":${reputation_tests},"runtime_wired":${reputation_runtime_wired},"gate_status":"${reputation_gate_status}"}}}
JSON

cat > "$md_path" <<MD
# CEF Track Report

Status: **PARTIAL / NOT CLOSED**

Truth boundary:
- MVP lab ready: \`true\`
- Full CEF closed: \`false\`
- Phase-1 closed: \`${phase1_closed}\`

Block status:
- mesh_runtime: \`${mesh_impl}\`
  checks: code=\`${mesh_code}\`, api=\`${mesh_api}\`, tests=\`${mesh_tests}\`
  runtime_wired: \`${mesh_runtime_wired}\`, gate_status: \`${mesh_gate_status}\`
  ${mesh_evidence}
- dht_discovery: \`${dht_impl}\`
  checks: code=\`${dht_code}\`, api=\`${dht_api}\`, tests=\`${dht_tests}\`
  runtime_wired: \`${dht_runtime_wired}\`, gate_status: \`${dht_gate_status}\`
  ${dht_evidence}
- distributed_policy_store: \`${dps_impl}\`
  checks: code=\`${dps_code}\`, api=\`${dps_api}\`, tests=\`${dps_tests}\`
  runtime_wired: \`${dps_runtime_wired}\`, gate_status: \`${dps_gate_status}\`
  ${dps_evidence}
- cooperative_relay_model: \`${relay_impl}\`
  checks: code=\`${relay_code}\`, api=\`${relay_api}\`, tests=\`${relay_tests}\`
  runtime_wired: \`${relay_runtime_wired}\`, gate_status: \`${relay_gate_status}\`
  ${relay_evidence}
- emergency_oob_carriers: \`${emergency_impl}\`
  checks: code=\`${emergency_code}\`, api=\`${emergency_api}\`, tests=\`${emergency_tests}\`
  runtime_wired: \`${emergency_runtime_wired}\`, gate_status: \`${emergency_gate_status}\`
  ${emergency_evidence}
- roaming_cache: \`${roaming_impl}\`
  checks: code=\`${roaming_code}\`, api=\`${roaming_api}\`, tests=\`${roaming_tests}\`
  runtime_wired: \`${roaming_runtime_wired}\`, gate_status: \`${roaming_gate_status}\`
  ${roaming_evidence}
- reputation_complaint_credit: \`${reputation_impl}\`
  checks: code=\`${reputation_code}\`, api=\`${reputation_api}\`, tests=\`${reputation_tests}\`
  runtime_wired: \`${reputation_runtime_wired}\`, gate_status: \`${reputation_gate_status}\`
  ${reputation_evidence}
MD

echo "CEF track report: saved to ${out_path}"
echo "CEF track report (md): saved to ${md_path}"
