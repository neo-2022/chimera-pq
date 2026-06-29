# CHIMERA Metadata Performance Attestation: Standby Shadow Explain Snapshot

## Scope

- Date: 2026-06-27
- Hot path: `standby_shadow_explain_snapshot`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice stays on the live standby-shadow explain path used by both
  `plan_path` rendering and DPS adaptation.
- It captures the needed preemptive-shadow fields once per call instead of
  rescanning `explain` repeatedly.
- It keeps standby output ordering and redaction unchanged.

## Change

- `crates/chimera-mesh/src/runtime/standby_shadow_explain_common.rs`
  - adds a small owned snapshot for preemptive-shadow explain fields;
  - removes the old repeated-scan helper.
- `crates/chimera-mesh/src/runtime/standby_shadow_explain_render.rs`
  - reads from the snapshot instead of rescanning `explain`.
- `crates/chimera-mesh/src/runtime/standby_shadow_explain_adapt.rs`
  - reads from the snapshot instead of rescanning `explain`.
- `crates/chimera-mesh/src/runtime/standby_shadow_explain.rs`
  - captures the snapshot once per call and passes it to render/adapt.
- `docs/PERFORMANCE.md`
  - records the new `standby_shadow_explain_snapshot` slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh tests_dps_explain`
- `cargo test -q -p chimera-mesh runtime_planning`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `just metadata-perf-smoke`

`just metadata-perf-smoke` output after this slice:

```json
{"live_dps_plan_path_from_payload_ops_per_sec":3537,"live_dps_plan_path_from_payload_p95_ns":293909}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/datapath checks remain SSH-stand work.
- Further standby / explain hot spots may still exist.

## Rollback

- Restore the old repeated `explain_value` scans in the standby shadow explain
  render/adapt paths.
- Restore the old wrapper pass-through behavior in `standby_shadow_explain.rs`.
- Remove the `standby_shadow_explain_snapshot` bullet from
  `docs/PERFORMANCE.md`.
