# CHIMERA Mesh Session Handoff: Metadata Performance

## Saved At

- Timestamp: 2026-06-26

## Active Objective

- Speed up service metadata that helps nodes find each other, choose paths,
  reconfigure, publish state, and keep lane/binding/route metadata efficient.
- Keep sealed transit payload opaque and untouched.

## What Was Done

- Held the required role council for the first performance slice.
- Selected `multipath_flow_lane_selection` as the first scoped hot path.
- Optimized `plan_multipath_flow` so the normal ordered active lane binding
  path no longer allocates and sorts a `Vec` per flow.
- Preserved the old sorted behavior as a slow fallback for unsorted active
  bindings.
- Added parity/fail-closed tests for unsorted bindings.
- Added `chimera-lab metadata-perf-smoke` and `just metadata-perf-smoke` for a
  targeted metadata benchmark.
- Recorded evidence in:
  - `docs/WORKFLOW_ATTESTATION_METADATA_PERF_MULTIPATH_FLOW_2026-06-26.md`
- Held the required role council for the second performance slice.
- Optimized path planner candidates with per-call `CandidateSlot<'a>` snapshots:
  accepted peers are no longer cloned during filtering/recovery/selection,
  normalized region keys are reused, and selected peers are materialized only at
  finalize.
- Preserved deterministic selection, explain output, fail-closed behavior and
  opaque sealed transit payload boundaries.
- Recorded evidence in:
  - `docs/WORKFLOW_ATTESTATION_METADATA_PERF_PATH_PLANNER_CANDIDATES_2026-06-26.md`
- Optimized discovery/rebuild fingerprinting so runtime reuses a single
  normalized region distribution snapshot inside the rebuild trigger path
  instead of rebuilding it twice per trigger.
- Added discovery/rebuild fingerprint smoke coverage in `chimera-lab` and
  regression tests for empty discovery batches and repeated same-value peer
  performance updates.
- Recorded evidence in:
  - `docs/WORKFLOW_ATTESTATION_METADATA_PERF_REBUILD_FINGERPRINT_2026-06-26.md`
- Optimized lane document plan snapshot access so hot carrier paths borrow the
  plan instead of cloning it.
- Added borrowed-access regressions plus a metadata smoke comparison of
  borrowed versus owned plan snapshot access.
- Recorded evidence in:
  - `docs/WORKFLOW_ATTESTATION_METADATA_PERF_LANE_DOCUMENT_PLAN_SNAPSHOT_ACCESS_2026-06-26.md`
- Optimized live binding reload so identical live documents and repeated reload
  errors short-circuit before snapshot replace and worker reconcile.
- Added reload-path regressions for identical no-churn reloads, changed reloads
  that replace stale workers, and fail-closed reload errors.
- Added an ignored carrier smoke and just target for the live binding reload
  no-op fast path.
- Recorded evidence in:
  - `docs/WORKFLOW_ATTESTATION_METADATA_PERF_LIVE_BINDING_RELOAD_NOOP_2026-06-26.md`

## Validation

- PASS: `cargo fmt --all -- --check`
- PASS: `cargo test -q -p chimera-mesh multipath_flow`
- PASS: `cargo test -q -p chimera-mesh tests_multipath_schedule`
- PASS: `cargo test -q -p chimera-carrier live_lane_selection`
- PASS: `cargo test -q -p chimera-carrier transit`
- PASS: `cargo test -q -p chimera-lab metadata_perf`
- PASS: `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- PASS: `cargo clippy -q -p chimera-lab --all-targets -- -D warnings`
- PASS: `just metadata-perf-smoke`
- PASS: `just metadata-perf-smoke-selfcheck`
- PASS: `cargo test -q -p chimera-carrier live_bindings`
- PASS: `cargo test -q -p chimera-carrier lane_document`
- PASS: `cargo clippy -q -p chimera-carrier --all-targets -- -D warnings`
- PASS: `just live-binding-reload-perf-smoke`
- PASS: `just live-binding-reload-perf-smoke-selfcheck`
- PASS: `cargo test -q -p chimera-lab metadata_perf`
- PASS: `cargo clippy -q -p chimera-lab --all-targets -- -D warnings`
- PASS: `just metadata-perf-smoke`
- PASS: `just metadata-perf-smoke-selfcheck`
- PASS: `just path-planner-candidate-snapshot-perf-smoke`
- PASS: `just path-planner-candidate-snapshot-perf-smoke-selfcheck`
- PASS: `just perf-smoke`
- PASS: `just benchmark-regression-selfcheck`
- PASS: `just benchmark-regression-check`
- PASS: `cargo test -q -p chimera-mesh path_planner_candidate_snapshot`
- PASS: `cargo test -q -p chimera-mesh tests_selection_behavior`
- PASS: `cargo test -q -p chimera-mesh tests_selection_policy`
- PASS: `cargo test -q -p chimera-mesh tests_auto_profile`
- PASS: `cargo test -q -p chimera-mesh tests_dps_runtime_flow`
- PASS: `cargo test -q -p chimera-mesh tests_failover_health`
- PASS: `cargo test -q -p chimera-mesh peer_performance`
- PASS: `cargo test -q -p chimera-cli tests_connect_probe_flow`
- PASS: `cargo test -q -p chimera-lab metadata_perf`
- PASS: `cargo clippy -q -p chimera-lab --all-targets -- -D warnings`
- PASS: `just metadata-perf-smoke`
- PASS: `just metadata-perf-smoke-selfcheck`

## Metadata Perf Snapshot

- Hot paths:
  - `multipath_flow_lane_selection`
  - `path_planner_candidate_snapshot`
  - `discovery_rebuild_fingerprint`
  - `lane_document_plan_snapshot_access`
- Scope: `hot_metadata_only`
- Network state: `not_modified`
- Iterations: 100000
- Active bindings: 16
- Fast sorted path: 1121551 ops/sec, p95 1235 ns
- Slow sorted fallback: 158176 ops/sec, p95 6676 ns
- Fast-vs-fallback speedup: 609.05%
- Path planner iterations: 10000
- Path planner peers: 64
- Path planner candidate snapshot: 7128 ops/sec, p95 146506 ns
- Discovery rebuild iterations: 10000
- Discovery rebuild peers: 64
- Discovery rebuild fingerprint: 3796 ops/sec, p95 265460 ns
- Lane document plan snapshot iterations: 10000
- Lane document plan snapshot borrowed: 66612932 ops/sec, p95 14 ns
- Lane document plan snapshot owned: 131597 ops/sec, p95 7663 ns

## Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Lane document plan snapshot access is now covered.
- Live binding reload no-op fast path is now covered.
- A further local binding-index cache may still be profiled later if changed
  reloads prove hot under real load.

## Next Step

- Continue with the next metadata hot path: live binding reload/index
  follow-up only if profiling says changed reloads still dominate; otherwise
  move to the next hot metadata path while preserving deterministic discovery
  state, explain output, fail-closed behavior and opaque sealed transit payload
  boundaries.

## Follow-Up Update

- Live binding reload/index work is now Lab/Metadata PASS.
- Evidence is recorded in:
  - `docs/WORKFLOW_ATTESTATION_METADATA_PERF_LIVE_BINDING_RELOAD_INDEX_2026-06-26.md`
- The main `chimera-lab metadata-perf-smoke` bundle now includes
  `live_binding_reload_index`.
- Path-planner selection metrics string assembly is now Lab/Metadata PASS.
- Evidence is recorded in:
  - `docs/WORKFLOW_ATTESTATION_METADATA_PERF_PATH_PLANNER_SELECTION_METRICS_2026-06-26.md`
- Remaining non-real-world limitation:
  - SSH runtime/datapath/load behavior is still unverified for this slice.

## Follow-Up Update

- Path planner candidate snapshot work is now Lab/Metadata PASS.
- Evidence is recorded in:
  - `docs/WORKFLOW_ATTESTATION_METADATA_PERF_PATH_PLANNER_CANDIDATES_2026-06-26.md`
- Discovery/rebuild fingerprint work is now Lab/Metadata PASS.
- Evidence is recorded in:
  - `docs/WORKFLOW_ATTESTATION_METADATA_PERF_REBUILD_FINGERPRINT_2026-06-26.md`
- Lane document plan snapshot access work is now Lab/Metadata PASS.
- Evidence is recorded in:
  - `docs/WORKFLOW_ATTESTATION_METADATA_PERF_LANE_DOCUMENT_PLAN_SNAPSHOT_ACCESS_2026-06-26.md`
- Live binding reload no-op fast path is now Lab/Metadata PASS.
- Evidence is recorded in:
  - `docs/WORKFLOW_ATTESTATION_METADATA_PERF_LIVE_BINDING_RELOAD_NOOP_2026-06-26.md`
- Remaining non-real-world limitation:
  - SSH runtime/datapath/load behavior remains unverified for this slice.
