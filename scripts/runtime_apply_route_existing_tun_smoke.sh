#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/runtime_route_netns_lib.sh"

printf '%s\n' \
  'carrier.profile = in-memory' \
  'carrier.addr = 127.0.0.1:443' \
  'carrier.server_name = gateway.local' \
  'capture.mode = tun' \
  'capture.tun_supported = true' \
  'rekey.max_age_seconds = 300' \
  'rekey.max_packets_per_key = 10000' > /tmp/chimera_client_route_existing_tun_smoke.conf

runtime_route_build_cli

set +e
apply_ok=false
rollback_ok=true
preexisting_tun_used=false
skipped_no_tun=false
skip_reason="none"
status="fail"
network_state="not_modified"
counts_for_release=false
namespace_mode="none"

write_artifact() {
  local notes="${1:?notes_required}"
  json="{\"status\":\"${status}\",\"kind\":\"runtime_apply_route_existing_tun_smoke\",\"message_en\":\"Runtime apply route with pre-existing TUN smoke executed.\",\"message_ru\":\"Smoke-проверка runtime apply route с предсозданным TUN выполнена.\",\"network_state\":\"${network_state}\",\"apply_attempt_ok\":${apply_ok},\"preexisting_tun_used\":${preexisting_tun_used},\"rollback_ok\":${rollback_ok},\"skipped_no_tun\":${skipped_no_tun},\"skip_reason\":\"${skip_reason}\",\"counts_for_release\":${counts_for_release},\"namespace_mode\":\"${namespace_mode}\",\"notes\":\"${notes}\"}"
  printf "%s\n" "$json" > docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json
}

probe='ip tuntap add dev chimera-probe0 mode tun; ip link delete dev chimera-probe0'
if runtime_route_select_netns "$probe"; then
  namespace_mode="$CHIMERA_ROUTE_NETNS_MODE"
  runtime_route_run_netns '
    ip tuntap add dev chimera-pre0 mode tun
    ip link set dev chimera-pre0 up
    ip addr add 10.88.0.2/30 peer 10.88.0.1/30 dev chimera-pre0

    "$CHIMERA_CLI_BIN" up \
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

    "$CHIMERA_CLI_BIN" down --state-file /tmp/chimera_runtime_route_existing_tun_state.json
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

write_artifact "Uses unshare user+net namespace, or CI-gated sudo net namespace, with a real TUN creation probe; host network fallback is forbidden."

if [[ "$status" != "ok" ]]; then
  echo "runtime apply route existing-TUN smoke: ${status} (${skip_reason})" >&2
  exit 1
fi

echo "runtime apply route existing-TUN smoke: PASS"
