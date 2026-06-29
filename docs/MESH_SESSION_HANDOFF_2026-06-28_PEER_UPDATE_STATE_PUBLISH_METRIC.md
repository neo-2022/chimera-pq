# CHIMERA Mesh Session Handoff: Peer Update State Publish Metric

## Saved At

- Timestamp: 2026-06-28

## Active Objective

- Keep speeding and hardening metadata/control paths that help nodes find peers,
  choose paths, rebuild lane/binding/route state, publish state, and avoid
  wasted CPU/RAM.
- Keep sealed transit payload opaque and untouched.

## What Was Done

- Added a narrow `chimera-bootstrap` library entrypoint so the bootstrap binary
  and lab metrics can share peer-update logic without spawning the binary.
- Extracted in-memory peer-update state publish decision logic into
  `peer_update/serve_state_publish.rs`.
- Kept the file writer in `serve_state.rs` as the only filesystem layer for
  private permissions, temp-file write, `sync_all`, and rename.
- Added `chimera-lab metadata-perf-smoke` metrics for:
  - `peer_update_state_publish_noop_ops_per_sec`;
  - `peer_update_state_publish_noop_p95_ns`;
  - `peer_update_state_publish_changed_generation_ops_per_sec`;
  - `peer_update_state_publish_changed_generation_p95_ns`.
- Updated `metadata-perf-smoke-selfcheck` so the new JSON fields are mandatory.
- Updated `docs/PERFORMANCE.md`.
- Added
  `docs/WORKFLOW_ATTESTATION_METADATA_PERF_PEER_UPDATE_STATE_PUBLISH_METRIC_2026-06-28.md`.

## Validation

PASS:

- `cargo fmt --all`
- `cargo check -q -p chimera-bootstrap`
- `cargo check -q -p chimera-lab`
- `cargo test -q -p chimera-bootstrap peer_update -- --nocapture`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only -- --nocapture`
- `cargo clippy -q -p chimera-bootstrap --all-targets -- -D warnings`
- `cargo clippy -q -p chimera-lab --all-targets -- -D warnings`
- `just metadata-perf-smoke-selfcheck`
- `just metadata-perf-smoke`
- `just rust-no-hardcode-guard-selfcheck`
- `just release-pack-schema-guard-selfcheck`
- `just ship-structure-guard-selfcheck`
- `git diff --check` for touched code/docs files.

Current metadata snapshot:

- `peer_update_state_publish_noop_ops_per_sec=13038745`
- `peer_update_state_publish_noop_p95_ns=76`
- `peer_update_state_publish_changed_generation_ops_per_sec=117124`
- `peer_update_state_publish_changed_generation_p95_ns=8671`
- `network_state=not_modified`
- `transit_payload_policy=opaque_sealed_payload_untouched`

## Not Closed

- Planner/control-plane consumption of `endpoint_generation` is not wired in
  this slice.
- Real-world runtime bind/rebind/reconnect was not checked in this slice.
- Broad WEAVE datapath performance is not closed.
- SSH stand evidence remains required before any Real-World PASS claim.

## Next Step

- Good next slice: wire `endpoint_generation` into planner/control-plane reload
  or dirty-set invalidation so same-generation/no-op state does not trigger
  rebuild and changed generation invalidates only the affected peer/update
  metadata path.
