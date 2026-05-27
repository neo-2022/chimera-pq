#!/usr/bin/env bash
set -euo pipefail

release_json="${1:-docs/RELEASE_READINESS_REPORT.json}"
pack_json="${2:-docs/REPORT_PACK.json}"
reality_json="${3:-docs/REALITY_AUDIT_LATEST.json}"

cargo run -q -p chimera-lab --bin release_pack_schema_guard -- \
  "$release_json" "$pack_json" "$reality_json"
