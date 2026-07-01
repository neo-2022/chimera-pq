#!/usr/bin/env bash
set -euo pipefail

cargo run -q -p chimera-lab --bin ai_architect_artifact_guard -- "$@"
