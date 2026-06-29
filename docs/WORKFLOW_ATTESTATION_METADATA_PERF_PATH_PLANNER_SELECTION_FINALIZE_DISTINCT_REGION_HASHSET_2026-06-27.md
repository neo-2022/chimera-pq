# CHIMERA Metadata Performance Attestation: Path Planner Selection Finalize Distinct Region HashSet

## Scope

- Date: 2026-06-27
- Hot path: `path_planner_selection_finalize_distinct_region_hashset`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice removes tree-based distinct-region counting from selection
  finalization.
- It uses borrowed `HashSet`s for candidate and selected distinct-region
  counts, which matches the hot path's read-only usage.
- Output keys, ordering, and redaction remain unchanged.

## Change

- `crates/chimera-mesh/src/runtime/path_planner_finalize.rs`
  - counts distinct candidate and selected regions with borrowed `HashSet`s
    instead of `BTreeSet`;
  - keeps the explain contract unchanged.
- `docs/PERFORMANCE.md`
  - records the new slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh tests_selection_behavior -- --nocapture`
- `cargo test -q -p chimera-mesh runtime_planning -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output after this slice:

```json
{"status":"ok","kind":"metadata_perf_smoke","hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint","lane_document_plan_snapshot_access","live_binding_reload_index","live_dps_plan_path_from_payload","status_explain"],"scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","iterations":100000,"active_bindings":16,"fast_sorted_ops_per_sec":1163292,"slow_sorted_fallback_ops_per_sec":159621,"fast_p95_ns":990,"slow_sorted_fallback_p95_ns":6512,"fast_vs_fallback_speedup_pct":628.78,"path_planner_iterations":10000,"path_planner_peer_count":64,"path_planner_candidate_snapshot_ops_per_sec":7422,"path_planner_candidate_snapshot_p95_ns":142099,"discovery_rebuild_iterations":10000,"discovery_rebuild_peer_count":64,"discovery_rebuild_fingerprint_ops_per_sec":3784,"discovery_rebuild_fingerprint_p95_ns":270671,"lane_document_plan_snapshot_iterations":10000,"lane_document_plan_snapshot_borrowed_ops_per_sec":67509637,"lane_document_plan_snapshot_borrowed_p95_ns":14,"lane_document_plan_snapshot_owned_ops_per_sec":132969,"lane_document_plan_snapshot_owned_p95_ns":7765,"live_dps_plan_path_from_payload_iterations":10000,"live_dps_plan_path_from_payload_peer_count":64,"live_dps_plan_path_from_payload_ops_per_sec":3594,"live_dps_plan_path_from_payload_p95_ns":288796,"status_explain_iterations":10000,"status_explain_peer_count":64,"status_explain_ops_per_sec":17710,"status_explain_p95_ns":59500,"live_binding_reload_index_iterations":100000,"live_binding_reload_index_spawn_count":799200,"live_binding_reload_index_ops_per_sec":151321,"live_binding_reload_index_p95_ns":6683,"network_state":"not_modified"}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.

## Rollback

- Restore `BTreeSet` distinct-region counting in `path_planner_finalize.rs`.
- Remove the `path_planner_selection_finalize_distinct_region_hashset` bullet
  from `docs/PERFORMANCE.md`.
