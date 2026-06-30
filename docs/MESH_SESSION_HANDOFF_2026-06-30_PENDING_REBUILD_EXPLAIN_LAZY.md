# CHIMERA Mesh Session Handoff: Pending Rebuild Explain Lazy

## Saved At

- Timestamp: 2026-06-30

## Active Objective

- Speed metadata/control paths that help mesh nodes find peers, choose paths,
  rebuild lane/binding/route state, publish state and avoid wasted CPU/RAM.
- Keep transit payload opaque/sealed and untouched.

## What Was Done

- Targeted the pending multipath rebuild control path.
- Removed permanent `Vec<String>` explain storage from
  `MeshMultipathRebuildDecision`.
- Removed per-decision owned copies of static rebuild policy/privacy labels.
- Added lazy explain rendering through `append_explain_to` and `explain()`.
- Changed full rebuild bridge paths to append explain directly to
  `MeshPathPlan.explain` instead of cloning a decision-owned vector.
- Added pending rebuild regression tests for full/core parity, existing-plan
  pending APIs, route binding preservation, pending signal clearing, core
  stale fail-closed behavior and explain cleanup.

## Validation

PASS on the final diff:

- `cargo fmt --all -- --check`
- `cargo check -q --workspace --all-targets`
- `cargo clippy -q --workspace --all-targets -- -D warnings`
- `cargo test -q -p chimera-mesh tests_multipath_schedule::rebuild_control -- --nocapture`
- `cargo test -q -p chimera-mesh tests_multipath_schedule::rebuild_trigger -- --nocapture`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only -- --nocapture`
- `cargo test -q -p chimera-cli nodes_private_state_advertise_discovery_update_reaches_runtime_planner -- --nocapture`
- `cargo test -q --workspace --all-targets`
- `cargo run -q -p chimera-lab --bin chimera-lab -- metadata-perf-smoke --iterations 20000 --json`
- explicit JSON field checks for pending rebuild metrics, `network_state` and
  transit payload policy
- `just metadata-perf-smoke-selfcheck`
- `just rust-no-hardcode-guard-selfcheck`
- `just ship-structure-guard-selfcheck`
- `just release-pack-schema-guard-selfcheck`
- `git diff --check`

## Current Metadata Snapshot

- `live_pending_rebuild_plan_path_ops_per_sec=5268`
- `live_pending_rebuild_plan_path_p95_ns=216828`
- `live_pending_rebuild_plan_core_ops_per_sec=12122`
- `live_pending_rebuild_plan_core_p95_ns=94299`
- `network_state=not_modified`
- `transit_payload_policy=opaque_sealed_payload_untouched`

## Interpretation

- This is Lab/Metadata PASS for pending rebuild explain/copy churn reduction.
- It is not a prod-ready, real-runtime, datapath or transparent app PASS.
- Local CHIMERA runtime was not started and local PC network state was not
  changed.
- Transit payload remained opaque/sealed and untouched.

## Next Step

- Best architectural next step remains the remote release/runtime gate:
  release artifact, install/update without `cargo`, start/stop/restart,
  reconnect/rebind, rollback and redacted diagnostics on the SSH stand.
- If one more local metadata batch is justified first, use the measured safe
  candidates from the senior Rust review: path planner setup allocations or
  status explain temporary strings. Do not continue random micro-optimizations.
