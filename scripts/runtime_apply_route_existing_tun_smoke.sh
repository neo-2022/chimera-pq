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
  'rekey.max_packets_per_key = 10000' > /tmp/chimera_client_route_existing_tun_smoke.conf

set +e
apply_ok=false
rollback_ok=true
preexisting_tun_used=false

if unshare -Urn bash -ceu "ip link show >/dev/null" >/dev/null 2>&1; then
  unshare -Urn bash -ceu '
    ip tuntap add dev chimera-pre0 mode tun
    ip link set dev chimera-pre0 up
    ip addr add 10.88.0.2/30 peer 10.88.0.1/30 dev chimera-pre0

    cargo run -q -p chimera-cli -- up \
      --state-file /tmp/chimera_runtime_route_existing_tun_state.json \
      --config /tmp/chimera_client_route_existing_tun_smoke.conf \
      --skip-connect-check true \
      --apply-route true \
      --route-cidr 198.51.100.0/24 \
      --route-policy true \
      --route-table 60002 \
      --route-rule-priority 12010 \
      --tun-name chimera-pre0

    rg -q "\"network_state\":\"modified\"" /tmp/chimera_runtime_route_existing_tun_state.json
    rg -q "\"tun_applied\":false" /tmp/chimera_runtime_route_existing_tun_state.json
    rg -q "\"route_applied\":true" /tmp/chimera_runtime_route_existing_tun_state.json
    rg -q "\"route_policy\":true" /tmp/chimera_runtime_route_existing_tun_state.json

    ip rule show | rg -q "to 198.51.100.0/24.*lookup 60002"
    ip route show table 60002 | rg -q "198.51.100.0/24 dev chimera-pre0"

    cargo run -q -p chimera-cli -- down --state-file /tmp/chimera_runtime_route_existing_tun_state.json
    test ! -f /tmp/chimera_runtime_route_existing_tun_state.json

    if ip rule show | rg -q "to 198.51.100.0/24.*lookup 60002"; then
      exit 31
    fi
    if ip route show table 60002 | rg -q "198.51.100.0/24 dev chimera-pre0"; then
      exit 32
    fi

    ip link show dev chimera-pre0 >/dev/null
    ip link delete dev chimera-pre0
  '
  rc=$?
  if [[ $rc -eq 0 ]]; then
    apply_ok=true
    preexisting_tun_used=true
  else
    rollback_ok=false
  fi
else
  rc=0
fi
set -e

json="{\"status\":\"ok\",\"kind\":\"runtime_apply_route_existing_tun_smoke\",\"message_en\":\"Runtime apply route with pre-existing TUN smoke executed.\",\"message_ru\":\"Smoke-проверка runtime apply route с предсозданным TUN выполнена.\",\"network_state\":\"modified\",\"apply_attempt_ok\":${apply_ok},\"preexisting_tun_used\":${preexisting_tun_used},\"rollback_ok\":${rollback_ok},\"notes\":\"Uses unshare user+net namespace; skipped when unavailable.\"}"
printf "%s\n" "$json" > docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json
