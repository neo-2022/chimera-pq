# CHIMERA Mesh Session Handoff: CLI Endpoint Generation Consumption

## Saved At

- Timestamp: 2026-06-29T09:18:48Z

## Active Objective

- Keep speeding and hardening metadata/control paths that help nodes find peers,
  choose paths, rebuild lane/binding/route state, publish state, and avoid
  wasted CPU/RAM.
- Keep sealed transit payload opaque and untouched.

## What Was Done

- Finished the CLI/inventory/update-state bridge into
  `MeshRuntime::merge_published_endpoint_updates(...)`.
- Added validated construction of `MeshPublishedEndpointUpdate` from inventory
  nodes with optional `endpoint_generation`.
- `mesh nodes probe --all` now applies published endpoint updates after normal
  discovery merge and before connect probe planning.
- Preserved legacy bootstrap and static inventory compatibility by keeping
  `endpoint_generation` optional and absent by default.
- Added config parser tests for positive, zero and invalid endpoint generation.
- Added signed discovery tests that preserve positive endpoint generation and
  reject zero generation.
- Strengthened advertise-state coverage so signed snapshots include
  `endpoint_generation` when private update state provides it.
- Added adapter proof that a node with newer endpoint generation updates the
  in-memory runtime peer endpoint and is visible to planner selection.

## Validation

PASS:

- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `cargo check -q -p chimera-cli`
- `cargo test -q -p chimera-cli`
- `cargo test -q -p chimera-mesh`
- `cargo test -q -p chimera-cli tests_nodes_inventory -- --nocapture`
- `cargo test -q -p chimera-cli tests_nodes_runtime_state -- --nocapture`
- `cargo test -q -p chimera-mesh tests_discovery_merge -- --nocapture`
- `cargo clippy -q -p chimera-cli --all-targets -- -D warnings`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `just rust-no-hardcode-guard-selfcheck`
- `just release-pack-schema-guard-selfcheck`
- `just ship-structure-guard-selfcheck`
- `./scripts/release_pack_schema_guard.sh`
- `./scripts/ship_structure_guard.sh`
- `git diff --check -- <touched files>`

## Role Council

- Architect: approved the control-plane bridge while keeping
  `MeshDiscoveryRecord` unchanged.
- Senior developer: required a small adapter in CLI/inventory and no
  bootstrap/diagnostics monolith.
- Tester: required legacy compatibility, positive and negative generation
  parsing, discovery preservation, advertise output and runtime consumption
  proof.
- Security: required strict generation validation, update URL validation,
  stale rollback protection through runtime merge and redacted diagnostics.
- DevOps/release: required lab-only status and guards against stand hardcode and
  release schema drift.
- Critic: required planner visibility, not just field propagation.

## Not Closed

- Runtime bind/rebind/reconnect was not checked.
- SSH stand evidence was not collected.
- Real-World PASS is not claimed.
- One-command install/update was not checked in this slice.
- Broad WEAVE datapath performance is not closed.
- Selective/partial planner rebuild is not implemented.

## Next Step

- Continue toward runtime automatic bind/rebind/reconnect: prove that an
  OS-selected listener endpoint is written to private runtime state, advertised
  with a monotonic `endpoint_generation`, consumed by peers through the
  published endpoint update path, and causes reconnect/retry without manual port
  selection. This next step needs a separate proof bundle and, for Real-World
  PASS, SSH stand evidence.
