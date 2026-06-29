# Workflow Attestation: CLI Endpoint Generation Consumption

## Scope

- Date: 2026-06-29
- Slice: metadata/control-plane endpoint publish path.
- Status: lab/control-plane evidence only.
- Network state: not modified.
- Transit payload policy: opaque/sealed payload untouched.

## Implemented

- `chimera-cli` now builds validated `MeshPublishedEndpointUpdate` records from
  inventory nodes that carry optional `endpoint_generation`.
- `mesh nodes probe --all` merges validated published endpoint updates into
  `MeshRuntime::merge_published_endpoint_updates(...)` after discovery merge and
  before connect probe planning.
- Legacy bootstrap/inventory paths keep `endpoint_generation = None`.
- Config and signed discovery parsing preserve positive `endpoint_generation`
  and reject zero/invalid values.
- Advertise snapshots include `endpoint_generation` when it is present in the
  private peer update state.
- CLI adapter proof covers runtime consumption and planner visibility of the
  newer endpoint in an in-memory model.

## Role Council Summary

- Architect: keep `MeshDiscoveryRecord` unchanged; use
  `MeshPublishedEndpointUpdate` as the runtime consumption boundary.
- Senior developer: keep the adapter in CLI/inventory, not in bootstrap or
  diagnostics; preserve legacy compatibility.
- Tester: require legacy, positive, invalid, discovery, advertise, adapter and
  runtime consumption tests.
- Security: fail closed on invalid generation/endpoint/update URL and keep
  diagnostics redacted.
- DevOps/release: lab-only; do not claim SSH, install/update or runtime PASS.
- Critic: do not count field propagation as PASS unless the applied runtime
  state is consumed by planning.

## Validation

PASS:

- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `cargo check -q -p chimera-cli`
- `cargo test -q -p chimera-cli nodes_inventory_config_accepts_endpoint_generation -- --nocapture`
- `cargo test -q -p chimera-cli nodes_inventory_config_rejects_zero_endpoint_generation -- --nocapture`
- `cargo test -q -p chimera-cli nodes_inventory_config_rejects_invalid_endpoint_generation -- --nocapture`
- `cargo test -q -p chimera-cli nodes_inventory_endpoint_generation_builds_published_update_consumed_by_runtime_plan -- --nocapture`
- `cargo test -q -p chimera-cli nodes_inventory_discovery_contract_preserves_endpoint_generation -- --nocapture`
- `cargo test -q -p chimera-cli nodes_inventory_discovery_contract_rejects_zero_endpoint_generation -- --nocapture`
- `cargo test -q -p chimera-cli nodes_advertise_publishes_runtime_endpoint_and_update_state_together -- --nocapture`
- `cargo test -q -p chimera-cli nodes_probe_all_uses_connect_probe_backend -- --nocapture`
- `cargo test -q -p chimera-cli tests_nodes_inventory -- --nocapture`
- `cargo test -q -p chimera-cli tests_nodes_runtime_state -- --nocapture`
- `cargo test -q -p chimera-cli`
- `cargo test -q -p chimera-mesh tests_discovery_merge::published_endpoint_update -- --nocapture`
- `cargo test -q -p chimera-mesh tests_discovery_merge -- --nocapture`
- `cargo test -q -p chimera-mesh`
- `cargo clippy -q -p chimera-cli --all-targets -- -D warnings`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `just rust-no-hardcode-guard-selfcheck`
- `just release-pack-schema-guard-selfcheck`
- `just ship-structure-guard-selfcheck`
- `./scripts/release_pack_schema_guard.sh`
- `./scripts/ship_structure_guard.sh`
- `git diff --check -- <touched files>`

Observed broad checks:

- `chimera-cli`: 400 tests passed.
- `chimera-mesh`: 294 unit tests passed plus 1 doc/integration-style test passed.
- `tests_nodes_inventory`: 25 tests passed.
- `tests_nodes_runtime_state`: 33 tests passed.
- `tests_discovery_merge`: 19 tests passed.

## Not Claimed

- Runtime bind/rebind/reconnect PASS is not claimed.
- SSH stand evidence is not claimed.
- Real-World PASS is not claimed.
- One-command install/update PASS is not claimed.
- Broad datapath performance PASS is not claimed.
