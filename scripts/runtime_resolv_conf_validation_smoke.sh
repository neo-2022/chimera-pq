#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

state_file="/tmp/chimera_runtime_resolv_validation_state.json"
rm -f "$state_file"

set +e
cargo run -q -p chimera-cli -- up \
  --state-file "$state_file" \
  --skip-connect-check true \
  --apply-dns true \
  --dns-server 9.9.9.9 \
  --resolv-conf relative/not-allowed.conf
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

json="{\"status\":\"ok\",\"kind\":\"runtime_resolv_conf_validation_smoke\",\"message_en\":\"Runtime resolv.conf path validation smoke executed.\",\"message_ru\":\"Smoke-проверка валидации пути resolv.conf выполнена.\",\"network_state\":\"not_modified\",\"apply_rejected\":${apply_rejected},\"state_not_created\":${state_not_created}}"
printf "%s\n" "$json" > docs/RUNTIME_RESOLV_CONF_VALIDATION_SMOKE.json

if [[ "$apply_rejected" != "true" || "$state_not_created" != "true" ]]; then
  echo "runtime resolv.conf validation smoke: FAIL" >&2
  exit 1
fi

echo "runtime resolv.conf validation smoke: PASS"
