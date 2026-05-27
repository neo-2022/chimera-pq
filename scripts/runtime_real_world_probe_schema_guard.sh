#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

probe_json="${1:-docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json}"

cargo run -q -p chimera-lab --bin runtime_real_world_probe_schema_guard -- "$probe_json"
