#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

state_file="/tmp/chimera_runtime_tun_name_validation_state.json"
rm -f "$state_file"

set +e
cargo run -q -p chimera-cli -- up \
  --state-file "$state_file" \
  --skip-connect-check true \
  --apply-route true \
  --route-policy true \
  --route-table 60003 \
  --route-rule-priority 12020 \
  --tun-name bad@if
rc=$?
set -e

apply_rejected=false
state_not_created=false
if [[ $rc -eq 2 ]]; then
  apply_rejected=true
fi
if [[ ! -f "$state_file" ]]; then
  state_not_created=true
fi

json="{\"status\":\"ok\",\"kind\":\"runtime_tun_name_validation_smoke\",\"message_en\":\"Runtime TUN name validation smoke executed.\",\"message_ru\":\"Smoke-проверка валидации TUN-имени выполнена.\",\"network_state\":\"not_modified\",\"apply_rejected\":${apply_rejected},\"state_not_created\":${state_not_created}}"
printf "%s\n" "$json" > docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json

if [[ "$apply_rejected" != "true" || "$state_not_created" != "true" ]]; then
  echo "runtime tun name validation smoke: FAIL" >&2
  exit 1
fi

echo "runtime tun name validation smoke: PASS"
