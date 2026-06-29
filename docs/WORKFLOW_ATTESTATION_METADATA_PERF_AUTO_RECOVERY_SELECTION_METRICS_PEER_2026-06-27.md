# CHIMERA Metadata Performance Attestation: Auto-Recovery Selection Metrics Peer

## Scope

- Date: 2026-06-27
- Hot path: `auto_recovery_selection_metrics_peer`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice removes repeated passes over selected peers when building
  auto-recovery selection metadata.
- It builds peer ids, regions, endpoints, scores, score/reliability/load sums,
  and region counts in one pass with direct String buffers.
- It keeps the existing output order, redaction, and explain contract intact.

## Change

- `crates/chimera-mesh/src/runtime/auto_recovery/selection_metrics_peer.rs`
  - builds selected peer ids, regions, endpoints, scores, sums, and region
    counts in one pass;
  - uses pre-sized String buffers and direct label pushes instead of
    `Vec + join`;
  - formats region counts directly from the accumulated map.
- `crates/chimera-mesh/src/runtime/auto_recovery/selection_metrics.rs`
  - drops the no-longer-needed average helper re-export.
- `docs/PERFORMANCE.md`
  - records the new slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh tests_selection_behavior -- --nocapture`
- `cargo test -q -p chimera-mesh tests_auto_profile -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output after this slice:

```json
{"status":"ok","kind":"metadata_perf_smoke","hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint","lane_document_plan_snapshot_access","live_binding_reload_index","live_dps_plan_path_from_payload","status_explain"],"scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","iterations":100000,"active_bindings":16,"fast_sorted_ops_per_sec":1107644,"slow_sorted_fallback_ops_per_sec":154604,"fast_p95_ns":1303,"slow_sorted_fallback_p95_ns":7604,"fast_vs_fallback_speedup_pct":616.44,"path_planner_iterations":10000,"path_planner_peer_count":64,"path_planner_candidate_snapshot_ops_per_sec":6440,"path_planner_candidate_snapshot_p95_ns":180869,"discovery_rebuild_iterations":10000,"discovery_rebuild_peer_count":64,"discovery_rebuild_fingerprint_ops_per_sec":3492,"discovery_rebuild_fingerprint_p95_ns":319260,"lane_document_plan_snapshot_iterations":10000,"lane_document_plan_snapshot_borrowed_ops_per_sec":65852293,"lane_document_plan_snapshot_borrowed_p95_ns":14,"lane_document_plan_snapshot_owned_ops_per_sec":129525,"lane_document_plan_snapshot_owned_p95_ns":9503,"live_dps_plan_path_from_payload_iterations":10000,"live_dps_plan_path_from_payload_peer_count":64,"live_dps_plan_path_from_payload_ops_per_sec":3414,"live_dps_plan_path_from_payload_p95_ns":319524,"status_explain_iterations":10000,"status_explain_peer_count":64,"status_explain_ops_per_sec":16941,"status_explain_p95_ns":73468,"live_binding_reload_index_iterations":100000,"live_binding_reload_index_spawn_count":799200,"live_binding_reload_index_ops_per_sec":145207,"live_binding_reload_index_p95_ns":7357,"network_state":"not_modified"}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.

## Rollback

- Restore the multi-pass `Vec + join` path in
  `auto_recovery/selection_metrics_peer.rs`.
- Restore the separate average and region-count passes.
- Remove the `auto_recovery_selection_metrics_peer` bullet from
  `docs/PERFORMANCE.md`.
