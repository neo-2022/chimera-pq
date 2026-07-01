#!/usr/bin/env bash
set -euo pipefail

attestation_json="${1:-docs/WORKFLOW_ATTESTATION.json}"

cargo run -q -p chimera-lab --bin workflow_attestation_guard -- "$attestation_json"
