# CHIMERA Metadata Performance Attestation: DPS Standby Shadow Cleanup Single Pass

## Scope

- Date: 2026-06-28
- Hot path: `live_dps_plan_path_from_payload`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- The current live DPS handoff called out remaining rescans around DPS explain
  cleanup and standby-shadow adaptation.
- DPS standby adaptation previously removed old `standby_shadow_*` lines and
  then rescanned explain to redact `preemptive_shadow_switch_target`.
- This slice keeps the same explain output contract while combining those two
  passes.

## Change

- `crates/chimera-mesh/src/runtime/standby_shadow_explain_common.rs`
  - adds `remove_and_redact_explain_keys()` using one `retain_mut` pass;
  - preserves remaining-line order;
  - keeps switch-target redaction fail-closed.
- `crates/chimera-mesh/src/runtime/standby_shadow_explain_adapt.rs`
  - uses the combined helper in DPS standby adaptation.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all`
- `cargo check -q -p chimera-mesh`
- `cargo test -q -p chimera-mesh tests_standby_shadow -- --nocapture`
- `cargo test -q -p chimera-mesh tests_dps_explain -- --nocapture`
- `cargo test -q -p chimera-mesh runtime_planning -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output after this slice and the selected-region
counts slice:

```json
{"status":"ok","kind":"metadata_perf_smoke","hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint","lane_document_plan_snapshot_access","live_binding_reload_index","live_dps_plan_path_from_payload","status_explain"],"scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","iterations":100000,"active_bindings":16,"fast_sorted_ops_per_sec":1200703,"slow_sorted_fallback_ops_per_sec":152532,"fast_p95_ns":947,"slow_sorted_fallback_p95_ns":7966,"fast_vs_fallback_speedup_pct":687.18,"path_planner_iterations":10000,"path_planner_peer_count":64,"path_planner_candidate_snapshot_ops_per_sec":7417,"path_planner_candidate_snapshot_p95_ns":142063,"discovery_rebuild_iterations":10000,"discovery_rebuild_peer_count":64,"discovery_rebuild_fingerprint_ops_per_sec":3734,"discovery_rebuild_fingerprint_p95_ns":277086,"lane_document_plan_snapshot_iterations":10000,"lane_document_plan_snapshot_borrowed_ops_per_sec":66118763,"lane_document_plan_snapshot_borrowed_p95_ns":14,"lane_document_plan_snapshot_owned_ops_per_sec":129238,"lane_document_plan_snapshot_owned_p95_ns":8185,"live_dps_plan_path_from_payload_iterations":10000,"live_dps_plan_path_from_payload_peer_count":64,"live_dps_plan_path_from_payload_ops_per_sec":4610,"live_dps_plan_path_from_payload_p95_ns":231307,"status_explain_iterations":10000,"status_explain_peer_count":64,"status_explain_ops_per_sec":16392,"status_explain_p95_ns":63423,"live_binding_reload_index_iterations":100000,"live_binding_reload_index_spawn_count":799200,"live_binding_reload_index_ops_per_sec":148677,"live_binding_reload_index_p95_ns":6843,"network_state":"not_modified"}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- The remaining DPS summary capture scan is not removed by this slice.

## Rollback

- Restore separate `remove_explain_keys()` and `redact_preemptive_switch_target()`
  calls in `standby_shadow_explain_adapt.rs`.
- Remove the `dps_standby_shadow_cleanup_single_pass` bullet from
  `docs/PERFORMANCE.md`.
