# CHIMERA Metadata Performance Attestation: Status Report Builder Shadow

## Scope

- Date: 2026-06-27
- Hot path: `status_report_builder_shadow`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice stays inside the status report builder that feeds `status_explain`.
- It removes repeated `format!` churn from the short shadow summary builders.
- It keeps the same public strings and ordering while reusing the tuple-based
  compact-consistency helper in the report builder.

## Change

- `crates/chimera-mesh/src/runtime/status_report_builder.rs`
  - keeps the shared `setup_compact_consistency()` tuple helper in the report
    builder path.
- `crates/chimera-mesh/src/runtime/status_report_builder_shadow.rs`
  - formats risk, guard, confirm state, and confirm summary strings directly
    into sized buffers instead of `format!`.
- `docs/PERFORMANCE.md`
  - records the new `status_report_builder_shadow` slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh runtime_status_explain -- --nocapture`
- `cargo test -q -p chimera-mesh tests_preemptive_status -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` outputs after this slice:

```json
{"status_explain_ops_per_sec":16877,"status_explain_p95_ns":70481,"live_dps_plan_path_from_payload_ops_per_sec":3349,"live_dps_plan_path_from_payload_p95_ns":327758,"path_planner_candidate_snapshot_ops_per_sec":7084,"path_planner_candidate_snapshot_p95_ns":160304}
```

```json
{"status_explain_ops_per_sec":16071,"status_explain_p95_ns":78107,"live_dps_plan_path_from_payload_ops_per_sec":3410,"live_dps_plan_path_from_payload_p95_ns":321122,"path_planner_candidate_snapshot_ops_per_sec":7014,"path_planner_candidate_snapshot_p95_ns":163881}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.

## Rollback

- Restore `format!` in `status_report_builder_shadow.rs`.
- Remove the `status_report_builder_shadow` bullet from `docs/PERFORMANCE.md`.
