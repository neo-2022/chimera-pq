# CHIMERA Mesh Session Handoff: Standby Shadow Redaction

## Saved At

- Timestamp: 2026-06-27

## Active Objective

- Keep speeding up the hot metadata paths that help nodes find each other,
  choose paths, reconfigure, publish state, and avoid wasted CPU/RAM.
- Keep sealed transit payload opaque and untouched.

## What Was Done

- Removed the intermediate `selected_peer_ids` clone from standby shadow
  redaction.
- Switched standby target redaction to work directly on `&[MeshPeerState]`.
- Kept the same public labels and the same fail-closed redaction behavior.
- Updated both standby explain call sites to pass selected peers directly.
- Benchmarked the result with `just metadata-perf-smoke`.

## Validation

- PASS: `cargo fmt --all -- --check`
- PASS: `cargo test -q -p chimera-mesh tests_standby_shadow`
- PASS: `cargo test -q -p chimera-mesh tests_preemptive_status`
- PASS: `cargo test -q -p chimera-mesh tests_dps_explain`
- PASS: `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- PASS: `just metadata-perf-smoke`

## Current Snapshot

- `path_planner_candidate_snapshot_ops_per_sec=7511`
- `path_planner_candidate_snapshot_p95_ns=139660`

## Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Explain-summary scanning remains another open metadata hotspot.

## Next Step

- Profile the next live summary hotspot, most likely `dps_payload_explain_summary`
  or the heavier status explain path, and keep the transit payload sealed.
