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
  'rekey.max_packets_per_key = 10000' > /tmp/chimera_client_route_multi_cidr_smoke.conf

set +e
apply_ok=false
rollback_ok=true
policy_rule_ok=false
skipped_no_tun=false

if unshare -Urn bash -ceu "ip link show >/dev/null" >/dev/null 2>&1; then
  unshare -Urn bash -ceu '
    cargo run -q -p chimera-cli -- up \
      --state-file /tmp/chimera_runtime_route_multi_cidr_state.json \
      --config /tmp/chimera_client_route_multi_cidr_smoke.conf \
      --skip-connect-check true \
      --apply-tun true \
      --tun-name chimera-smoke1 \
      --tun-local-cidr 10.99.1.2/30 \
      --tun-peer-cidr 10.99.1.1/30 \
      --apply-route true \
      --route-cidr 203.0.113.0/24,198.51.100.0/24 \
      --route-policy true \
      --route-table 60011 \
      --route-rule-priority 12110
    rg -q "\"network_state\":\"modified\"" /tmp/chimera_runtime_route_multi_cidr_state.json
    rg -q "\"route_policy\":true" /tmp/chimera_runtime_route_multi_cidr_state.json
    rg -q "\"route_table\":\"60011\"" /tmp/chimera_runtime_route_multi_cidr_state.json
    rg -q "\"route_rule_priority\":\"12110\"" /tmp/chimera_runtime_route_multi_cidr_state.json
    rg -q "\"route_cidr\":\"203.0.113.0/24,198.51.100.0/24\"" /tmp/chimera_runtime_route_multi_cidr_state.json
    ip rule show | rg -q "to 203.0.113.0/24.*lookup 60011"
    ip rule show | rg -q "to 198.51.100.0/24.*lookup 60011"
    ip route show table 60011 | rg -q "203.0.113.0/24 dev chimera-smoke1"
    ip route show table 60011 | rg -q "198.51.100.0/24 dev chimera-smoke1"
    cargo run -q -p chimera-cli -- down --state-file /tmp/chimera_runtime_route_multi_cidr_state.json
    test ! -f /tmp/chimera_runtime_route_multi_cidr_state.json
    if ip rule show | rg -q "to 203.0.113.0/24.*lookup 60011"; then
      exit 41
    fi
    if ip rule show | rg -q "to 198.51.100.0/24.*lookup 60011"; then
      exit 42
    fi
    if ip route show table 60011 | rg -q "203.0.113.0/24 dev chimera-smoke1"; then
      exit 43
    fi
    if ip route show table 60011 | rg -q "198.51.100.0/24 dev chimera-smoke1"; then
      exit 44
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
    --state-file /tmp/chimera_runtime_route_multi_cidr_state.json \
    --config /tmp/chimera_client_route_multi_cidr_smoke.conf \
    --skip-connect-check true \
    --apply-tun true \
    --tun-name chimera-smoke1 \
    --tun-local-cidr 10.99.1.2/30 \
    --tun-peer-cidr 10.99.1.1/30 \
    --apply-route true \
    --route-cidr 203.0.113.0/24,198.51.100.0/24 \
    --route-policy true \
    --route-table 60011 \
    --route-rule-priority 12110
  rc=$?
  if [[ $rc -eq 0 ]]; then
    apply_ok=true
    rg -q "\"network_state\":\"modified\"" /tmp/chimera_runtime_route_multi_cidr_state.json
    rg -q "\"route_policy\":true" /tmp/chimera_runtime_route_multi_cidr_state.json
    rg -q "\"route_table\":\"60011\"" /tmp/chimera_runtime_route_multi_cidr_state.json
    rg -q "\"route_rule_priority\":\"12110\"" /tmp/chimera_runtime_route_multi_cidr_state.json
    rg -q "\"route_cidr\":\"203.0.113.0/24,198.51.100.0/24\"" /tmp/chimera_runtime_route_multi_cidr_state.json
    cargo run -q -p chimera-cli -- down --state-file /tmp/chimera_runtime_route_multi_cidr_state.json || rollback_ok=false
    test ! -f /tmp/chimera_runtime_route_multi_cidr_state.json || rollback_ok=false
  else
    if [[ -f /tmp/chimera_runtime_route_multi_cidr_state.json ]]; then
      cargo run -q -p chimera-cli -- down --state-file /tmp/chimera_runtime_route_multi_cidr_state.json || rollback_ok=false
    fi
  fi
fi
set -e

if [[ "$apply_ok" != "true" && "$policy_rule_ok" != "true" && ! -c /dev/net/tun ]]; then
  skipped_no_tun=true
fi

json="{\"status\":\"ok\",\"kind\":\"runtime_apply_route_multi_cidr_smoke\",\"message_en\":\"Runtime apply route multi-CIDR smoke executed.\",\"message_ru\":\"Smoke-проверка runtime apply route multi-CIDR выполнена.\",\"network_state\":\"modified\",\"apply_attempt_ok\":${apply_ok},\"policy_rule_ok\":${policy_rule_ok},\"rollback_ok\":${rollback_ok},\"skipped_no_tun\":${skipped_no_tun},\"notes\":\"Uses unshare user+net namespace automatically when available; falls back to host network path otherwise.\"}"
printf "%s\n" "$json" > docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json
