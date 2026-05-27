#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

VPS_JSON="/tmp/chimera_mesh_launch_preflight_vps_smoke.json"
LAPTOP_JSON="/tmp/chimera_mesh_launch_preflight_laptop_smoke.json"
VERIFY_JSON="/tmp/chimera_mesh_launch_preflight_verify_smoke.json"

cleanup() {
  rm -f "$VPS_JSON" "$LAPTOP_JSON" "$VERIFY_JSON"
}
trap cleanup EXIT

cat >"$VPS_JSON" <<'EOF'
{"status":"ready","network_state":"not_modified","namespace":"cef-public","node":"node-a","timeout_ms":1200,"ready_for_real_launch":true,"blockers":[],"selected_peers":["node-b"],"connected_peer":"node-b","connected_endpoint":"127.0.0.1:443","connect_probe_success":true,"attempts":[{"peer_id":"node-b","endpoint":"127.0.0.1:443","success":true,"error":""}],"explain":["connect probe reached node-b via endpoint 127.0.0.1:443"]}
EOF

cat >"$LAPTOP_JSON" <<'EOF'
{"status":"ready","network_state":"not_modified","namespace":"cef-public","node":"node-b","timeout_ms":1200,"ready_for_real_launch":true,"blockers":[],"selected_peers":["node-a"],"connected_peer":"node-a","connected_endpoint":"127.0.0.1:443","connect_probe_success":true,"attempts":[{"peer_id":"node-a","endpoint":"127.0.0.1:443","success":true,"error":""}],"explain":["connect probe reached node-a via endpoint 127.0.0.1:443"]}
EOF

cargo run -q -p chimera-cli -- mesh launch-preflight-verify \
  --vps-report "$VPS_JSON" \
  --laptop-report "$LAPTOP_JSON" \
  --json \
  --out "$VERIFY_JSON"

CHIMERA_MESH_PREFLIGHT_MAX_AGE_SEC=300 \
CHIMERA_MESH_PREFLIGHT_VPS_JSON="$VPS_JSON" \
CHIMERA_MESH_PREFLIGHT_LAPTOP_JSON="$LAPTOP_JSON" \
CHIMERA_MESH_PREFLIGHT_VERIFY_JSON="$VERIFY_JSON" \
just mesh-launch-preflight-evidence-guard

cleanup
if [[ -e "$VPS_JSON" || -e "$LAPTOP_JSON" || -e "$VERIFY_JSON" ]]; then
  echo "mesh launch preflight evidence smoke: temp artifact cleanup failed"
  exit 1
fi
trap - EXIT

echo "mesh launch preflight evidence smoke: PASS"
