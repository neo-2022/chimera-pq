# Workflow Attestation: Private State To Planner Causality

## Scope

- Date: 2026-06-29
- Slice: private peer update state -> advertise -> signed discovery import ->
  published endpoint update -> runtime planner.
- Status: lab/control-plane evidence only.
- Network state: not modified.
- Transit payload policy: opaque/sealed payload untouched.

## Implemented

- Added a causality proof that starts from private peer update state carrying
  `endpoint`, `update_bootstrap_url` and `endpoint_generation`.
- The proof runs through the real `mesh nodes advertise` command path to write a
  signed discovery artifact.
- The signed discovery artifact is served through a one-shot lab HTTP fixture
  and loaded through the normal discovery inventory loader.
- The discovery inventory builds validated `MeshPublishedEndpointUpdate` records
  through the existing CLI/inventory adapter.
- An existing runtime peer starts with an old endpoint, consumes the published
  endpoint update, records `published_endpoint_changed`, and planner selection
  sees the new endpoint.
- The same test verifies that identical and stale updates do not roll planner
  state back to the old endpoint.

## Role Council Summary

- Architect: required end-to-end control-plane proof without changing
  `MeshDiscoveryRecord`.
- Senior developer: required using boundary APIs and avoiding new production
  dependencies between bootstrap and CLI.
- Tester: required private state, advertise artifact, signed discovery import,
  runtime merge, planner before/after and negative stale/no-op checks.
- Security: required strict validation, stale rollback prevention, redaction and
  no payload/datapath access.
- DevOps/release: required lab-only wording, no stand hardcode and guard checks.
- Critic: required proof that planner state changes, not only field propagation.

## Validation

PASS:

- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `cargo check -q -p chimera-bootstrap`
- `cargo check -q -p chimera-cli`
- `cargo check -q -p chimera-mesh`
- `cargo test -q -p chimera-cli nodes_private_state_advertise_discovery_update_reaches_runtime_planner -- --nocapture`
- `cargo test -q -p chimera-cli tests_nodes_runtime_state -- --nocapture`
- `cargo test -q -p chimera-cli`
- `cargo test -q -p chimera-bootstrap`
- `cargo test -q -p chimera-mesh tests_discovery_merge -- --nocapture`
- `cargo test -q -p chimera-mesh`
- `cargo clippy -q -p chimera-bootstrap --all-targets -- -D warnings`
- `cargo clippy -q -p chimera-cli --all-targets -- -D warnings`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `just rust-no-hardcode-guard-selfcheck`
- `just release-pack-schema-guard-selfcheck`
- `just ship-structure-guard-selfcheck`
- `./scripts/release_pack_schema_guard.sh`
- `./scripts/ship_structure_guard.sh`
- `git diff --check -- <touched files>`

Observed broad checks:

- `chimera-cli`: 401 tests passed.
- `chimera-bootstrap`: 34 tests passed.
- `chimera-mesh`: 294 unit tests passed plus 1 doc/integration-style test passed.
- `tests_nodes_runtime_state`: 34 tests passed.
- `tests_discovery_merge`: 19 tests passed.

## Not Claimed

- Runtime reconnect/retry PASS is not claimed.
- SSH stand evidence is not claimed.
- Real-World PASS is not claimed.
- One-command install/update PASS is not claimed.
- Broad datapath performance PASS is not claimed.
