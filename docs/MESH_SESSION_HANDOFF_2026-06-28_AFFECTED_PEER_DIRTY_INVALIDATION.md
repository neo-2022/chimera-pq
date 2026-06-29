# CHIMERA Mesh Session Handoff: Affected Peer Dirty Invalidation

## Saved At

- Timestamp: 2026-06-28

## Active Objective

- Keep speeding and hardening metadata/control paths that help nodes find peers,
  choose paths, rebuild lane/binding/route state, publish state, and avoid
  wasted CPU/RAM.
- Keep sealed transit payload opaque and untouched.

## What Was Done

- Added redacted dirty metadata to multipath rebuild signals:
  - `dirty_scope=unknown|peer_set`;
  - `affected_peer_count=N`;
  - legacy signals default to `unknown/0`.
- Added rebuild decision explain lines:
  - `multipath_rebuild_dirty_scope=...`;
  - `multipath_rebuild_affected_peer_count=...`.
- Discovery, health, and performance updates now count only peers whose
  rebuild-relevant metadata actually changed.
- Mixed discovery batches now count only changed records; identical records
  refresh liveness but do not inflate dirty metadata.
- Stale peer/health cleanup and table-enforcement drops fall back to
  `unknown/0`.
- Added tests for peer-set redaction, zero-count rejection, mixed discovery,
  mixed health/performance, enforcement fallback, and stale-eviction fallback.
- Updated `docs/PERFORMANCE.md`.
- Added
  `docs/WORKFLOW_ATTESTATION_METADATA_PERF_AFFECTED_PEER_DIRTY_INVALIDATION_2026-06-28.md`.

## Validation

PASS:

- `cargo fmt --all -- --check`
- `cargo check -q -p chimera-mesh`
- `cargo check -q -p chimera-lab`
- `cargo test -q -p chimera-mesh tests_multipath_schedule::rebuild_control -- --nocapture`
- `cargo test -q -p chimera-mesh tests_multipath_schedule::rebuild_trigger -- --nocapture`
- `cargo test -q -p chimera-mesh tests_discovery_merge -- --nocapture`
- `cargo test -q -p chimera-mesh tests_multipath_schedule -- --nocapture`
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

Current metadata snapshot:

- `discovery_update_noop_ops_per_sec=130392`
- `discovery_update_noop_p95_ns=8529`
- `live_binding_reload_index_ops_per_sec=170517`
- `network_state=not_modified`
- `transit_payload_policy=opaque_sealed_payload_untouched`

## Role Council

- Architect: agreed only after exact changed-peer count replaced batch-size
  counting.
- Senior developer: agreed; required diff logic near update paths and no
  diagnostics-side recalculation.
- Tester: required mixed-batch and ambiguous-fallback tests.
- Security: agreed only for aggregate-only scope/count with no ids, endpoints,
  route keys, payload, secrets, or stand details.
- DevOps/release: required guard bundle and no runtime/Real-World PASS claim.
- Critic: treated batch-size counting as a blocker; accepted the exact-count
  correction plus tests.

## Not Closed

- Selective/partial planner rebuild is not implemented.
- Runtime bind/rebind/reconnect was not checked in this slice.
- SSH stand evidence was not collected.
- Real-World PASS is not claimed.
- Broad WEAVE datapath performance is not closed.
- Full endpoint-generation planner consumption remains separate work.

## Next Step

- Continue with automatic endpoint publish consumption in the mesh control path:
  connect `endpoint_generation` / published endpoint state to discovery and
  planner invalidation so bind/rebind updates can be consumed without manual
  ports.
