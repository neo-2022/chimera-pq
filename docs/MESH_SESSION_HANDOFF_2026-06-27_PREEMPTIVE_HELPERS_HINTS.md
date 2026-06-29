# CHIMERA Mesh Session Handoff: Preemptive Helpers Hints

## Saved At

- Timestamp: 2026-06-27

## Active Objective

- Keep speeding the hot metadata/status paths that help nodes publish state,
  explain route and peer decisions, and avoid wasted CPU/RAM.
- Keep sealed transit payload opaque and untouched.

## What Was Done

- Switched `preemptive_helpers_hints.rs` to direct sized-buffer formatting for
  the shared hints summary helper.
- Kept the exact summary strings and source labels unchanged.
- Recorded the slice in `docs/PERFORMANCE.md`.
- Benchmarked the change with `just metadata-perf-smoke` twice and
  `just metadata-perf-smoke-selfcheck`.

## Validation

- PASS: `cargo fmt --all -- --check`
- PASS: `cargo test -q -p chimera-mesh tests_dps_explain -- --nocapture`
- PASS: `cargo test -q -p chimera-mesh runtime_status_explain -- --nocapture`
- PASS: `cargo test -q -p chimera-mesh tests_selection_behavior -- --nocapture`
- PASS: `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- PASS: `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- PASS: `just metadata-perf-smoke`
- PASS: `just metadata-perf-smoke`
- PASS: `just metadata-perf-smoke-selfcheck`

## Current Snapshot

- `path_planner_candidate_snapshot_ops_per_sec=7251`
- `path_planner_candidate_snapshot_p95_ns=145467`
- `live_dps_plan_path_from_payload_ops_per_sec=3412`
- `live_dps_plan_path_from_payload_p95_ns=314270`
- `status_explain_ops_per_sec=17237`
- `status_explain_p95_ns=62218`

## Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.

## Next Step

- Move to the next measured metadata hotspot only if the current smoke band
  still looks worth mining; otherwise stop squeezing this shared helper.
