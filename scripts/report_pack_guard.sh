#!/usr/bin/env bash
set -euo pipefail

pack_json="${1:-docs/REPORT_PACK.json}"
pack_md="${2:-docs/REPORT_PACK.md}"
reality_json="${3:-docs/REALITY_AUDIT_LATEST.json}"

cargo run -q -p chimera-lab --bin report_pack_guard -- \
  "$pack_json" "$pack_md" "$reality_json"
