# CHIMERA Mesh Session Handoff: Connect Retry Profile

## Saved At

- Timestamp: 2026-06-27

## Active Objective

- Keep speeding up the hot metadata paths that help nodes find each other,
  choose paths, reconfigure, publish state, and avoid wasted CPU/RAM.
- Keep sealed transit payload opaque and untouched.

## What Was Done

- Rewrote connect priority, retry-plan, and backoff-profile formatting to write
  directly into one `String` buffer instead of building intermediate vectors.
- Kept the existing redaction and ordering contract intact.
- Added a direct connect-priority regression test alongside the existing
  retry-plan regression.
- Benchmarked the result with `just metadata-perf-smoke`.

## Validation

- PASS: `cargo fmt --all -- --check`
- PASS: `cargo test -q -p chimera-mesh retry_plan_redacts_ports_and_keeps_next_peer_chain`
- PASS: `cargo test -q -p chimera-mesh connect_priority_redacts_and_preserves_order`
- PASS: `cargo test -q -p chimera-mesh runtime_planning`
- PASS: `cargo test -q -p chimera-mesh tests_selection_behavior`
- PASS: `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- PASS: `just metadata-perf-smoke`

## Current Snapshot

- `path_planner_candidate_snapshot_ops_per_sec=7453`
- `path_planner_candidate_snapshot_p95_ns=138596`

## Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- The smoke numbers are still noisy enough that one pass is not a proof of a
  large gain.

## Next Step

- Move to the next live metadata summary hotspot only if it still looks worth
  the CPU/alloc cost after measuring larger samples or real load.
