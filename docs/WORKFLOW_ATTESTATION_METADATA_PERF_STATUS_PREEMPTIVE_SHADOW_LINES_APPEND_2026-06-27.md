# CHIMERA Metadata Performance Attestation: Status Preemptive Shadow Lines Append

## Scope

- Date: 2026-06-27
- Hot path: `status_preemptive_shadow_lines_append`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice removes a temporary standby-status Vec from the preemptive status
  explain path.
- `preemptive_status_lines.rs` now appends standby lines directly into the
  caller buffer.
- Output keys, order, and redaction remain unchanged.

## Change

- `crates/chimera-mesh/src/runtime/standby_status_lines.rs`
  - turns the standby shadow helper into an append-style function.
- `crates/chimera-mesh/src/runtime/preemptive_status_lines.rs`
  - appends standby lines directly instead of extending from a temporary Vec.
- `docs/PERFORMANCE.md`
  - records the new slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh tests_preemptive_status -- --nocapture`
- `cargo test -q -p chimera-mesh runtime_status_explain -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output after this slice:

```json
{"status":"ok","kind":"metadata_perf_smoke","hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint","lane_document_plan_snapshot_access","live_binding_reload_index","live_dps_plan_path_from_payload","status_explain"],"scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","iterations":100000,"active_bindings":16,"fast_sorted_ops_per_sec":1163266,"slow_sorted_fallback_ops_per_sec":150801,"fast_p95_ns":1061,"slow_sorted_fallback_p95_ns":8148,"fast_vs_fallback_speedup_pct":671.39,"path_planner_iterations":10000,"path_planner_peer_count":64,"path_planner_candidate_snapshot_ops_per_sec":6347,"path_planner_candidate_snapshot_p95_ns":180595,"discovery_rebuild_iterations":10000,"discovery_rebuild_peer_count":64,"discovery_rebuild_fingerprint_ops_per_sec":3508,"discovery_rebuild_fingerprint_p95_ns":314883,"lane_document_plan_snapshot_iterations":10000,"lane_document_plan_snapshot_borrowed_ops_per_sec":61571047,"lane_document_plan_snapshot_borrowed_p95_ns":15,"lane_document_plan_snapshot_owned_ops_per_sec":119650,"lane_document_plan_snapshot_owned_p95_ns":12241,"live_dps_plan_path_from_payload_iterations":10000,"live_dps_plan_path_from_payload_peer_count":64,"live_dps_plan_path_from_payload_ops_per_sec":3252,"live_dps_plan_path_from_payload_p95_ns":344872,"status_explain_iterations":10000,"status_explain_peer_count":64,"status_explain_ops_per_sec":16290,"status_explain_p95_ns":72775,"live_binding_reload_index_iterations":100000,"live_binding_reload_index_spawn_count":799200,"live_binding_reload_index_ops_per_sec":136029,"live_binding_reload_index_p95_ns":8275,"network_state":"not_modified"}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.

## Rollback

- Restore the temporary `Vec<String>` return path in
  `standby_status_lines.rs`.
- Restore `lines.extend(...)` in `preemptive_status_lines.rs`.
- Remove the `status_preemptive_shadow_lines_append` bullet from
  `docs/PERFORMANCE.md`.
