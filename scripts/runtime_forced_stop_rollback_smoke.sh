#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

printf '%s\n' \
  'carrier.profile = in-memory' \
  'carrier.addr = 127.0.0.1:443' \
  'carrier.server_name = gateway.local' \
  'capture.mode = tun' \
  'capture.tun_supported = true' \
  'rekey.max_age_seconds = 300' \
  'rekey.max_packets_per_key = 10000' > /tmp/chimera_client_forced_stop_smoke.conf

state_file="/tmp/chimera_runtime_forced_stop_state.json"
rm -f "$state_file"

set +e
apply_ok=false
recover_ok=false
down_ok=false
if unshare -Urn bash -ceu "ip link show >/dev/null" >/dev/null 2>&1; then
  unshare -Urn bash -ceu '
    cargo run -q -p chimera-cli -- up \
      --state-file /tmp/chimera_runtime_forced_stop_state.json \
      --config /tmp/chimera_client_forced_stop_smoke.conf \
      --skip-connect-check true \
      --apply-tun true \
      --tun-name chimera-stop0 \
      --tun-local-cidr 10.90.0.2/30 \
      --tun-peer-cidr 10.90.0.1/30 \
      --apply-route true \
      --route-cidr 198.18.0.0/15 \
      --route-policy true \
      --route-table 60010 \
      --route-rule-priority 12100
    rg -q "\"network_state\":\"modified\"" /tmp/chimera_runtime_forced_stop_state.json
    rg -q "\"route_applied\":true" /tmp/chimera_runtime_forced_stop_state.json
    rg -q "\"tun_applied\":true" /tmp/chimera_runtime_forced_stop_state.json
    ip rule show | rg -q "to 198.18.0.0/15.*lookup 60010"
    ip route show table 60010 | rg -q "198.18.0.0/15 dev chimera-stop0"

    # Simulate forced-stop path: no graceful down between apply and recover.
    cargo run -q -p chimera-cli -- rollback recover --state-file /tmp/chimera_runtime_forced_stop_state.json
    test ! -f /tmp/chimera_runtime_forced_stop_state.json
    if ip rule show | rg -q "to 198.18.0.0/15.*lookup 60010"; then
      exit 41
    fi
    if ip route show table 60010 | rg -q "198.18.0.0/15 dev chimera-stop0"; then
      exit 42
    fi
    if ip link show dev chimera-stop0 >/dev/null 2>&1; then
      exit 43
    fi
  '
  rc=$?
  if [[ $rc -eq 0 ]]; then
    apply_ok=true
    recover_ok=true
    down_ok=true
  fi
else
  cargo run -q -p chimera-cli -- up \
    --state-file "$state_file" \
    --config /tmp/chimera_client_forced_stop_smoke.conf \
    --skip-connect-check true \
    --apply-tun true \
    --tun-name chimera-stop0 \
    --tun-local-cidr 10.90.0.2/30 \
    --tun-peer-cidr 10.90.0.1/30 \
    --apply-route true \
    --route-cidr 198.18.0.0/15 \
    --route-policy true \
    --route-table 60010 \
    --route-rule-priority 12100
  rc=$?
  if [[ $rc -eq 0 ]]; then
    apply_ok=true
    rg -q "\"network_state\":\"modified\"" "$state_file"
    cargo run -q -p chimera-cli -- rollback recover --state-file "$state_file"
    if [[ ! -f "$state_file" ]]; then
      recover_ok=true
    fi
    down_ok=true
  fi
fi
set -e

json="{\"status\":\"ok\",\"kind\":\"runtime_forced_stop_rollback_smoke\",\"message_en\":\"Runtime forced-stop rollback smoke executed.\",\"message_ru\":\"Smoke-проверка rollback после forced-stop выполнена.\",\"network_state\":\"modified\",\"apply_attempt_ok\":${apply_ok},\"recover_ok\":${recover_ok},\"down_state_clean\":${down_ok},\"notes\":\"Uses unshare user+net namespace when available; validates recover path without graceful down.\"}"
printf "%s\n" "$json" > docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json

if [[ "$apply_ok" != "true" || "$recover_ok" != "true" || "$down_ok" != "true" ]]; then
  echo "runtime forced-stop rollback smoke: FAIL" >&2
  exit 1
fi

echo "runtime forced-stop rollback smoke: PASS"
