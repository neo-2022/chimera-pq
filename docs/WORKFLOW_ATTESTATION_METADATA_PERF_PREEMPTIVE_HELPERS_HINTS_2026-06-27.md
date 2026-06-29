# CHIMERA Metadata Performance Attestation: Preemptive Helpers Hints

## Scope

- Date: 2026-06-27
- Hot path: `preemptive_helpers_hints`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This shared helper feeds both status explain and DPS explain paths.
- It removes `format!`/formatting-engine churn from the short hints summary
  builders while keeping the exact string contract.
- The helper remains fully redacted and does not touch sealed transit payload.

## Change

- `crates/chimera-mesh/src/runtime/preemptive_helpers_hints.rs`
  - formats the base hints summary directly into a sized buffer;
  - appends the source label without temporary `String` churn;
  - keeps `format_hints_summary()` and `format_hints_summary_with_source()`
    output unchanged.
- `docs/PERFORMANCE.md`
  - records the new `preemptive_helpers_hints` slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh tests_dps_explain -- --nocapture`
- `cargo test -q -p chimera-mesh runtime_status_explain -- --nocapture`
- `cargo test -q -p chimera-mesh tests_selection_behavior -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

Current smoke snapshot:

```json
{"path_planner_candidate_snapshot_ops_per_sec":7251,"path_planner_candidate_snapshot_p95_ns":145467,"live_dps_plan_path_from_payload_ops_per_sec":3412,"live_dps_plan_path_from_payload_p95_ns":314270,"status_explain_ops_per_sec":17237,"status_explain_p95_ns":62218}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.

## Rollback

- Restore the `format!` builder in `preemptive_helpers_hints.rs`.
- Remove the `preemptive_helpers_hints` bullet from `docs/PERFORMANCE.md`.
