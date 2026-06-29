# CHIMERA Metadata Performance Attestation: Path Planner Selection Explain Capacity

## Scope

- Date: 2026-06-27
- Hot path: `path_planner_selection_explain_capacity`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- `plan_path` builds a long explain tail after candidate selection.
- Reserving the tail capacity up front reduces `Vec` growth in the hot
  selection-finalization path.
- The existing `path_planner_candidate_snapshot` benchmark already covers the
  path end-to-end.

## Change

- `crates/chimera-mesh/src/runtime/path_planner_finalize.rs`
  - reserves 47 extra explain slots before appending selection and candidate
    metadata;
  - preserves explain ordering, keys, and redaction.
- `docs/PERFORMANCE.md`
  - records the new selection explain capacity slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh planning`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output after this slice:

```json
{"path_planner_candidate_snapshot_ops_per_sec":6828,"path_planner_candidate_snapshot_p95_ns":160999}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.

## Rollback

- Remove the `explain.reserve(47)` line from `path_planner_finalize.rs`.
- Remove the bullet from `docs/PERFORMANCE.md`.
