# CHIMERA Mesh Session Handoff: Published Endpoint Consumption

## Saved At

- Timestamp: 2026-06-29T08:36:22Z

## Active Objective

- Keep speeding and hardening metadata/control paths that help nodes find peers,
  choose paths, rebuild lane/binding/route state, publish state, and avoid
  wasted CPU/RAM.
- Keep sealed transit payload opaque and untouched.

## What Was Done

- Added `MeshPublishedEndpointUpdate` as an internal control-plane update type
  for published endpoint state.
- Added `MeshRuntime::merge_published_endpoint_updates(...)`.
- Stored the last accepted `endpoint_generation` and optional private
  `update_bootstrap_url` in peer metadata.
- Added redacted `Debug` for published endpoint updates and peer metadata.
- Made newer endpoint generation update existing peer endpoint state and mark
  `dirty_scope=peer_set` with exact `affected_peer_count`.
- Made same-generation/same-state updates no-op.
- Made stale/lower generation deterministic no-op without rollback.
- Made same-generation/different-state, zero generation, invalid endpoint and
  duplicate batch cases fail before mutation.
- Prevented normal discovery records without generation from downgrading an
  endpoint accepted through published endpoint state.
- Included endpoint and endpoint-generation-sensitive metadata in rebuild
  fingerprints so applied endpoint updates can drive planner invalidation.
- Added tests for no-op, newer, stale, zero, invalid, conflict, mixed exact
  count, two-peer count, downgrade protection and redaction.

## Validation

PASS:

- `cargo fmt --all -- --check`
- `cargo check -q -p chimera-mesh`
- `cargo check -q -p chimera-lab`
- `cargo test -q -p chimera-mesh tests_discovery_merge -- --nocapture`
- `cargo test -q -p chimera-mesh tests_multipath_schedule::rebuild_trigger -- --nocapture`
- `cargo test -q -p chimera-mesh tests_multipath_schedule::rebuild_control -- --nocapture`
- `cargo test -q -p chimera-mesh`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo clippy -q -p chimera-lab --all-targets -- -D warnings`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`
- `just rust-no-hardcode-guard-selfcheck`
- `just release-pack-schema-guard-selfcheck`
- `just ship-structure-guard-selfcheck`
- `./scripts/release_pack_schema_guard.sh`
- `./scripts/ship_structure_guard.sh`
- `git diff --check -- <touched files>`

Current metadata snapshot:

- `discovery_rebuild_fingerprint_ops_per_sec=3168`
- `discovery_rebuild_fingerprint_p95_ns=331524`
- `discovery_update_noop_ops_per_sec=120184`
- `discovery_update_noop_p95_ns=8535`
- `peer_update_state_publish_noop_ops_per_sec=13001889`
- `peer_update_state_publish_noop_p95_ns=116`
- `peer_update_state_publish_changed_generation_ops_per_sec=115116`
- `peer_update_state_publish_changed_generation_p95_ns=8998`
- `network_state=not_modified`
- `transit_payload_policy=opaque_sealed_payload_untouched`

## Role Council

- Architect: required no public discovery-contract expansion and explicit
  discovery downgrade protection.
- Senior developer: required a small internal adapter and no runtime/network
  scope creep.
- Tester: required no-op, newer, stale, zero, invalid, mixed exact-count and
  redaction tests.
- Security: required monotonic generation handling, URL/endpoint validation,
  fail-closed conflict behavior and no payload involvement.
- DevOps/release: required lab-only status and no SSH/Real-World PASS claim.
- Critic: required causal proof that applied endpoint state drives pending
  rebuild, not leftover signals or diagnostics-only output.

## Not Closed

- Runtime bind/rebind/reconnect was not checked.
- SSH stand evidence was not collected.
- Real-World PASS is not claimed.
- One-command install/update was not checked in this slice.
- Broad WEAVE datapath performance is not closed.
- Selective/partial planner rebuild is not implemented.

## Next Step

- Continue with endpoint publish path integration from CLI/inventory/update
  state into `merge_published_endpoint_updates(...)`, still lab-first:
  parse/construct published endpoint updates from existing advertised runtime
  state without adding stand-specific defaults or changing public discovery
  records.
