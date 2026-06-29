# CHIMERA Metadata Performance Attestation: Path Planner Selection Metrics Strings

## Scope

- Date: 2026-06-26
- Hot path: `path_planner_selection_metrics_strings`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Council Result

- Architect: keep this as a local explain-string optimization inside the
  existing `plan_path` flow; do not split metadata into a new persistent model.
- Senior Rust: replace `Vec` collect/join patterns with direct string builders
  for selected peer and stability summaries.
- Tester: require selection-behavior tests and the main metadata smoke to stay
  redaction-safe.
- Security: accept only if the output stays redacted and no payload bytes are
  inspected.
- DevOps: treat this as an extension of the existing `path_planner_candidate_snapshot`
  measurement, not a new transport or runtime mode.
- Critic: reject any change that alters selected peer order, explain keys, or
  the fail-closed behavior of path planning.

## Change

- `crates/chimera-mesh/src/runtime/path_planner_selection_metrics_format.rs`
  - adds a tiny direct string-builder helper for comma-separated selected-peer
    values.
- `crates/chimera-mesh/src/runtime/path_planner_selection_metrics_peer.rs`
  - builds selected peer ids, regions, endpoints, scores, and region counts
    directly into `String` values instead of collecting intermediate `Vec`s;
  - preserves ordering and redacted labels.
- `crates/chimera-mesh/src/runtime/path_planner_selection_metrics_stability.rs`
  - builds selected stability and replacement summary strings directly instead
    of collecting intermediate `Vec`s;
  - preserves ordering and the existing explain keys.
- `docs/PERFORMANCE.md`
  - records the new `path_planner_selection_metrics_strings` sub-slice.

## Evidence

Commands passed locally without changing network state:

- `cargo test -q -p chimera-mesh tests_selection_behavior`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo test -q -p chimera-lab metadata_perf`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output:

```json
{"status":"ok","kind":"metadata_perf_smoke","hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint","lane_document_plan_snapshot_access","live_binding_reload_index"],"scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","iterations":100000,"active_bindings":16,"fast_sorted_ops_per_sec":1099861,"slow_sorted_fallback_ops_per_sec":157394,"fast_p95_ns":1064,"slow_sorted_fallback_p95_ns":6979,"fast_vs_fallback_speedup_pct":598.79,"path_planner_iterations":10000,"path_planner_peer_count":64,"path_planner_candidate_snapshot_ops_per_sec":6999,"path_planner_candidate_snapshot_p95_ns":154442,"discovery_rebuild_iterations":10000,"discovery_rebuild_peer_count":64,"discovery_rebuild_fingerprint_ops_per_sec":3769,"discovery_rebuild_fingerprint_p95_ns":270533,"lane_document_plan_snapshot_iterations":10000,"lane_document_plan_snapshot_borrowed_ops_per_sec":66210696,"lane_document_plan_snapshot_borrowed_p95_ns":14,"lane_document_plan_snapshot_owned_ops_per_sec":134109,"lane_document_plan_snapshot_owned_p95_ns":7620,"live_binding_reload_index_iterations":100000,"live_binding_reload_index_spawn_count":799200,"live_binding_reload_index_ops_per_sec":152263,"live_binding_reload_index_p95_ns":6587,"network_state":"not_modified"}
```

## What Is Not Closed

- This is not broad WEAVE datapath performance closure.
- This is not Real-World PASS.
- SSH runtime/datapath/load checks on the external stand remain separate.

## Rollback

- Restore `Vec` collection and `join()` in the selection peer/stability
  builders.
- Remove the `path_planner_selection_metrics_strings` bullet from
  `docs/PERFORMANCE.md`.
