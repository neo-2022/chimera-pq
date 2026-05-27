#!/usr/bin/env bash
set -euo pipefail

ship_json="${1:-docs/SHIP_READINESS_REPORT.json}"
release_json="${2:-docs/RELEASE_READINESS_REPORT.json}"
pack_json="${3:-docs/REPORT_PACK.json}"
rt_dns_json="${4:-docs/RUNTIME_APPLY_DNS_SMOKE.json}"
rt_route_json="${5:-docs/RUNTIME_APPLY_ROUTE_SMOKE.json}"
rt_route_multi_cidr_json="${6:-docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json}"
rt_forced_stop_json="${7:-docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json}"
rt_real_world_probe_json="${8:-docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json}"
reality_json="${9:-docs/REALITY_AUDIT_LATEST.json}"

cargo run -q -p chimera-lab --bin ship_nonregression_guard -- \
  "$ship_json" \
  "$release_json" \
  "$pack_json" \
  "$rt_dns_json" \
  "$rt_route_json" \
  "$rt_route_multi_cidr_json" \
  "$rt_forced_stop_json" \
  "$rt_real_world_probe_json" \
  "$reality_json"
