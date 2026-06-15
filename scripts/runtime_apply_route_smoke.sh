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
skipped_no_tun=false
skip_reason="none"
status="fail"
network_state="not_modified"
counts_for_release=false

write_artifact() {
  local notes="${1:?notes_required}"
  json="{\"status\":\"${status}\",\"kind\":\"runtime_apply_route_smoke\",\"message_en\":\"Runtime apply route smoke executed.\",\"message_ru\":\"Smoke-проверка runtime apply route выполнена.\",\"network_state\":\"${network_state}\",\"apply_attempt_ok\":${apply_ok},\"policy_rule_ok\":${policy_rule_ok},\"rollback_ok\":${rollback_ok},\"skipped_no_tun\":${skipped_no_tun},\"skip_reason\":\"${skip_reason}\",\"counts_for_release\":${counts_for_release},\"notes\":\"${notes}\"}"
  printf "%s\n" "$json" > docs/RUNTIME_APPLY_ROUTE_SMOKE.json
}

if unshare -Urn bash -ceu "ip tuntap add dev chimera-probe0 mode tun; ip link delete dev chimera-probe0" >/dev/null 2>&1; then
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
    status="ok"
    network_state="modified"
    counts_for_release=true
  else
    rollback_ok=false
  fi
else
  skipped_no_tun=true
  skip_reason="tun_permission_unavailable"
  status="skipped"
fi
set -e

write_artifact "Uses unshare user+net namespace with a real TUN creation probe; host network fallback is forbidden."

if [[ "$status" != "ok" ]]; then
  echo "runtime apply route smoke: ${status} (${skip_reason})" >&2
  exit 1
fi

echo "runtime apply route smoke: PASS"
