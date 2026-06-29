# CHIMERA Mesh Session Handoff: Path Planner Selection Metrics Peer Single Pass

## Saved At

- Timestamp: 2026-06-28

## Active Objective

- Keep speeding the hot metadata/status paths that help nodes publish state,
  explain route and peer decisions, and avoid wasted CPU/RAM.
- Keep sealed transit payload opaque and untouched.

## What Was Done

- Folded path-planner peer summary and stability summary into one pass over
  `selected_peers`.
- Kept exact explain keys, ordering, and redaction shape unchanged.
- Preserved the existing connect summary text while moving it into the merged
  summary pass.
- Marked the old standalone connect summary helpers as test-only so the runtime
  build no longer treats them as dead code.
- Recorded the slice in `docs/PERFORMANCE.md` and a new workflow attestation.

## Validation

- PASS: `cargo fmt --all -- --check`
- PASS: `cargo test -q -p chimera-mesh --tests -- --nocapture`
- PASS: `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- PASS: `just metadata-perf-smoke`
- PASS: `just metadata-perf-smoke-selfcheck`

## Current Snapshot

- `path_planner_candidate_snapshot_ops_per_sec=6865`
- `path_planner_candidate_snapshot_p95_ns=157945`
- `live_dps_plan_path_from_payload_ops_per_sec=3485`
- `live_dps_plan_path_from_payload_p95_ns=305483`
- `status_explain_ops_per_sec=15997`
- `status_explain_p95_ns=69801`

## Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.

## Next Step

- Continue with the remaining explain hot spots around `status_base_explain.rs`
  only after confirming the perf gain justifies the risk; otherwise move to the
  next metadata path with clear redundant scans or allocations.
