# CHIMERA Metadata Performance Attestation: DPS Payload Snapshot Fast Flags

## Scope

- Date: 2026-06-28
- Hot path: `live_dps_plan_path_from_payload`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- The latest handoff moved focus away from `status_explain` and back to the
  slower live DPS path.
- `plan_dps_adaptation.rs` checks the presence of a few mesh policy keys
  repeatedly on the same parsed payload snapshot.
- The DPS explain path also rebuilt the mesh-keys fingerprint string on demand.
- This slice keeps the same explain contract while removing repeated lookup and
  clone work from the live DPS metadata path.

## Change

- `crates/chimera-mesh/src/dps_payload_snapshot.rs`
  - stores the mesh-keys fingerprint once inside the parsed snapshot;
  - returns the stored fingerprint by borrowed `&str`;
  - records direct presence flags for the hot mesh policy keys used by DPS
    adaptation;
  - returns `route_binding_id` by copy instead of cloning.
- `crates/chimera-mesh/src/multipath_model.rs`
  - marks `MeshRouteBindingId` as `Copy`, matching its `NonZeroU64` payload.
- `crates/chimera-mesh/src/runtime/dps_payload_explain.rs`
  - keeps the borrowed DPS summary capture scoped without explicit `drop()`.
- `crates/chimera-mesh/src/runtime/multipath_schedule.rs`
  - reuses copied route-binding ids when building carrier lane bindings.
- `crates/chimera-mesh/src/runtime/multipath_rebuild_bridge.rs`
  - reuses copied route-binding ids during rebuild.
- `crates/chimera-mesh/src/runtime/multipath_aggregate/planner.rs`
  - reuses copied route-binding ids in shard planning.
- `crates/chimera-mesh/src/tests_multipath_schedule/flow_assignment.rs`
  - keeps the route-binding test path aligned with `Copy`.
- `crates/chimera-mesh/src/tests_multipath_schedule/flow_fail_closed.rs`
  - keeps the overflow fail-closed path aligned with `Copy`.
- `docs/PERFORMANCE.md`
  - should record this slice as the newest live DPS improvement.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all`
- `cargo check -q -p chimera-mesh`
- `cargo test -q -p chimera-mesh tests_dps_explain -- --nocapture`
- `cargo test -q -p chimera-mesh tests_dps_policy -- --nocapture`
- `cargo test -q -p chimera-mesh tests_multipath_schedule -- --nocapture`
- `cargo test -q -p chimera-mesh runtime_planning -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output after this slice:

```json
{"status":"ok","kind":"metadata_perf_smoke","hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint","lane_document_plan_snapshot_access","live_binding_reload_index","live_dps_plan_path_from_payload","status_explain"],"scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","iterations":100000,"active_bindings":16,"fast_sorted_ops_per_sec":1153124,"slow_sorted_fallback_ops_per_sec":154291,"fast_p95_ns":1046,"slow_sorted_fallback_p95_ns":7145,"fast_vs_fallback_speedup_pct":647.37,"path_planner_iterations":10000,"path_planner_peer_count":64,"path_planner_candidate_snapshot_ops_per_sec":6924,"path_planner_candidate_snapshot_p95_ns":171533,"discovery_rebuild_iterations":10000,"discovery_rebuild_peer_count":64,"discovery_rebuild_fingerprint_ops_per_sec":3710,"discovery_rebuild_fingerprint_p95_ns":280787,"lane_document_plan_snapshot_iterations":10000,"lane_document_plan_snapshot_borrowed_ops_per_sec":65861401,"lane_document_plan_snapshot_borrowed_p95_ns":14,"lane_document_plan_snapshot_owned_ops_per_sec":126900,"lane_document_plan_snapshot_owned_p95_ns":9137,"live_dps_plan_path_from_payload_iterations":10000,"live_dps_plan_path_from_payload_peer_count":64,"live_dps_plan_path_from_payload_ops_per_sec":4390,"live_dps_plan_path_from_payload_p95_ns":243589,"status_explain_iterations":10000,"status_explain_peer_count":64,"status_explain_ops_per_sec":15446,"status_explain_p95_ns":76070,"live_binding_reload_index_iterations":100000,"live_binding_reload_index_spawn_count":799200,"live_binding_reload_index_ops_per_sec":142820,"live_binding_reload_index_p95_ns":7551,"network_state":"not_modified"}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- `live_dps_plan_path_from_payload` is improved, but it is still slower than
  `status_explain` and `path_planner_candidate_snapshot`.
- The next live DPS gain likely sits in explain-key removal or standby-shadow
  rescans rather than in route-binding metadata.

## Rollback

- Remove the cached fingerprint and fast presence flags from
  `dps_payload_snapshot.rs`.
- Return `MeshRouteBindingId` to `Clone`-only semantics if the copy semantics
  turn out to be misleading for later work.
- Restore the old borrowed-summary scope in `dps_payload_explain.rs`.
- Remove the `Copy`-based call-site cleanup in the multipath helpers and tests.
