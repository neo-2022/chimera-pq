# CHIMERA Metadata Performance Attestation: Standby Shadow Redaction

## Scope

- Date: 2026-06-27
- Hot path: `standby_shadow_redaction`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice sits on the live `plan_path` explain path.
- It removes a full `selected_peer_ids` clone before standby redaction.
- Redaction now searches the selected peer slice directly and still keeps the
  same public labels and fail-closed behavior.

## Change

- `crates/chimera-mesh/src/runtime/standby_shadow_target.rs`
  - accepts `&[MeshPeerState]` instead of `&[String]`;
  - clones only the final chosen target string.
- `crates/chimera-mesh/src/runtime/standby_shadow_explain_common.rs`
  - redacts standby targets and preemptive shadow switch targets directly from
    the selected peer slice;
  - removes the intermediate selected-peer-id list helper.
- `crates/chimera-mesh/src/runtime/standby_shadow_explain_render.rs`
  - passes selected peers directly into standby redaction.
- `crates/chimera-mesh/src/runtime/standby_shadow_explain_adapt.rs`
  - passes selected peers directly into standby redaction.
- `docs/PERFORMANCE.md`
  - records the new `standby_shadow_redaction` slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh tests_standby_shadow`
- `cargo test -q -p chimera-mesh tests_preemptive_status`
- `cargo test -q -p chimera-mesh tests_dps_explain`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `just metadata-perf-smoke`

`just metadata-perf-smoke` output after this slice:

```json
{"path_planner_candidate_snapshot_ops_per_sec":7511,"path_planner_candidate_snapshot_p95_ns":139660}
```

Previous saved snapshot in the handoff was:

```json
{"path_planner_candidate_snapshot_ops_per_sec":7367,"path_planner_candidate_snapshot_p95_ns":138986}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Explain summary code still has heavier repeated scanning in other paths.

## Rollback

- Restore `selected_peer_ids()` and the old `&[String]` redaction signatures.
- Restore the clone-based redaction calls in standby explain render/adapt.
- Remove the `standby_shadow_redaction` bullet from `docs/PERFORMANCE.md`.
