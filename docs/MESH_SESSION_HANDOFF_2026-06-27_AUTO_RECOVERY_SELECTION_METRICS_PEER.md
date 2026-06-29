# CHIMERA Mesh Session Handoff: Auto-Recovery Selection Metrics Peer

## Saved At

- Timestamp: 2026-06-27

## Active Objective

- Keep speeding the hot metadata/status paths that help nodes publish state,
  explain route and peer decisions, and avoid wasted CPU/RAM.
- Keep sealed transit payload opaque and untouched.

## What Was Done

- Refactored `auto_recovery/selection_metrics_peer.rs` to gather ids, regions,
  endpoints, scores, sums, averages, and region counts in one pass.
- Kept exact output text, order, and redaction unchanged.
- Removed the extra average helper re-export from `selection_metrics.rs`.

## Validation

- PASS: `cargo fmt --all -- --check`
- PASS: `cargo test -q -p chimera-mesh tests_selection_behavior -- --nocapture`
- PASS: `cargo test -q -p chimera-mesh tests_auto_profile -- --nocapture`
- PASS: `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- PASS: `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- PASS: `just metadata-perf-smoke`
- PASS: `just metadata-perf-smoke-selfcheck`

## Current Snapshot

- `path_planner_candidate_snapshot_ops_per_sec=6440`
- `path_planner_candidate_snapshot_p95_ns=180869`
- `live_dps_plan_path_from_payload_ops_per_sec=3414`
- `live_dps_plan_path_from_payload_p95_ns=319524`
- `status_explain_ops_per_sec=16941`
- `status_explain_p95_ns=73468`

## Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.
