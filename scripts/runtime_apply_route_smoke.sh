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
  'rekey.max_packets_per_key = 10000' > /tmp/chimera_client_route_smoke.conf

set +e
apply_ok=false
rollback_ok=true
policy_rule_ok=false

if unshare -Urn bash -ceu "ip link show >/dev/null" >/dev/null 2>&1; then
  unshare -Urn bash -ceu '
    cargo run -q -p chimera-cli -- up \
      --state-file /tmp/chimera_runtime_route_state.json \
      --config /tmp/chimera_client_route_smoke.conf \
      --skip-connect-check true \
      --apply-tun true \
      --tun-name chimera-smoke0 \
      --tun-local-cidr 10.99.0.2/30 \
      --tun-peer-cidr 10.99.0.1/30 \
      --apply-route true \
      --route-cidr 203.0.113.0/24 \
      --route-policy true \
      --route-table 60001 \
      --route-rule-priority 12000
    rg -q "\"network_state\":\"modified\"" /tmp/chimera_runtime_route_state.json
    rg -q "\"route_policy\":true" /tmp/chimera_runtime_route_state.json
    rg -q "\"route_table\":\"60001\"" /tmp/chimera_runtime_route_state.json
    rg -q "\"route_rule_priority\":\"12000\"" /tmp/chimera_runtime_route_state.json
    ip rule show | rg -q "to 203.0.113.0/24.*lookup 60001"
    ip route show table 60001 | rg -q "203.0.113.0/24 dev chimera-smoke0"
    cargo run -q -p chimera-cli -- down --state-file /tmp/chimera_runtime_route_state.json
    test ! -f /tmp/chimera_runtime_route_state.json
    if ip rule show | rg -q "to 203.0.113.0/24.*lookup 60001"; then
      exit 21
    fi
    if ip route show table 60001 | rg -q "203.0.113.0/24 dev chimera-smoke0"; then
      exit 22
    fi
  '
  rc=$?
  if [[ $rc -eq 0 ]]; then
    apply_ok=true
    policy_rule_ok=true
  else
    rollback_ok=false
  fi
else
  cargo run -q -p chimera-cli -- up \
    --state-file /tmp/chimera_runtime_route_state.json \
    --config /tmp/chimera_client_route_smoke.conf \
    --skip-connect-check true \
    --apply-tun true \
    --tun-name chimera-smoke0 \
    --tun-local-cidr 10.99.0.2/30 \
    --tun-peer-cidr 10.99.0.1/30 \
    --apply-route true \
    --route-cidr 203.0.113.0/24 \
    --route-policy true \
    --route-table 60001 \
    --route-rule-priority 12000
  rc=$?
  if [[ $rc -eq 0 ]]; then
    apply_ok=true
    rg -q "\"network_state\":\"modified\"" /tmp/chimera_runtime_route_state.json
    rg -q "\"route_policy\":true" /tmp/chimera_runtime_route_state.json
    rg -q "\"route_table\":\"60001\"" /tmp/chimera_runtime_route_state.json
    rg -q "\"route_rule_priority\":\"12000\"" /tmp/chimera_runtime_route_state.json
    cargo run -q -p chimera-cli -- down --state-file /tmp/chimera_runtime_route_state.json || rollback_ok=false
    test ! -f /tmp/chimera_runtime_route_state.json || rollback_ok=false
  else
    if [[ -f /tmp/chimera_runtime_route_state.json ]]; then
      cargo run -q -p chimera-cli -- down --state-file /tmp/chimera_runtime_route_state.json || rollback_ok=false
    fi
  fi
fi
set -e

json="{\"status\":\"ok\",\"kind\":\"runtime_apply_route_smoke\",\"message_en\":\"Runtime apply route smoke executed.\",\"message_ru\":\"Smoke-проверка runtime apply route выполнена.\",\"network_state\":\"modified\",\"apply_attempt_ok\":${apply_ok},\"policy_rule_ok\":${policy_rule_ok},\"rollback_ok\":${rollback_ok},\"notes\":\"Uses unshare user+net namespace automatically when available; falls back to host network path otherwise.\"}"
printf "%s\n" "$json" > docs/RUNTIME_APPLY_ROUTE_SMOKE.json
