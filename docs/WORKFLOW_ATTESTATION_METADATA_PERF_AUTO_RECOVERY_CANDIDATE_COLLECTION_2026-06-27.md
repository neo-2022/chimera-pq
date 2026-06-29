# CHIMERA Metadata Performance Attestation: Auto-Recovery Candidate Collection

## Scope

- Date: 2026-06-27
- Hot path: `auto_recovery_candidate_collection`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice targets the recovery branch inside the path-planner hot path.
- It removes avoidable allocation pressure by:
  - pre-reserving the candidate vector to peer count;
  - pre-reserving the candidate explain buffer to peer count;
  - growing the auto-recovery trace buffer so long recovery traces do not
    reallocate as early;
  - formatting the candidate summary, recovery explain lines, and
    auto-recovery trace directly instead of building temporary short-lived
    strings.
- The explain contract, key order, and redaction remain unchanged.

## Change

- `crates/chimera-mesh/src/runtime/candidate_filter.rs`
  - initializes the candidate vector with `Vec::with_capacity(peers.len())`;
  - reserves explain capacity up front for the candidate scan;
  - keeps candidate acceptance/rejection text unchanged.
- `crates/chimera-mesh/src/runtime/path_planner_recovery_steps.rs`
  - increases the auto-recovery trace buffer capacity.
- `crates/chimera-mesh/src/runtime/path_planner_recovery_explain.rs`
  - formats the recovery explain lines and final trace line without temporary
    `String` churn.
- `crates/chimera-mesh/src/runtime/auto_recovery/selection_explain_counters.rs`
  - reserves explain capacity up front;
  - builds `candidate_summary` with a pre-sized `String` buffer.
- `crates/chimera-mesh/src/runtime/path_planner.rs`
  - reserves extra explain capacity proportional to peer count.
- `docs/PERFORMANCE.md`
  - records the new slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh tests_selection_behavior -- --nocapture`
- `cargo test -q -p chimera-mesh tests_auto_profile -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output after this slice:

```json
{"path_planner_candidate_snapshot_ops_per_sec":7108,"path_planner_candidate_snapshot_p95_ns":160964,"live_dps_plan_path_from_payload_ops_per_sec":3268,"live_dps_plan_path_from_payload_p95_ns":336705,"status_explain_ops_per_sec":16989,"status_explain_p95_ns":63255}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.

## Rollback

- Restore the `Vec::new()` allocation in `candidate_filter.rs`.
- Restore the smaller trace buffer in `path_planner_recovery_steps.rs`.
- Restore the temporary `String` path in `path_planner_recovery_explain.rs`.
- Restore the old `format!` candidate summary in `selection_explain_counters.rs`.
- Restore the previous explain capacity in `path_planner.rs`.
