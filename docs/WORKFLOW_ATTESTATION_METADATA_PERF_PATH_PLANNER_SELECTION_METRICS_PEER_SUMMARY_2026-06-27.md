# CHIMERA Metadata Performance Attestation: Path Planner Selection Metrics Peer Summary

## Scope

- Date: 2026-06-27
- Hot path: `path_planner_selection_metrics_peer_summary`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice reduces the number of passes over `selected_peers` while building
  the peer-selection explain payload.
- It keeps the exact selected-peer order, keys, and redaction contract intact.
- It uses a small shared label helper instead of repeating `format!`
  allocations for redacted peer and endpoint labels.

## Change

- `crates/chimera-mesh/src/runtime/path_planner_selection_metrics_peer.rs`
  - builds peer ids, regions, endpoints, scores, sums, and region counts in
    one pass over `selected_peers`;
  - keeps the output text and order unchanged;
  - removes the old multi-pass `join_selected` helper.
- `crates/chimera-mesh/src/runtime/path_planner_selection_metrics_format.rs`
  - keeps the shared redacted label helpers used by the peer-summary builder.
- `docs/PERFORMANCE.md`
  - records the new slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh tests_selection_behavior -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output after this slice:

```json
{"path_planner_candidate_snapshot_ops_per_sec":6955,"path_planner_candidate_snapshot_p95_ns":155094,"live_dps_plan_path_from_payload_ops_per_sec":3556,"live_dps_plan_path_from_payload_p95_ns":289852,"status_explain_ops_per_sec":17123,"status_explain_p95_ns":61958}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.

## Rollback

- Restore the multi-pass `join_selected` approach in
  `path_planner_selection_metrics_peer.rs`.
- Remove the `path_planner_selection_metrics_peer_summary` bullet from
  `docs/PERFORMANCE.md`.
