# CEF Phase-1 Gates

## Purpose

Define closure criteria for the first Full CEF execution phase, separate from
MVP/Lab gates.

## Phase-1 Blocks

1. `dht_discovery`
2. `distributed_policy_store`
3. `mesh_runtime_skeleton`
4. `cooperative_relay_model`
5. `emergency_oob_carriers_skeleton`
6. `roaming_cache_skeleton`
7. `reputation_complaint_skeleton`

## Gate Contract (per block)

A block may be marked `implemented=true` only if all conditions are true:

1. Code evidence:
   - dedicated crate/module path exists in `crates/`.
2. Interface evidence:
   - explicit public API for the block exists (not comments-only).
3. Test evidence:
   - at least one unit/integration test for core parser/state path.
4. Artifact evidence:
   - block status appears in `docs/CEF_TRACK_REPORT.json`.
5. Non-regression:
   - `just truth-contract-check` remains PASS.

## Current Snapshot (2026-05-18)

- `dht_discovery`: implemented, runtime wired
- `distributed_policy_store`: implemented, runtime wired
- `mesh_runtime_skeleton`: implemented, runtime wired
- `cooperative_relay_model`: implemented, runtime wired
- `emergency_oob_carriers_skeleton`: implemented, runtime wired
- `roaming_cache_skeleton`: implemented, runtime wired
- `reputation_complaint_skeleton`: implemented, runtime wired
- `phase1_closed` (`docs/CEF_TRACK_REPORT.json`): `true`

## Exit Criteria for Phase-1

Phase-1 is closed only when all listed blocks satisfy the gate contract and:

1. every block has `runtime_wired=true` in `docs/CEF_TRACK_REPORT.json`;
2. `phase1_closed=true` in `docs/CEF_TRACK_REPORT.json`;
3. `just truth-contract-check` remains PASS.
