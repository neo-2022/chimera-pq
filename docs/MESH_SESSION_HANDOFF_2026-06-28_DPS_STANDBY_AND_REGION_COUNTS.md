# CHIMERA Mesh Session Handoff: DPS Standby Cleanup and Region Counts

## Saved At

- Timestamp: 2026-06-28

## Active Objective

- Keep speeding the hot metadata/status paths that help nodes publish state,
  explain route and peer decisions, and avoid wasted CPU/RAM.
- Keep sealed transit payload opaque and untouched.

## What Was Done

- Combined DPS standby-shadow cleanup and switch-target redaction into one
  retained explain pass.
- Replaced path-planner selected-region count tree bookkeeping with a small
  vector plus one final normalized-region sort.
- Added a reverse-order unit test so `selected_region_counts` remains sorted
  by normalized region even when selected peers arrive as `us,EU`.
- Preserved explain keys, ordering, redaction shape, and sealed transit payload
  boundaries.

## Validation

- PASS: `cargo fmt --all`
- PASS: `cargo check -q -p chimera-mesh`
- PASS: `cargo test -q -p chimera-mesh tests_standby_shadow -- --nocapture`
- PASS: `cargo test -q -p chimera-mesh tests_dps_explain -- --nocapture`
- PASS: `cargo test -q -p chimera-mesh build_peer_selection_summary_sorts_region_counts_by_normalized_region_not_insertion_order -- --nocapture`
- PASS: `cargo test -q -p chimera-mesh tests_selection_behavior -- --nocapture`
- PASS: `cargo test -q -p chimera-mesh runtime_planning -- --nocapture`
- PASS: `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- PASS: `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- PASS: `just metadata-perf-smoke`
- PASS: `just metadata-perf-smoke-selfcheck`

## Current Snapshot

- `path_planner_candidate_snapshot_ops_per_sec=7417`
- `path_planner_candidate_snapshot_p95_ns=142063`
- `live_dps_plan_path_from_payload_ops_per_sec=4610`
- `live_dps_plan_path_from_payload_p95_ns=231307`
- `status_explain_ops_per_sec=16392`
- `status_explain_p95_ns=63423`

## Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- The remaining DPS summary capture scan is still present.
- The older unused `path_planner_selection_metrics_stability.rs` file should
  be audited before any future work targets it directly.

## Next Step

- Audit whether the remaining `DpsPayloadExplainSnapshot::capture()` scan can
  be replaced by structured fields without changing explain output.
- If not, continue with real path-planner hot code in
  `path_planner_selection_metrics_peer.rs`, not the stale standalone stability
  file.
