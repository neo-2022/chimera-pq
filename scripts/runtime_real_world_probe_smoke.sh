#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

probe_mode="${CHIMERA_REAL_WORLD_PROBE_MODE:-live}"
case "$probe_mode" in
  live|ci_snapshot) ;;
  *)
    echo "runtime real-world probe smoke: invalid CHIMERA_REAL_WORLD_PROBE_MODE, expected live or ci_snapshot" >&2
    exit 2
    ;;
esac

cargo run -q -p chimera-lab --bin runtime_real_world_probe
