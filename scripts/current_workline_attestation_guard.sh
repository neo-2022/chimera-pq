#!/usr/bin/env bash
set -euo pipefail

attestation_json="${1:-docs/CURRENT_WORKLINE_ATTESTATION.json}"

cargo run -q -p chimera-lab --bin current_workline_attestation_guard -- "$attestation_json"
