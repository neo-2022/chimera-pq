# CHIMERA Mesh Session Handoff: Discovery Update No-op Dirty Set

## Saved At

- Timestamp: 2026-06-28

## Active Objective

- Keep speeding and hardening metadata/control paths that help nodes find peers,
  choose paths, rebuild lane/binding/route state, publish state, and avoid
  wasted CPU/RAM.
- Keep sealed transit payload opaque and untouched.

## What Was Done

- Added an identical-record fast path for existing discovery peer updates:
  - only active when previous metadata exists;
  - only within the stability window;
  - only when endpoint, region, reliability score, and load score match;
  - refreshes `last_seen_tick`;
  - returns before changing counters folded into the rebuild fingerprint.
- Added rebuild-trigger tests proving:
  - empty discovery batch keeps pending rebuild clear;
  - identical existing discovery keeps pending rebuild clear;
  - identical existing discovery keeps peer snapshot unchanged;
  - identical existing discovery keeps selected stability at `u1:r0:h0:d0`;
  - identical existing discovery refreshes liveness and avoids stale eviction;
  - changed endpoint, region, load score, and reliability score each raise
    `peer_table_changed`.
- Added `chimera-lab metadata-perf-smoke` metrics for:
  - `discovery_update_noop_ops_per_sec`;
  - `discovery_update_noop_p95_ns`.
- Updated `metadata-perf-smoke-selfcheck`.
- Updated `docs/PERFORMANCE.md`.
- Added
  `docs/WORKFLOW_ATTESTATION_METADATA_PERF_DISCOVERY_UPDATE_NOOP_DIRTY_SET_2026-06-28.md`.

## Validation

PASS:

- `cargo fmt --all -- --check`
- `cargo check -q -p chimera-mesh`
- `cargo check -q -p chimera-lab`
- `cargo test -q -p chimera-mesh tests_multipath_schedule::rebuild_trigger -- --nocapture`
- `cargo test -q -p chimera-mesh tests_discovery_merge -- --nocapture`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo clippy -q -p chimera-lab --all-targets -- -D warnings`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`
- `just rust-no-hardcode-guard-selfcheck`
- `just release-pack-schema-guard-selfcheck`
- `just ship-structure-guard-selfcheck`
- `git diff --check` for touched files.

Current metadata snapshot:

- `discovery_update_noop_ops_per_sec=130240`
- `discovery_update_noop_p95_ns=8609`
- `network_state=not_modified`
- `transit_payload_policy=opaque_sealed_payload_untouched`

## Role Council

- Architect: agreed; required liveness proof for `last_seen_tick`.
- Senior developer: agreed conditionally; warned not to claim full control-plane
  completion without broader dirty-set proof.
- Tester: agreed conditionally; requested fixture guard proving initial pending
  rebuild existed before no-op measurement.
- Security: agreed conditionally; required aggregate-only JSON and no runtime
  claim.
- DevOps/release: agreed conditionally; lab/control-path evidence only.
- Critic: warned against claiming runtime/pass and requested liveness/fingerprint
  evidence. Liveness was added through stale-eviction test; fingerprint evidence
  is indirect through no pending rebuild plus unchanged stability counters.

## Not Closed

- Runtime bind/rebind/reconnect was not checked in this slice.
- SSH stand evidence was not collected.
- Real-World PASS is not claimed.
- Broad WEAVE datapath performance is not closed.
- Full endpoint-generation planner consumption is still separate work.

## Next Step

- Continue with planner/control-plane consumption of endpoint-generation or
  affected-peer dirty invalidation, without changing the public discovery
  contract unless a separate council approves it.
