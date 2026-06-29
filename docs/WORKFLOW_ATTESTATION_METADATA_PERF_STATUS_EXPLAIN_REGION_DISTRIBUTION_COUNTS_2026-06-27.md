# CHIMERA Metadata Performance Attestation: Status Explain Region Distribution Counts

## Scope

- Date: 2026-06-27
- Hot path: `status_explain_region_distribution_counts`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This micro-slice removes one extra collection step from the status explain
  region-distribution path.
- `status_base_explain.rs` now formats from the counts map directly instead of
  collecting an intermediate `Vec` first.
- Output keys, ordering, and redaction remain unchanged.

## Change

- `crates/chimera-mesh/src/runtime/status_base_explain.rs`
  - formats region distribution directly from the counts map;
  - keeps the existing explain contract unchanged.
- `docs/PERFORMANCE.md`
  - records the new slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh tests_preemptive_status -- --nocapture`
- `cargo test -q -p chimera-mesh tests_selection_behavior -- --nocapture`
- `cargo test -q -p chimera-mesh runtime_status_explain -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output after this slice:

```json
{"status":"ok","kind":"metadata_perf_smoke","hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint","lane_document_plan_snapshot_access","live_binding_reload_index","live_dps_plan_path_from_payload","status_explain"],"scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","iterations":100000,"active_bindings":16,"fast_sorted_ops_per_sec":1178847,"slow_sorted_fallback_ops_per_sec":154376,"fast_p95_ns":985,"slow_sorted_fallback_p95_ns":7268,"fast_vs_fallback_speedup_pct":663.62,"path_planner_iterations":10000,"path_planner_peer_count":64,"path_planner_candidate_snapshot_ops_per_sec":6440,"path_planner_candidate_snapshot_p95_ns":175727,"discovery_rebuild_iterations":10000,"discovery_rebuild_peer_count":64,"discovery_rebuild_fingerprint_ops_per_sec":3518,"discovery_rebuild_fingerprint_p95_ns":302052,"lane_document_plan_snapshot_iterations":10000,"lane_document_plan_snapshot_borrowed_ops_per_sec":65597429,"lane_document_plan_snapshot_borrowed_p95_ns":14,"lane_document_plan_snapshot_owned_ops_per_sec":130811,"lane_document_plan_snapshot_owned_p95_ns":8716,"live_dps_plan_path_from_payload_iterations":10000,"live_dps_plan_path_from_payload_peer_count":64,"live_dps_plan_path_from_payload_ops_per_sec":3403,"live_dps_plan_path_from_payload_p95_ns":309569,"status_explain_iterations":10000,"status_explain_peer_count":64,"status_explain_ops_per_sec":16653,"status_explain_p95_ns":67765,"live_binding_reload_index_iterations":100000,"live_binding_reload_index_spawn_count":799200,"live_binding_reload_index_ops_per_sec":146724,"live_binding_reload_index_p95_ns":7097,"network_state":"not_modified"}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.

## Rollback

- Restore `runtime.region_distribution()` as the source for the status explain
  region line.
- Restore the intermediate `Vec` collection in `status_base_explain.rs`.
- Remove the `status_explain_region_distribution_counts` bullet from
  `docs/PERFORMANCE.md`.
