# CHIMERA Metadata Performance Attestation: Path Planner Selection Region Counts Small Vec

## Scope

- Date: 2026-06-28
- Hot path: `path_planner_candidate_snapshot`
- Secondary affected path: `live_dps_plan_path_from_payload`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- `selected_region_counts` is a short hot metadata summary in the path-planner
  peer selection loop.
- The previous implementation used tree bookkeeping while selected peer counts
  are small and bounded by policy.
- The output contract requires normalized-region lexicographic ordering, so the
  replacement keeps one final sort before formatting.

## Change

- `crates/chimera-mesh/src/runtime/path_planner_selection_metrics_peer.rs`
  - replaces per-peer `BTreeMap` updates with a small `Vec<(String, usize)>`;
  - keeps `normalize_region_key()` for explain output;
  - sorts once before formatting so `selected_region_counts` remains
    lexicographic by normalized region;
  - adds a unit test for reverse insertion order: `us,EU` still emits
    `eu:1,us:1`.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all`
- `cargo check -q -p chimera-mesh`
- `cargo test -q -p chimera-mesh build_peer_selection_summary_sorts_region_counts_by_normalized_region_not_insertion_order -- --nocapture`
- `cargo test -q -p chimera-mesh tests_selection_behavior -- --nocapture`
- `cargo test -q -p chimera-mesh runtime_planning -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output after this slice:

```json
{"status":"ok","kind":"metadata_perf_smoke","hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint","lane_document_plan_snapshot_access","live_binding_reload_index","live_dps_plan_path_from_payload","status_explain"],"scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","iterations":100000,"active_bindings":16,"fast_sorted_ops_per_sec":1200703,"slow_sorted_fallback_ops_per_sec":152532,"fast_p95_ns":947,"slow_sorted_fallback_p95_ns":7966,"fast_vs_fallback_speedup_pct":687.18,"path_planner_iterations":10000,"path_planner_peer_count":64,"path_planner_candidate_snapshot_ops_per_sec":7417,"path_planner_candidate_snapshot_p95_ns":142063,"discovery_rebuild_iterations":10000,"discovery_rebuild_peer_count":64,"discovery_rebuild_fingerprint_ops_per_sec":3734,"discovery_rebuild_fingerprint_p95_ns":277086,"lane_document_plan_snapshot_iterations":10000,"lane_document_plan_snapshot_borrowed_ops_per_sec":66118763,"lane_document_plan_snapshot_borrowed_p95_ns":14,"lane_document_plan_snapshot_owned_ops_per_sec":129238,"lane_document_plan_snapshot_owned_p95_ns":8185,"live_dps_plan_path_from_payload_iterations":10000,"live_dps_plan_path_from_payload_peer_count":64,"live_dps_plan_path_from_payload_ops_per_sec":4610,"live_dps_plan_path_from_payload_p95_ns":231307,"status_explain_iterations":10000,"status_explain_peer_count":64,"status_explain_ops_per_sec":16392,"status_explain_p95_ns":63423,"live_binding_reload_index_iterations":100000,"live_binding_reload_index_spawn_count":799200,"live_binding_reload_index_ops_per_sec":148677,"live_binding_reload_index_p95_ns":6843,"network_state":"not_modified"}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other path-planner metadata formatting hotspots may still remain.

## Rollback

- Restore `BTreeMap<String, usize>` region-count bookkeeping in
  `path_planner_selection_metrics_peer.rs`.
- Remove the reverse-order region-count unit test.
- Remove the `path_planner_selection_region_counts_small_vec` bullet from
  `docs/PERFORMANCE.md`.
