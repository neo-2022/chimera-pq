#!/usr/bin/env bash
set -euo pipefail

reality_json="${1:-docs/REALITY_AUDIT_LATEST.json}"
ship_json="${2:-docs/SHIP_READINESS_REPORT.json}"
release_json="${3:-docs/RELEASE_READINESS_REPORT.json}"
pack_json="${4:-docs/REPORT_PACK.json}"
snapshot_json="${5:-docs/MVP_SNAPSHOT.json}"
verify_json="${6:-docs/MVP_VERIFY.json}"
release_audit_json="${7:-docs/release_readiness_audit.json}"
ship_md="${8:-docs/SHIP_READINESS_REPORT.md}"
pack_md="${9:-docs/REPORT_PACK.md}"
rt_real_world_probe_json="${10:-docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json}"

cargo run -q -p chimera-lab --bin reality_truth_guard -- \
  "$reality_json" \
  "$ship_json" \
  "$release_json" \
  "$pack_json" \
  "$snapshot_json" \
  "$verify_json" \
  "$release_audit_json" \
  "$ship_md" \
  "$pack_md" \
  "$rt_real_world_probe_json"
