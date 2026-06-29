# CHIMERA Metadata Performance Attestation: Status Explain Tightening

## Scope

- Date: 2026-06-28
- Hot path: `status_explain_tightening`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice removes redundant substring self-scans from the status explain
  contract checks.
- `status_base_explain.rs` now builds region distribution directly in one pass
  instead of using an intermediate `Vec` and `join()`.
- `preemptive_status_lines_sections_confirm.rs` and
  `preemptive_status_lines_sections_validation_tuning.rs` now use direct line
  builders instead of repeated `format!` calls.
- Exact explain text, ordering, and redaction remain unchanged.

## Change

- `crates/chimera-mesh/src/runtime/status_base_explain.rs`
  - builds region distribution directly;
  - drops redundant `summary_matches_fields` substring scans;
  - keeps the same emitted keys and order.
- `crates/chimera-mesh/src/runtime/preemptive_status_lines_sections_confirm.rs`
  - switches confirm lines to direct builders.
- `crates/chimera-mesh/src/runtime/preemptive_status_lines_sections_validation_tuning.rs`
  - switches validation/tuning lines to direct builders.
- `docs/PERFORMANCE.md`
  - records the new slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh tests_preemptive_status -- --nocapture`
- `cargo test -q -p chimera-mesh tests_peer_table_runtime -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output after this slice:

```json
{"status":"ok","kind":"metadata_perf_smoke","status_explain_ops_per_sec":16083,"status_explain_p95_ns":65926,"path_planner_candidate_snapshot_ops_per_sec":6910,"live_dps_plan_path_from_payload_ops_per_sec":3520,"network_state":"not_modified"}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.

## Rollback

- Restore the `collect::<Vec<_>>().join(",")` region distribution builder in
  `status_base_explain.rs`.
- Restore the substring self-scans for `summary_matches_fields` in
  `status_base_explain.rs`.
- Restore the `format!`-based line builders in the confirm/validation tuning
  status sections.
- Remove the `status_explain_tightening` bullet from `docs/PERFORMANCE.md`.
