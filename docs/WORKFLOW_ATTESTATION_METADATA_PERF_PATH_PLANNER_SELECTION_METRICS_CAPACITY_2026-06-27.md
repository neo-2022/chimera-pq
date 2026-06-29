# CHIMERA Metadata Performance Attestation: Path Planner Selection Metrics Capacity

## Scope

- Date: 2026-06-27
- Hot path: `path_planner_selection_metrics_capacity`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice keeps the existing path-planner selection explain contract intact
  while reducing avoidable string growth in the selection-metrics builders.
- It pre-reserves buffers for peer-region counts, stability summaries,
  selection-pressure summaries, candidate summaries, and the top-level explain
  tail that carries those lines.
- It does not change ordering, keys, or redaction.

## Change

- `crates/chimera-mesh/src/runtime/path_planner_selection_metrics_peer.rs`
  - pre-reserves peer region-count output capacity.
- `crates/chimera-mesh/src/runtime/path_planner_selection_metrics_stability.rs`
  - pre-reserves the stability summary buffers before the per-peer loop;
  - keeps the direct string layout unchanged.
- `crates/chimera-mesh/src/runtime/path_planner_selection_metrics_build.rs`
  - formats `selection_pressure_reason` and `selection_pressure_compact`
    directly into pre-sized buffers.
- `crates/chimera-mesh/src/runtime/path_planner_selection_explain_sections.rs`
  - formats `selection_pressure_summary` and `candidate_summary` directly into
    pre-sized buffers.
- `docs/PERFORMANCE.md`
  - records the new slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh tests_selection_behavior -- --nocapture`
- `cargo test -q -p chimera-mesh tests_auto_profile -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output after this slice:

```json
{"path_planner_candidate_snapshot_ops_per_sec":6866,"path_planner_candidate_snapshot_p95_ns":157724,"live_dps_plan_path_from_payload_ops_per_sec":3621,"live_dps_plan_path_from_payload_p95_ns":280767,"status_explain_ops_per_sec":17655,"status_explain_p95_ns":61406}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.

## Rollback

- Restore `String::with_capacity(...)` back to `String::new()` in the
  selection-metrics builders.
- Restore `format!`-based assembly of selection-pressure and candidate summary
  strings.
- Remove the `path_planner_selection_metrics_capacity` bullet from
  `docs/PERFORMANCE.md`.
