#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

SIDE_A_JSON="/tmp/chimera_mesh_launch_preflight_side_a_smoke.json"
SIDE_B_JSON="/tmp/chimera_mesh_launch_preflight_side_b_smoke.json"
VERIFY_JSON="/tmp/chimera_mesh_launch_preflight_verify_smoke.json"

cleanup() {
  rm -f "$SIDE_A_JSON" "$SIDE_B_JSON" "$VERIFY_JSON"
}
trap cleanup EXIT

cat >"$SIDE_A_JSON" <<'EOF'
{"status":"ready","network_state":"not_modified","namespace":"cef-public","node":"<redacted>","timeout_ms":1200,"ready_for_real_launch":true,"blockers":[],"selected_peers":["peer#1"],"connected_peer":"peer#1","connected_endpoint":"endpoint#1:<redacted>","connect_probe_success":true,"attempts":[{"peer_id":"peer#1","endpoint":"endpoint#1:<redacted>","success":true,"error":""}],"explain":["connect_probe_connected_peer=peer#1","connect_probe_connected_endpoint=endpoint#1:<redacted>"]}
EOF

cat >"$SIDE_B_JSON" <<'EOF'
{"status":"ready","network_state":"not_modified","namespace":"cef-public","node":"<redacted>","timeout_ms":1200,"ready_for_real_launch":true,"blockers":[],"selected_peers":["peer#1"],"connected_peer":"peer#1","connected_endpoint":"endpoint#1:<redacted>","connect_probe_success":true,"attempts":[{"peer_id":"peer#1","endpoint":"endpoint#1:<redacted>","success":true,"error":""}],"explain":["connect_probe_connected_peer=peer#1","connect_probe_connected_endpoint=endpoint#1:<redacted>"]}
EOF

cargo run -q -p chimera-cli -- mesh launch-preflight-verify \
  --side-a-report "$SIDE_A_JSON" \
  --side-b-report "$SIDE_B_JSON" \
  --json \
  --out "$VERIFY_JSON"

CHIMERA_MESH_PREFLIGHT_MAX_AGE_SEC=300 \
CHIMERA_MESH_PREFLIGHT_SIDE_A_JSON="$SIDE_A_JSON" \
CHIMERA_MESH_PREFLIGHT_SIDE_B_JSON="$SIDE_B_JSON" \
CHIMERA_MESH_PREFLIGHT_VERIFY_JSON="$VERIFY_JSON" \
just mesh-launch-preflight-evidence-guard

cleanup
if [[ -e "$SIDE_A_JSON" || -e "$SIDE_B_JSON" || -e "$VERIFY_JSON" ]]; then
  echo "mesh launch preflight evidence smoke: temp artifact cleanup failed"
  exit 1
fi
trap - EXIT

echo "mesh launch preflight evidence smoke: PASS"
