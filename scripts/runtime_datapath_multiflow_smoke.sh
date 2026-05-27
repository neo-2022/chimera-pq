#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

tmp_json="/tmp/chimera_runtime_datapath_multiflow_tmp.json"
rm -f "$tmp_json"

cargo run -q -p chimera-lab --bin chimera-lab -- datapath-report --json --out "$tmp_json"

gateway_ok=false
block_ok=false
direct_ok=false
if rg -q '"gateway_explain":"matched rule' "$tmp_json"; then
  gateway_ok=true
fi
if rg -q '"block_explain":"matched rule' "$tmp_json"; then
  block_ok=true
fi
if rg -q '"direct_explain":"matched rule' "$tmp_json"; then
  direct_ok=true
fi

json="{\"status\":\"ok\",\"kind\":\"runtime_datapath_multiflow_smoke\",\"message_en\":\"Runtime datapath multiflow smoke executed.\",\"message_ru\":\"Smoke-проверка runtime datapath multiflow выполнена.\",\"network_state\":\"not_modified\",\"gateway_ok\":${gateway_ok},\"block_ok\":${block_ok},\"direct_ok\":${direct_ok}}"
printf "%s\n" "$json" > docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json

if [[ "$gateway_ok" != "true" || "$block_ok" != "true" || "$direct_ok" != "true" ]]; then
  echo "runtime datapath multiflow smoke: FAIL" >&2
  exit 1
fi

echo "runtime datapath multiflow smoke: PASS"
