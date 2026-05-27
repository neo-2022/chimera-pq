#!/usr/bin/env bash
set -euo pipefail

reality_json="${1:-docs/REALITY_AUDIT_LATEST.json}"

cargo run -q -p chimera-lab --bin reality_audit_schema_guard -- "$reality_json"
