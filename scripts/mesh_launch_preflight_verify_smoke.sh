#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

VPS_JSON="/tmp/chimera_mesh_launch_preflight_vps_ready.json"
LAPTOP_JSON="/tmp/chimera_mesh_launch_preflight_laptop_ready.json"
OUT_JSON="docs/MESH_LAUNCH_PREFLIGHT_VERIFY_SMOKE.json"

cat >"$VPS_JSON" <<'EOF'
{"status":"ready","namespace":"cef-public","ready_for_real_launch":true,"connect_probe_success":true,"network_state":"not_modified","blockers":[]}
EOF

cat >"$LAPTOP_JSON" <<'EOF'
{"status":"ready","namespace":"cef-public","ready_for_real_launch":true,"connect_probe_success":true,"network_state":"not_modified","blockers":[]}
EOF

cargo run -q -p chimera-cli -- mesh launch-preflight-verify \
  --vps-report "$VPS_JSON" \
  --laptop-report "$LAPTOP_JSON" \
  --json \
  --out "$OUT_JSON"

rg -q '"status":"ready"' "$OUT_JSON"
rg -q '"all_ready":true' "$OUT_JSON"
rg -q '"vps_ready":true' "$OUT_JSON"
rg -q '"laptop_ready":true' "$OUT_JSON"
rg -q '"network_state":"not_modified"' "$OUT_JSON"

rm -f "$VPS_JSON" "$LAPTOP_JSON"
echo "mesh launch preflight verify smoke: PASS"
