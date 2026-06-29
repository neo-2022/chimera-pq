# CHIMERA Metadata Performance Attestation: DPS Payload Explain Summary

## Scope

- Date: 2026-06-27
- Hot path: `dps_payload_explain_summary`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice stays on the live `live_dps_plan_path_from_payload` path.
- It removes repeated `format!` churn from the decision and standby summary
  appenders.
- It reuses one hints-summary string across both hint branches and keeps the
  explain key order and redaction shape unchanged.

## Change

- `crates/chimera-mesh/src/runtime/preemptive_helpers_hints.rs`
  - formats hints summaries with direct buffer builders;
  - keeps `format_hints_summary_with_source()` on the same output contract.
- `crates/chimera-mesh/src/runtime/dps_payload_explain_hints.rs`
  - switches hint explain lines to direct push helpers;
  - reuses one hints summary string across the preemptive and DPS branches;
  - keeps the same keys and order.
- `crates/chimera-mesh/src/runtime/dps_payload_explain_summary.rs`
  - switches decision and standby summary lines to direct push helpers;
  - reserves explain space up front;
  - keeps the same key order and redaction.
- `docs/PERFORMANCE.md`
  - records the new `dps_payload_explain_summary` slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh tests_dps_explain -- --nocapture`
- `cargo test -q -p chimera-mesh runtime_planning -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

Current smoke snapshot from the second run:

```json
{"path_planner_candidate_snapshot_ops_per_sec":6835,"path_planner_candidate_snapshot_p95_ns":161877,"live_dps_plan_path_from_payload_ops_per_sec":3499,"live_dps_plan_path_from_payload_p95_ns":301717,"status_explain_ops_per_sec":16409,"status_explain_p95_ns":66639}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.

## Rollback

- Restore the repeated `format!` lines in `dps_payload_explain_hints.rs` and
  `dps_payload_explain_summary.rs`.
- Restore the old `format!` summary builders in
  `preemptive_helpers_hints.rs`.
- Remove the new `dps_payload_explain_summary` bullet from
  `docs/PERFORMANCE.md`.
