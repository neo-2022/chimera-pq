#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

SIDE_A_JSON="/tmp/chimera_mesh_launch_preflight_side_a_ready.json"
SIDE_B_JSON="/tmp/chimera_mesh_launch_preflight_side_b_ready.json"
OUT_JSON="docs/MESH_LAUNCH_PREFLIGHT_VERIFY_SMOKE.json"

cat >"$SIDE_A_JSON" <<'EOF'
{"status":"ready","namespace":"cef-public","ready_for_real_launch":true,"connect_probe_success":true,"network_state":"not_modified","blockers":[]}
EOF

cat >"$SIDE_B_JSON" <<'EOF'
{"status":"ready","namespace":"cef-public","ready_for_real_launch":true,"connect_probe_success":true,"network_state":"not_modified","blockers":[]}
EOF

cargo run -q -p chimera-cli -- mesh launch-preflight-verify \
  --side-a-report "$SIDE_A_JSON" \
  --side-b-report "$SIDE_B_JSON" \
  --json \
  --out "$OUT_JSON"

rg -q '"status":"ready"' "$OUT_JSON"
rg -q '"all_ready":true' "$OUT_JSON"
rg -q '"side_a_ready":true' "$OUT_JSON"
rg -q '"side_b_ready":true' "$OUT_JSON"
rg -q '"network_state":"not_modified"' "$OUT_JSON"

rm -f "$SIDE_A_JSON" "$SIDE_B_JSON"
echo "mesh launch preflight verify smoke: PASS"
