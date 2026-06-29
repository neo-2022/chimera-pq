# Workflow Attestation: Metadata Perf Discovery Update No-op Dirty Set

## Status

Status: partial, lab/control-plane PASS only.

This slice proves that repeated identical discovery records do not create
unnecessary multipath rebuild churn in the in-memory mesh control path. It does
not claim runtime, datapath, SSH-stand, or Real-World PASS.

## Scope

- Hot path: `discovery_update_noop_dirty_set`.
- Runtime layer touched: `chimera-mesh` discovery merge metadata handling.
- Lab metric touched: `chimera-lab metadata-perf-smoke`.
- Transit payload rule: opaque sealed payload untouched.
- Network state: not modified.

## Behavior Proven

- Existing peer update with identical usable metadata:
  - refreshes peer liveness;
  - keeps peer snapshot unchanged;
  - keeps selected peer stability counters at `u1:r0:h0:d0`;
  - does not set pending multipath rebuild.
- Existing peer update with changed endpoint marks pending rebuild with
  `peer_table_changed`.
- Existing peer update with changed region marks pending rebuild with
  `peer_table_changed`.
- Existing peer update with changed load score marks pending rebuild with
  `peer_table_changed`.
- Existing peer update with changed reliability score marks pending rebuild with
  `peer_table_changed`.
- The metadata perf smoke fixture first confirms that initial discovery creates
  a real pending rebuild, clears it through the planner path, and only then
  measures repeated identical discovery no-op merges.

## Evidence

Commands passed:

```text
cargo fmt --all -- --check
cargo check -q -p chimera-mesh
cargo check -q -p chimera-lab
cargo test -q -p chimera-mesh tests_multipath_schedule::rebuild_trigger -- --nocapture
cargo test -q -p chimera-mesh tests_discovery_merge -- --nocapture
cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only -- --nocapture
cargo clippy -q -p chimera-mesh --all-targets -- -D warnings
cargo clippy -q -p chimera-lab --all-targets -- -D warnings
just metadata-perf-smoke
just metadata-perf-smoke-selfcheck
just rust-no-hardcode-guard-selfcheck
just release-pack-schema-guard-selfcheck
just ship-structure-guard-selfcheck
git diff --check -- crates/chimera-mesh/src/runtime/peer_discovery_update.rs crates/chimera-mesh/src/tests_multipath_schedule/rebuild_trigger.rs crates/chimera-lab/src/metadata_perf.rs justfile docs/PERFORMANCE.md
```

Latest `just metadata-perf-smoke` excerpt:

```text
discovery_update_noop_iterations=10000
discovery_update_noop_ops_per_sec=130240
discovery_update_noop_p95_ns=8609
network_state=not_modified
transit_payload_policy=opaque_sealed_payload_untouched
```

## Changed Files

- `crates/chimera-mesh/src/runtime/peer_discovery_update.rs`
- `crates/chimera-mesh/src/tests_multipath_schedule/rebuild_trigger.rs`
- `crates/chimera-lab/src/metadata_perf.rs`
- `justfile`
- `docs/PERFORMANCE.md`

## Not Closed

- Runtime bind/rebind/reconnect was not checked.
- SSH stand evidence was not collected.
- Real-World PASS is not claimed.
- Broad WEAVE datapath performance is not closed.
- Full endpoint-generation planner consumption remains separate work.

## Risks

- This is in-memory lab/control-plane evidence only.
- Perf numbers are smoke evidence, not a stable benchmark claim.
- Repeated changed metadata can still trigger legitimate rebuild work; DoS
  control for noisy peers remains a broader control-plane hardening task.
