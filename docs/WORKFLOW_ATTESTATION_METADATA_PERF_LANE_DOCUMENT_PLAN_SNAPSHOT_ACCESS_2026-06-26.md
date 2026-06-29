# CHIMERA Metadata Performance Attestation: Lane Document Plan Snapshot Access

## Scope

- Date: 2026-06-26
- Hot path: `lane_document_plan_snapshot_access`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Council Result

- Architect: add borrowed accessors to `TransitLaneDocument` and keep the
  existing owned API for compatibility.
- Senior Rust: remove the plan clone on hot read paths that only inspect or
  forward the snapshot.
- Tester: require borrowed-access regressions plus compile coverage on the hot
  ingress and transit paths.
- Security: accept only if the change preserves redaction and never inspects
  payload bytes.
- DevOps: measure the borrowed path against the owned clone path in the metadata
  smoke so the win is visible.
- Critic: reject any redesign that turns the plan snapshot into a cache layer or
  changes the document format.

## Change

- `crates/chimera-carrier/src/peer_egress/lane_document.rs`
  - adds `mesh_path_plan_ref()` and `require_mesh_path_plan_ref()`;
  - keeps `mesh_path_plan()` and `require_mesh_path_plan()` for compatibility.
- `crates/chimera-carrier/src/peer_egress/live_bindings/contract.rs`
  - switches contract validation to borrowed plan access.
- `crates/chimera-carrier/src/peer_egress/aggregate_peer_ingress.rs`
  - forwards planned aggregate transit using a borrowed plan reference.
- `crates/chimera-carrier/src/peer_egress/modes_local_ingress.rs`
  - local ingress lane selection now borrows the plan snapshot.
- `crates/chimera-carrier/src/peer_egress/transit_document.rs`
  - transit forwarding now uses borrowed plan access.
- `crates/chimera-carrier/src/peer_egress/transit_local.rs`
  - local transit relay now uses borrowed plan access.
- `crates/chimera-carrier/src/peer_egress/lane_document/tests.rs`
  - adds a regression for borrowed snapshot access.
- `crates/chimera-lab/src/metadata_perf.rs`
  - adds a `lane_document_plan_snapshot_access` hot path;
  - measures borrowed vs owned plan snapshot access.
- `justfile`
  - extends `metadata-perf-smoke` and its selfcheck for the new plan-snapshot
    metric fields.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-carrier lane_document`
- `cargo test -q -p chimera-carrier live_bindings`
- `cargo test -q -p chimera-carrier transit`
- `cargo test -q -p chimera-lab metadata_perf`
- `cargo clippy -q -p chimera-carrier --all-targets -- -D warnings`
- `cargo clippy -q -p chimera-lab --all-targets -- -D warnings`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output:

```json
{"status":"ok","kind":"metadata_perf_smoke","hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint","lane_document_plan_snapshot_access"],"scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","iterations":100000,"active_bindings":16,"fast_sorted_ops_per_sec":1178543,"slow_sorted_fallback_ops_per_sec":158709,"fast_p95_ns":1066,"slow_sorted_fallback_p95_ns":6808,"fast_vs_fallback_speedup_pct":642.58,"path_planner_iterations":10000,"path_planner_peer_count":64,"path_planner_candidate_snapshot_ops_per_sec":6867,"path_planner_candidate_snapshot_p95_ns":160483,"discovery_rebuild_iterations":10000,"discovery_rebuild_peer_count":64,"discovery_rebuild_fingerprint_ops_per_sec":3752,"discovery_rebuild_fingerprint_p95_ns":271623,"lane_document_plan_snapshot_iterations":10000,"lane_document_plan_snapshot_borrowed_ops_per_sec":66612932,"lane_document_plan_snapshot_borrowed_p95_ns":14,"lane_document_plan_snapshot_owned_ops_per_sec":131597,"lane_document_plan_snapshot_owned_p95_ns":7663,"network_state":"not_modified"}
```

## What Is Not Closed

- This does not close broad WEAVE datapath performance.
- This does not change the carrier document format or policy model.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Further cache work is not required for this slice unless a later profile shows
  a new hotspot.

## Rollback

- Remove the borrowed accessors from `TransitLaneDocument`.
- Restore the owned `MeshPathPlan` clone at the hot call sites.
- Remove the borrowed-access regression and the benchmark fields.
- Remove the new `justfile` smoke checks.
