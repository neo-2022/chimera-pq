# Workflow Attestation: Metadata Perf Affected Peer Dirty Invalidation

## Status

Status: partial, lab/control-plane PASS only.

This slice proves that pending multipath rebuild signals can carry redacted
aggregate dirty metadata for discovery, health, and performance changes. It
does not claim selective planner rebuild, runtime, datapath, SSH-stand, or
Real-World PASS.

## Scope

- Hot path: `affected_peer_dirty_invalidation`.
- Runtime layer touched: `chimera-mesh` multipath rebuild signal metadata.
- Explain layer touched: in-memory multipath rebuild decision explain lines.
- Transit payload rule: opaque sealed payload untouched.
- Network state: not modified.

## Behavior Proven

- Legacy rebuild signals default to `dirty_scope=unknown` and
  `affected_peer_count=0`.
- Peer-set rebuild signals explain only aggregate dirty metadata:
  `multipath_rebuild_dirty_scope=peer_set` and
  `multipath_rebuild_affected_peer_count=N`.
- `peer_set(0)` is rejected, so a narrow peer-set signal cannot claim an empty
  changed set.
- Discovery update dirty count is exact for the changed input records:
  - one changed discovery record gives `affected_peer_count=1`;
  - two changed discovery records give `affected_peer_count=2`;
  - mixed batches count only records that changed rebuild-relevant metadata;
  - repeated identical discovery records keep pending rebuild clear.
- Table enforcement and stale cleanup fall back to `dirty_scope=unknown` and
  `affected_peer_count=0`, because the narrow input batch is no longer the full
  cause of the rebuild.
- Health and performance mixed batches count only peers whose metadata value
  changed; unchanged repeated records do not inflate `affected_peer_count`.
- Explain/debug coverage checks that raw peer ids and endpoints are not exposed
  by the dirty metadata path.

## Role Council

- Architect: approved only with exact changed-peer count; no-op records must not
  inflate dirty metadata.
- Senior developer: required the diff logic to stay near discovery/health/
  performance update paths and not be recalculated independently in diagnostics.
- Tester: required mixed-batch tests for unchanged plus changed records and
  negative-path fallback to `unknown/0`.
- Security: allowed aggregate-only metadata; no peer id, endpoint, route key,
  payload, secret, or stand detail may be emitted.
- DevOps/release: required release/ship guards and no runtime/SSH PASS claim.
- Critic: treated batch-size counting as a blocker; accepted only after exact
  changed-count tests were added.

## Evidence

Commands passed:

```text
cargo fmt --all -- --check
cargo check -q -p chimera-mesh
cargo check -q -p chimera-lab
cargo test -q -p chimera-mesh tests_multipath_schedule::rebuild_control -- --nocapture
cargo test -q -p chimera-mesh tests_multipath_schedule::rebuild_trigger -- --nocapture
cargo test -q -p chimera-mesh tests_discovery_merge -- --nocapture
cargo test -q -p chimera-mesh tests_multipath_schedule -- --nocapture
cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only -- --nocapture
cargo clippy -q -p chimera-mesh --all-targets -- -D warnings
cargo clippy -q -p chimera-lab --all-targets -- -D warnings
just metadata-perf-smoke
just metadata-perf-smoke-selfcheck
just rust-no-hardcode-guard-selfcheck
just release-pack-schema-guard-selfcheck
just ship-structure-guard-selfcheck
./scripts/release_pack_schema_guard.sh
./scripts/ship_structure_guard.sh
```

Latest `just metadata-perf-smoke` excerpt:

```text
status=ok
kind=metadata_perf_smoke
scope=hot_metadata_only
transit_payload_policy=opaque_sealed_payload_untouched
discovery_update_noop_ops_per_sec=130392
discovery_update_noop_p95_ns=8529
live_binding_reload_index_ops_per_sec=170517
network_state=not_modified
```

## Changed Files

- `crates/chimera-mesh/src/runtime/multipath_rebuild_model.rs`
- `crates/chimera-mesh/src/runtime/multipath_rebuild_trigger.rs`
- `crates/chimera-mesh/src/runtime/peer_discovery.rs`
- `crates/chimera-mesh/src/runtime/peer_discovery_update.rs`
- `crates/chimera-mesh/src/runtime/peer_health_lifecycle.rs`
- `crates/chimera-mesh/src/runtime/peer_performance.rs`
- `crates/chimera-mesh/src/runtime.rs`
- `crates/chimera-mesh/src/lib.rs`
- `crates/chimera-mesh/src/tests_multipath_schedule/rebuild_control.rs`
- `crates/chimera-mesh/src/tests_multipath_schedule/rebuild_trigger.rs`
- `docs/PERFORMANCE.md`

## Not Closed

- Selective/partial planner rebuild is not implemented by this slice.
- Runtime bind/rebind/reconnect was not checked.
- SSH-stand evidence was not collected.
- Real-World PASS is not claimed.
- Broad WEAVE datapath performance is not closed.
- Full endpoint-generation planner consumption remains separate work.

## Risks

- `affected_peer_count` is topology metadata. It remains aggregate-only in this
  slice and must not be expanded into peer identities in logs or exports.
- Perf numbers are smoke evidence, not a stable benchmark promise.
- Ambiguous causes must continue falling back to `unknown/0`.
