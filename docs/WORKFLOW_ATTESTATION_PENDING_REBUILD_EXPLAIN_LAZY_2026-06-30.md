# CHIMERA Metadata Performance Attestation: Pending Rebuild Explain Lazy

## Scope

- Date: 2026-06-30
- Workline: metadata/control performance
- Status: Lab/Metadata PASS
- Scope: pending multipath rebuild decision/explain metadata
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Why This Batch

This batch targets a measured control-plane hotspot where pending rebuild full
planning is much slower than the core path. The narrow root cause addressed
here is avoidable explain/copy churn:

- `MeshMultipathRebuildDecision` built and stored explain strings for every
  decision, even when the core path did not need plan diagnostics;
- full path then cloned those explain strings into `MeshPathPlan.explain`;
- decision also copied static policy/privacy labels into owned strings.

The change keeps full diagnostic output available while avoiding permanent
explain storage in every decision.

## Changes

- `MeshMultipathRebuildDecision` no longer stores a `Vec<String>` explain
  payload.
- Static rebuild policy/privacy labels are no longer copied into every decision.
- Added `append_explain_to` and `explain()` so explain text is built only when
  diagnostics/tests request it.
- Full plan rebuild paths append explain directly into `MeshPathPlan.explain`
  instead of cloning from a decision-owned vector.
- Existing tests now call `decision.explain()` only when they explicitly verify
  diagnostic text.
- Added a dedicated pending rebuild regression module covering:
  - full/core pending plan parity;
  - existing-plan pending full/core parity;
  - route binding preservation;
  - pending signal clearing;
  - core stale pending fail-closed behavior;
  - replacement of stale `multipath_rebuild_*` explain lines.

## Council Result

- Architect: selected `pending rebuild full/core divergence` as the strongest
  measured next hotspot. Evidence: previous smoke showed
  `live_pending_rebuild_plan_path_ops_per_sec=5476` and
  `live_pending_rebuild_plan_core_ops_per_sec=12470`.
- Senior Rust reviewer: identified the `decision.explain.iter().cloned()` area
  as expensive but warned that removing it directly would require API/behavior
  care. The implemented solution changes storage so the clone is not needed in
  full plan paths while keeping explicit diagnostic rendering.
- Tester: found missing regression coverage around pending full/core parity,
  existing-plan pending APIs, core fail-closed behavior, and explain cleanup.
  Dedicated tests were added.
- Security/critic follow-up agents could not complete because the external
  sub-agent service returned quota errors. Their results are not used as
  evidence.

## Evidence

Commands passed on the final diff:

```text
cargo fmt --all -- --check
cargo check -q --workspace --all-targets
cargo clippy -q --workspace --all-targets -- -D warnings
cargo test -q -p chimera-mesh tests_multipath_schedule::rebuild_control -- --nocapture
cargo test -q -p chimera-mesh tests_multipath_schedule::rebuild_trigger -- --nocapture
cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only -- --nocapture
cargo run -q -p chimera-lab --bin chimera-lab -- metadata-perf-smoke --iterations 20000 --json
json="$(cargo run -q -p chimera-lab --bin chimera-lab -- metadata-perf-smoke --iterations 20000 --json)" && pending-field rg checks
just metadata-perf-smoke-selfcheck
just rust-no-hardcode-guard-selfcheck
just ship-structure-guard-selfcheck
just release-pack-schema-guard-selfcheck
git diff --check
```

The first full workspace test run had one unrelated `chimera-cli` failure in
`nodes_private_state_advertise_discovery_update_reaches_runtime_planner`.
That exact test passed on targeted rerun, and the final full workspace run
passed:

```text
cargo test -q -p chimera-cli nodes_private_state_advertise_discovery_update_reaches_runtime_planner -- --nocapture
cargo test -q --workspace --all-targets
```

Final `metadata-perf-smoke --iterations 20000 --json` snapshot captured during
this batch:

```text
live_pending_rebuild_plan_path_ops_per_sec=5268
live_pending_rebuild_plan_path_p95_ns=216828
live_pending_rebuild_plan_core_ops_per_sec=12122
live_pending_rebuild_plan_core_p95_ns=94299
network_state=not_modified
transit_payload_policy=opaque_sealed_payload_untouched
```

The smoke values are not claimed as a stable benchmark improvement. The proven
change is removal of permanent decision-owned explain strings and full-path
explain cloning.

## Interpretation

- This is a Lab/Metadata PASS for reducing pending rebuild explain/copy churn.
- Full diagnostic explain output remains available and tested.
- Core pending paths keep schedule/selection parity with full paths where
  tested.
- Local CHIMERA runtime was not started and local PC network settings were not
  changed.

## Limits

- This is not prod-ready, release-ready or real-runtime PASS.
- It does not close remote install/update/start/stop/reconnect/rollback.
- It does not prove transparent app behavior.
- It does not address the separate path planner setup/status explain allocation
  candidates identified for a later safe batch.
