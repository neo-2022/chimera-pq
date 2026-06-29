# CHIMERA Metadata Performance Attestation: Discovery/Rebuild Fingerprint

## Scope

- Date: 2026-06-26
- Hot path: `discovery_rebuild_fingerprint`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Council Result

- Architectural choice: reuse one private normalized region distribution
  snapshot inside the rebuild trigger path; do not add a persistent cache.
- Tester: require no-op discovery coverage and repeated same-value performance
  coverage so rebuild suppression stays honest.
- Security / critic: reject narrower fingerprints, order-dependent snapshots,
  or any change that leaks raw peer fields or payload state into debug output.

## Change

- `crates/chimera-mesh/src/runtime.rs`
  - adds private `region_distribution_counts()` helper;
  - `region_distribution()` now delegates to that helper.
- `crates/chimera-mesh/src/runtime/multipath_rebuild_trigger.rs`
  - `rebuild_trigger_fingerprint()` now reuses one region distribution snapshot
    instead of calling `region_distribution()` twice.
- `crates/chimera-mesh/src/runtime/peer_performance.rs`
  - adds a regression test for repeated same-value performance updates.
- `crates/chimera-mesh/src/tests_multipath_schedule/rebuild_trigger.rs`
  - adds a regression test for empty discovery batches staying no-op on stable
    state.
- `crates/chimera-lab/src/metadata_perf.rs`
  - adds a discovery/rebuild fingerprint benchmark fixture;
  - exposes discovery rebuild metrics in CLI output and JSON;
  - keeps discovery/rebuild smoke redacted and metadata-only.
- `justfile`
  - extends `metadata-perf-smoke-selfcheck` to require the new discovery rebuild
    metric field.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh tests_multipath_schedule`
- `cargo test -q -p chimera-mesh peer_performance`
- `cargo test -q -p chimera-lab metadata_perf`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo clippy -q -p chimera-lab --all-targets -- -D warnings`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output:

```json
{"status":"ok","kind":"metadata_perf_smoke","hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint"],"scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","iterations":100000,"active_bindings":16,"fast_sorted_ops_per_sec":1181462,"slow_sorted_fallback_ops_per_sec":163998,"fast_p95_ns":1216,"slow_sorted_fallback_p95_ns":6241,"fast_vs_fallback_speedup_pct":620.41,"path_planner_iterations":10000,"path_planner_peer_count":64,"path_planner_candidate_snapshot_ops_per_sec":7107,"path_planner_candidate_snapshot_p95_ns":147493,"discovery_rebuild_iterations":10000,"discovery_rebuild_peer_count":64,"discovery_rebuild_fingerprint_ops_per_sec":3796,"discovery_rebuild_fingerprint_p95_ns":265460,"network_state":"not_modified"}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Live binding reload/index optimization remains open.

## Rollback

- Remove `region_distribution_counts()` and restore the previous double lookup.
- Remove the discovery rebuild benchmark fields and smoke checks.
- Remove the no-op discovery and repeated same-value performance regressions.
