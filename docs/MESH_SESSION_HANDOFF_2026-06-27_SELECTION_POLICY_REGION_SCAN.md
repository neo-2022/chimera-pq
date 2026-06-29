# CHIMERA Mesh Session Handoff: Selection Policy Region Scan

## Saved At

- Timestamp: 2026-06-27

## Active Objective

- Keep speeding the hot metadata/status paths that help nodes publish state,
  explain route and peer decisions, and avoid wasted CPU/RAM.
- Keep sealed transit payload opaque and untouched.

## What Was Done

- Reworked selection-policy region bookkeeping to use pre-reserved linear
  region lists instead of tree-based maps and sets.
- Kept selection ordering, region-cap behavior, and redaction unchanged.

## Validation

- PASS: `cargo fmt --all -- --check`
- PASS: `cargo test -q -p chimera-mesh tests_selection_behavior -- --nocapture`
- PASS: `cargo test -q -p chimera-mesh runtime_planning -- --nocapture`
- PASS: `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- PASS: `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- PASS: `just metadata-perf-smoke`
- PASS: `just metadata-perf-smoke-selfcheck`

## Current Snapshot

- `path_planner_candidate_snapshot_ops_per_sec=7149`
- `path_planner_candidate_snapshot_p95_ns=158561`
- `live_dps_plan_path_from_payload_ops_per_sec=3630`
- `live_dps_plan_path_from_payload_p95_ns=286461`
- `status_explain_ops_per_sec=17811`
- `status_explain_p95_ns=58645`

## Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.
