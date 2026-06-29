# CHIMERA Metadata Performance Attestation: Table Consistency

## Scope

- Date: 2026-06-27
- Hot path: `table_consistency`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice removes temporary `format!` needle allocations from the compact
  consistency check and removes the temp-vector/join pattern from the runtime
  consistency summary path used by status/report explain.
- It keeps the contract and output format intact.
- The existing `status_explain` benchmark already covers the hot path because
  status/report explain invokes this helper directly.

## Change

- `crates/chimera-mesh/src/runtime/table_consistency.rs`
  - `setup_compact_consistency_summary()` now parses `setup_compact` directly
    instead of building temporary `format!` needles;
  - `consistency_summary()` and `degraded_summary()` now build direct strings
    without an intermediate `Vec`/`join` path;
  - `evaluate_table_consistency()` now builds the warn gate directly from the
    two boolean flags;
  - the function still returns the same summary text and match semantics;
  - added a regression test for exact-match and mismatch behavior.
- `docs/PERFORMANCE.md`
  - records the new `table_consistency` slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh setup_compact_consistency_summary_matches_fields_without_temp_strings`
- `cargo test -q -p chimera-mesh tests_preemptive_status`
- `cargo test -q -p chimera-mesh runtime_status_explain`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output after this slice:

```json
{"status_explain_iterations":10000,"status_explain_peer_count":64,"status_explain_ops_per_sec":17807,"status_explain_p95_ns":56437}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.

## Rollback

- Restore the `contains(&format!(...))` checks in
  `setup_compact_consistency_summary()`.
- Remove the regression test added in `table_consistency.rs`.
- Remove the `table_consistency` bullet from `docs/PERFORMANCE.md`.
