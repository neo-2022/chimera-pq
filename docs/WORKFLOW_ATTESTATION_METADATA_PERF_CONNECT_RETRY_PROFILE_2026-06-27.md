# CHIMERA Metadata Performance Attestation: Connect Retry Profile

## Scope

- Date: 2026-06-27
- Hot path: `connect_retry_profile`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice sits on the measured `path_planner_candidate_snapshot` path.
- It removes repeated `Vec` collection and `join()` work from connect retry
  metadata formatting.
- The output remains redacted and ordered exactly as before.

## Change

- `crates/chimera-mesh/src/runtime/connect_retry_profile.rs`
  - `build_connect_priority()` now writes directly into one buffer;
  - `build_connect_retry_plan()` now writes directly into one buffer;
  - `build_connect_backoff_profile()` now writes directly into one buffer;
  - added a direct redaction/order regression for connect priority;
  - preserved the existing retry-plan redaction regression.
- `docs/PERFORMANCE.md`
  - records the new `connect_retry_profile` slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh retry_plan_redacts_ports_and_keeps_next_peer_chain`
- `cargo test -q -p chimera-mesh connect_priority_redacts_and_preserves_order`
- `cargo test -q -p chimera-mesh runtime_planning`
- `cargo test -q -p chimera-mesh tests_selection_behavior`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `just metadata-perf-smoke`

`just metadata-perf-smoke` output after this slice:

```json
{"path_planner_candidate_snapshot_ops_per_sec":7453,"path_planner_candidate_snapshot_p95_ns":138596}
```

Previous saved snapshot in the handoff was:

```json
{"path_planner_candidate_snapshot_ops_per_sec":7511,"path_planner_candidate_snapshot_p95_ns":139660}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- The smoke numbers are still in the same noisy band, so larger samples may be
  needed to prove the size of the gain.

## Rollback

- Restore the `Vec` collection and `join()` builders in `connect_retry_profile.rs`.
- Remove the new direct redaction test.
- Remove the `connect_retry_profile` bullet from `docs/PERFORMANCE.md`.
