# CHIMERA Metadata Performance Attestation: Selection Policy Region Scan

## Scope

- Date: 2026-06-27
- Hot path: `selection_policy_region_scan`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice targets the small region-bookkeeping sets used during candidate
  selection and resilient spread scoring.
- It replaces tree-based region bookkeeping with pre-reserved linear lists.
- It keeps selection order, region-cap behavior, and redaction unchanged.

## Change

- `crates/chimera-mesh/src/runtime/selection_policy_select.rs`
  - uses pre-reserved linear region lists for region-cap bookkeeping instead of
    tree-based maps and sets;
  - keeps selected/backlog order unchanged.
- `crates/chimera-mesh/src/runtime/selection_policy_spread.rs`
  - uses a pre-reserved linear region list for resilient spread counting.
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
{"status":"ok","kind":"metadata_perf_smoke","hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint","lane_document_plan_snapshot_access","live_binding_reload_index","live_dps_plan_path_from_payload","status_explain"],"scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","iterations":100000,"active_bindings":16,"fast_sorted_ops_per_sec":1120814,"slow_sorted_fallback_ops_per_sec":154667,"fast_p95_ns":1019,"slow_sorted_fallback_p95_ns":7308,"fast_vs_fallback_speedup_pct":624.66,"path_planner_iterations":10000,"path_planner_peer_count":64,"path_planner_candidate_snapshot_ops_per_sec":7149,"path_planner_candidate_snapshot_p95_ns":158561,"discovery_rebuild_iterations":10000,"discovery_rebuild_peer_count":64,"discovery_rebuild_fingerprint_ops_per_sec":3751,"discovery_rebuild_fingerprint_p95_ns":270533,"lane_document_plan_snapshot_iterations":10000,"lane_document_plan_snapshot_borrowed_ops_per_sec":66215518,"lane_document_plan_snapshot_borrowed_p95_ns":14,"lane_document_plan_snapshot_owned_ops_per_sec":134595,"lane_document_plan_snapshot_owned_p95_ns":7670,"live_dps_plan_path_from_payload_iterations":10000,"live_dps_plan_path_from_payload_peer_count":64,"live_dps_plan_path_from_payload_ops_per_sec":3630,"live_dps_plan_path_from_payload_p95_ns":286461,"status_explain_iterations":10000,"status_explain_peer_count":64,"status_explain_ops_per_sec":17811,"status_explain_p95_ns":58645,"live_binding_reload_index_iterations":100000,"live_binding_reload_index_spawn_count":799200,"live_binding_reload_index_ops_per_sec":147776,"live_binding_reload_index_p95_ns":6795,"network_state":"not_modified"}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.

## Rollback

- Restore the tree-based `BTreeMap`/`BTreeSet` bookkeeping in
  `selection_policy_select.rs`.
- Restore the tree-based region counting in `selection_policy_spread.rs`.
- Remove the `selection_policy_region_scan` bullet from `docs/PERFORMANCE.md`.
