#!/usr/bin/env bash
set -euo pipefail

release_json="${1:-docs/RELEASE_READINESS_REPORT.json}"
release_ru_md="${2:-docs/RELEASE_READINESS_REPORT_RU.md}"
reality_json="${3:-docs/REALITY_AUDIT_LATEST.json}"

cargo run -q -p chimera-lab --bin release_ru_guard -- \
  "$release_json" "$release_ru_md" "$reality_json"
