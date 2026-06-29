# CHIMERA Metadata Performance Attestation: Path Planner Selection Finalize Capacity

## Scope

- Date: 2026-06-27
- Hot path: `path_planner_selection_finalize_capacity`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice removes a stale under-reserve in `finalize_selection`.
- It also materializes selected peers with exact capacity instead of a default
  collect path.
- Output ordering, keys, and redaction remain unchanged.

## Change

- `crates/chimera-mesh/src/runtime/path_planner_finalize.rs`
  - increases `explain.reserve(...)` to cover the current explain tail;
  - builds `selected_peers` with exact capacity before materialization.
- `docs/PERFORMANCE.md`
  - records the new slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh tests_selection_behavior -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output after this slice:

```json
{"status":"ok","kind":"metadata_perf_smoke","hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint","lane_document_plan_snapshot_access","live_binding_reload_index","live_dps_plan_path_from_payload","status_explain"],"scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","iterations":100000,"active_bindings":16,"fast_sorted_ops_per_sec":1185368,"slow_sorted_fallback_ops_per_sec":156749,"fast_p95_ns":1009,"slow_sorted_fallback_p95_ns":6974,"fast_vs_fallback_speedup_pct":656.22,"path_planner_iterations":10000,"path_planner_peer_count":64,"path_planner_candidate_snapshot_ops_per_sec":7039,"path_planner_candidate_snapshot_p95_ns":156724,"discovery_rebuild_iterations":10000,"discovery_rebuild_peer_count":64,"discovery_rebuild_fingerprint_ops_per_sec":3762,"discovery_rebuild_fingerprint_p95_ns":271649,"lane_document_plan_snapshot_iterations":10000,"lane_document_plan_snapshot_borrowed_ops_per_sec":66529173,"lane_document_plan_snapshot_borrowed_p95_ns":14,"lane_document_plan_snapshot_owned_ops_per_sec":130712,"lane_document_plan_snapshot_owned_p95_ns":7751,"live_dps_plan_path_from_payload_iterations":10000,"live_dps_plan_path_from_payload_peer_count":64,"live_dps_plan_path_from_payload_ops_per_sec":3612,"live_dps_plan_path_from_payload_p95_ns":280475,"status_explain_iterations":10000,"status_explain_peer_count":64,"status_explain_ops_per_sec":17894,"status_explain_p95_ns":56193,"live_binding_reload_index_iterations":100000,"live_binding_reload_index_spawn_count":799200,"live_binding_reload_index_ops_per_sec":150208,"live_binding_reload_index_p95_ns":6700,"network_state":"not_modified"}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.

## Rollback

- Restore the old `explain.reserve(47)` value in
  `path_planner_finalize.rs`.
- Restore the `collect()` path for `selected_peers`.
- Remove the `path_planner_selection_finalize_capacity` bullet from
  `docs/PERFORMANCE.md`.
