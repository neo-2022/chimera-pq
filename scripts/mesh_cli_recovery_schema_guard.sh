#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

cargo test -q -p chimera-cli tests_json_operator_cross_contract
cargo test -q -p chimera-cli tests_json_success_presence
cargo test -q -p chimera-cli tests_json_error_contract

# Success envelope artifact
cargo run -q -p chimera-cli -- mesh route-explain --namespace cef-public --node node-client --invite-token inv-1 --policy-payload 'allow=mesh;mesh_allowed_regions=eu;mesh_min_reliability=70;mesh_max_load=60;mesh_max_peers=1' --peer 'node-eu-1@198.51.100.7:443@eu@20@92' --peer 'node-eu-2@198.51.100.8:443@eu@40@88' --failed-node node-eu-1 --cooldown-node node-eu-2 --json --out docs/MESH_ROUTE_EXPLAIN.json
# Error envelope artifact
set +e
cargo run -q -p chimera-cli -- mesh route-explain --namespace cef-public --node node-client --invite-token inv-1 --policy-payload 'mesh_max_peers=0' --peer 'node-eu-1@198.51.100.7:443@eu@20@92' --json --out docs/MESH_ROUTE_EXPLAIN_ERROR.json
error_rc=$?
set -e
if [[ "$error_rc" -ne 2 ]]; then
  echo "mesh cli recovery schema guard: expected mesh route-explain error exit code 2, got ${error_rc}" >&2
  exit 1
fi

cargo run -q -p chimera-lab --bin mesh_cli_recovery_schema_guard -- docs/MESH_ROUTE_EXPLAIN.json
cargo run -q -p chimera-lab --bin mesh_cli_recovery_schema_guard -- docs/MESH_ROUTE_EXPLAIN_ERROR.json

echo "mesh cli recovery schema guard: PASS"
