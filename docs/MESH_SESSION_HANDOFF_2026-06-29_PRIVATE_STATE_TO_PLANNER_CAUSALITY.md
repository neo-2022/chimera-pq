# CHIMERA Mesh Session Handoff: Private State To Planner Causality

## Saved At

- Timestamp: 2026-06-29T09:57:14Z

## Active Objective

- Keep speeding and hardening metadata/control paths that help nodes find peers,
  choose paths, rebuild lane/binding/route state, publish state, and avoid
  wasted CPU/RAM.
- Keep sealed transit payload opaque and untouched.

## What Was Done

- Closed the lab/control-plane causality gap between private peer update state
  and runtime planner state.
- Added test coverage that starts with private update state, runs the real
  `mesh nodes advertise` command path, imports the signed discovery artifact
  through the discovery loader, builds published endpoint updates, merges them
  into an existing runtime peer, and verifies planner before/after endpoint
  selection.
- Verified the accepted update records `published_endpoint_changed` with
  `affected_peer_count=1`.
- Verified identical and stale updates do not roll planner selection back to the
  old endpoint.
- Kept the change lab/control-plane only: no local CHIMERA runtime start/stop,
  no routes/DNS/firewall/proxy changes and no SSH stand claim.

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

## Role Council

- Architect: required no public discovery-contract expansion and a full
  control-plane causality proof.
- Senior developer: required using boundary APIs and keeping helper logic out of
  product runtime modules.
- Tester: required private state, signed artifact, discovery loader, runtime
  merge, planner before/after and stale/no-op checks.
- Security: required stale rollback protection, redaction and no payload access.
- DevOps/release: required lab-only status and hardcode/schema/structure guards.
- Critic: required planner evidence, not only field propagation.

## Not Closed

- Runtime reconnect/retry after remote peer endpoint change is not checked.
- SSH stand evidence was not collected.
- Real-World PASS is not claimed.
- One-command install/update was not checked in this slice.
- Broad WEAVE datapath performance is not closed.
- Selective/partial planner rebuild is not implemented.

## Next Step

- Continue with runtime reconnect/retry behavior for an existing peer after a
  published endpoint update. The next proof must show the selected connection
  path uses the updated endpoint and does not require manual port selection. For
  Real-World PASS this must later be verified on the SSH stand.
