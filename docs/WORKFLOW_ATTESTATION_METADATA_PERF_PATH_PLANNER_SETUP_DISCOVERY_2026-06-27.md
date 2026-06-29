# CHIMERA Metadata Performance Attestation: Path Planner Setup Discovery Explain

## Scope

- Date: 2026-06-27
- Hot path: `path_planner_setup_discovery_explain`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice stays on the live `plan_path` setup path instead of a late explain
  leaf.
- It removes a pointless source-name clone before join and uses a static
  `join_mode` label instead of `Debug` formatting.
- It reserves explain capacity up front so the plan explain buffer churns less.

## Change

- `crates/chimera-mesh/src/model.rs`
  - adds `MeshJoinMode::label()`.
- `crates/chimera-mesh/src/runtime/path_planner.rs`
  - reserves the top-level explain buffer with `Vec::with_capacity(128)`.
- `crates/chimera-mesh/src/runtime/path_planner_setup_explain_preface.rs`
  - reserves explain capacity before appending preface and shadow lines.
- `crates/chimera-mesh/src/runtime/path_planner_setup_explain_discovery.rs`
  - uses `MeshJoinMode::label()`;
  - joins discovery source names directly from the runtime source set;
  - avoids the extra `source_list()` clone path;
  - keeps the existing explain keys and redaction shape.
- `crates/chimera-mesh/src/runtime/plan_orchestration_discovery_table.rs`
  - reuses the same `join_mode` label helper for consistency.
- `docs/PERFORMANCE.md`
  - records the new `path_planner_setup_discovery_explain` slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh tests_selection_behavior`
- `cargo test -q -p chimera-mesh runtime_planning`
- `cargo test -q -p chimera-mesh tests_dps_explain`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo clippy -q -p chimera-lab --all-targets -- -D warnings`
- `just metadata-perf-smoke`

`just metadata-perf-smoke` output after this slice:

```json
{"path_planner_candidate_snapshot_ops_per_sec":7367,"path_planner_candidate_snapshot_p95_ns":138986}
```

Previous saved snapshot in the handoff was:

```json
{"path_planner_candidate_snapshot_ops_per_sec":7267,"path_planner_candidate_snapshot_p95_ns":146409}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Further explain-path tuning may still exist if larger samples or real load
  show a new hotspot.

## Rollback

- Restore `Vec::new()` in `path_planner.rs`.
- Restore the `Debug`-formatted `join_mode` strings.
- Restore `runtime.source_list()` in the discovery explain path.
- Remove the added `reserve()` calls and the `MeshJoinMode::label()` helper.
