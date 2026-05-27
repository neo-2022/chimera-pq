#!/usr/bin/env bash
set -euo pipefail

snapshot_json="${1:-docs/MVP_SNAPSHOT.json}"
verify_json="${2:-docs/MVP_VERIFY.json}"
reality_json="${3:-docs/REALITY_AUDIT_LATEST.json}"

cargo run -q -p chimera-lab --bin mvp_snapshot_verify_guard -- \
  "$snapshot_json" "$verify_json" "$reality_json"
