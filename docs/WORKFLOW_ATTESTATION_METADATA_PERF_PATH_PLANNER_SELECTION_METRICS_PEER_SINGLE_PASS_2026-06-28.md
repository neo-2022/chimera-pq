# CHIMERA Metadata Performance Attestation: Path Planner Selection Metrics Peer Single Pass

## Scope

- Date: 2026-06-28
- Hot path: `path_planner_selection_metrics_peer_single_pass`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice removes the separate stability scan from the path-planner peer
  summary path.
- `path_planner_selection_metrics_peer.rs` now folds peer identity, region
  counts, connect summaries, and stability counters into one pass over
  `selected_peers`.
- Exact explain keys, ordering, and redaction remain unchanged.

## Change

- `crates/chimera-mesh/src/runtime/path_planner_selection_metrics_peer.rs`
  - merges peer-summary and stability-summary collection into one pass;
  - keeps selected-peer redaction and stability redaction shape intact;
  - keeps connect summary text stable.
- `crates/chimera-mesh/src/runtime/path_planner_selection_metrics_build.rs`
  - consumes the merged summary directly.
- `crates/chimera-mesh/src/runtime/connect_retry_profile.rs`
  - marks the direct connect summary helpers as test-only runtime helpers so
    clippy no longer treats them as dead runtime code.
- `docs/PERFORMANCE.md`
  - records the new slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh --tests -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output after this slice:

```json
{"status":"ok","kind":"metadata_perf_smoke","hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint","lane_document_plan_snapshot_access","live_binding_reload_index","live_dps_plan_path_from_payload","status_explain"],"scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","iterations":100000,"active_bindings":16,"fast_sorted_ops_per_sec":1147917,"slow_sorted_fallback_ops_per_sec":156397,"fast_p95_ns":1277,"slow_sorted_fallback_p95_ns":6771,"fast_vs_fallback_speedup_pct":633.98,"path_planner_iterations":10000,"path_planner_peer_count":64,"path_planner_candidate_snapshot_ops_per_sec":6865,"path_planner_candidate_snapshot_p95_ns":157945,"discovery_rebuild_iterations":10000,"discovery_rebuild_peer_count":64,"discovery_rebuild_fingerprint_ops_per_sec":3706,"discovery_rebuild_fingerprint_p95_ns":277484,"lane_document_plan_snapshot_iterations":10000,"lane_document_plan_snapshot_borrowed_ops_per_sec":63555013,"lane_document_plan_snapshot_borrowed_p95_ns":14,"lane_document_plan_snapshot_owned_ops_per_sec":132440,"lane_document_plan_snapshot_owned_p95_ns":7767,"live_dps_plan_path_from_payload_iterations":10000,"live_dps_plan_path_from_payload_peer_count":64,"live_dps_plan_path_from_payload_ops_per_sec":3485,"live_dps_plan_path_from_payload_p95_ns":305483,"status_explain_iterations":10000,"status_explain_peer_count":64,"status_explain_ops_per_sec":15997,"status_explain_p95_ns":69801,"live_binding_reload_index_iterations":100000,"live_binding_reload_index_spawn_count":799200,"live_binding_reload_index_ops_per_sec":148200,"live_binding_reload_index_p95_ns":7056,"network_state":"not_modified"}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.

## Rollback

- Restore the separate stability builder in
  `path_planner_selection_metrics_stability.rs`.
- Restore the two-pass composition in
  `path_planner_selection_metrics_build.rs`.
- Remove the `path_planner_selection_metrics_peer_single_pass` bullet from
  `docs/PERFORMANCE.md`.
