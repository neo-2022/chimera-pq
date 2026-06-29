# CHIMERA Mesh Session Handoff: Auto-Recovery Candidate Collection

## Saved At

- Timestamp: 2026-06-27

## Active Objective

- Keep speeding the hot metadata/status paths that help nodes publish state,
  explain route and peer decisions, and avoid wasted CPU/RAM.
- Keep sealed transit payload opaque and untouched.

## What Was Done

- Pre-reserved the candidate vector in `candidate_filter.rs`.
- Pre-reserved the candidate explain buffer in `candidate_filter.rs`.
- Grew the auto-recovery trace buffer in `path_planner_recovery_steps.rs`.
- Formatted the final auto-recovery trace line without a temporary `String`.
- Switched `path_planner_recovery_explain.rs` to direct line builders for the
  recovery summary lines.
- Built the candidate summary with a pre-sized buffer instead of a temporary
  `format!` allocation.
- Pre-reserved the top-level path-planner explain buffer proportionally to peer
  count.

## Validation

- PASS: `cargo fmt --all -- --check`
- PASS: `cargo test -q -p chimera-mesh tests_selection_behavior -- --nocapture`
- PASS: `cargo test -q -p chimera-mesh tests_auto_profile -- --nocapture`
- PASS: `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- PASS: `just metadata-perf-smoke`
- PASS: `just metadata-perf-smoke-selfcheck`

## Current Snapshot

- `path_planner_candidate_snapshot_ops_per_sec=7108`
- `path_planner_candidate_snapshot_p95_ns=160964`
- `live_dps_plan_path_from_payload_ops_per_sec=3268`
- `live_dps_plan_path_from_payload_p95_ns=336705`
- `status_explain_ops_per_sec=16989`
- `status_explain_p95_ns=63255`

## Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.
