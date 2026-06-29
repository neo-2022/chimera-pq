# CHIMERA Metadata Performance Attestation: Live Binding Reload / Index Path

## Scope

- Date: 2026-06-26
- Hot path: `live_binding_reload_index`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Council Result

- Architect: keep the reload fast path local to `live_bindings.rs`; do not move
  binding caches into `TransitLaneDocument`.
- Senior Rust: borrow the live document during snapshot replacement and borrow
  the desired binding index during reconcile; clone registrations only when a
  worker actually needs a fresh owned copy.
- Tester: require both no-op and changed-reload smokes, plus reload-error
  fail-closed regression.
- Security: accept only if the change remains metadata-only, keeps payload
  opaque, and preserves fail-closed snapshot replacement.
- DevOps: add a changed-reload smoke target so the reload/index path is
  measurable separately from the no-op reload.
- Critic: reject any optimization that weakens worker eviction, stale binding
  replacement, or snapshot consistency.

## Change

- `crates/chimera-carrier/src/peer_egress/live_bindings.rs`
  - `replace_live_transit_lane_snapshot()` now stores `Arc<TransitLaneDocument>`
    directly instead of cloning the entire document into a fresh Arc;
  - `apply_live_transit_lane_reload()` wraps the changed document once and
    reuses that Arc for snapshot replacement and reconcile;
  - `reconcile_live_transit_lane_workers()` now builds a borrowed desired
    binding index and clones registrations only for actual worker insert/update
    cases;
  - adds an ignored changed-reload performance smoke that alternates live
    desired bindings and exercises reconcile churn.
- `crates/chimera-lab/src/metadata_perf.rs`
  - main `metadata-perf-smoke` now includes `live_binding_reload_index` and
    reports its iterations, spawn count, ops/sec and p95.
- `justfile`
  - adds `live-binding-reload-index-perf-smoke`;
  - adds `live-binding-reload-index-perf-smoke-selfcheck`.
- `docs/PERFORMANCE.md`
  - records the new `live_binding_reload_index` slice as an applied metadata
    optimization and notes that the main metadata smoke includes it.

## Evidence

Commands passed locally without changing network state:

- `cargo test -q -p chimera-carrier live_bindings`
- `cargo clippy -q -p chimera-carrier --all-targets -- -D warnings`
- `cargo test -q -p chimera-lab metadata_perf`
- `just metadata-perf-smoke`
- `cargo test -q -p chimera-carrier peer_egress::live_bindings::tests::reload_noop_fast_path_perf_smoke -- --ignored --exact --nocapture --test-threads=1`
- `cargo test -q -p chimera-carrier peer_egress::live_bindings::tests::reload_changed_document_reconcile_perf_smoke -- --ignored --exact --nocapture --test-threads=1`

No-op reload smoke:

```json
{"status":"ok","kind":"live_binding_reload_perf_smoke","iterations":100000,"spawn_count":0,"ops_per_sec":5484655,"p95_ns":282,"network_state":"not_modified"}
```

Changed reload/index smoke:

```json
{"status":"ok","kind":"live_binding_reload_index_perf_smoke","iterations":100000,"spawn_count":399604,"ops_per_sec":201480,"p95_ns":5601,"network_state":"not_modified"}
```

Main metadata smoke now also includes `live_binding_reload_index` in its hot
paths and JSON output.

`just metadata-perf-smoke` output:

```json
{"status":"ok","kind":"metadata_perf_smoke","hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint","lane_document_plan_snapshot_access","live_binding_reload_index"],"scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","iterations":100000,"active_bindings":16,"fast_sorted_ops_per_sec":1076482,"slow_sorted_fallback_ops_per_sec":158020,"fast_p95_ns":1505,"slow_sorted_fallback_p95_ns":7607,"fast_vs_fallback_speedup_pct":581.23,"path_planner_iterations":10000,"path_planner_peer_count":64,"path_planner_candidate_snapshot_ops_per_sec":6841,"path_planner_candidate_snapshot_p95_ns":183561,"discovery_rebuild_iterations":10000,"discovery_rebuild_peer_count":64,"discovery_rebuild_fingerprint_ops_per_sec":3674,"discovery_rebuild_fingerprint_p95_ns":306302,"lane_document_plan_snapshot_iterations":10000,"lane_document_plan_snapshot_borrowed_ops_per_sec":66184841,"lane_document_plan_snapshot_borrowed_p95_ns":14,"lane_document_plan_snapshot_owned_ops_per_sec":132011,"lane_document_plan_snapshot_owned_p95_ns":7802,"live_binding_reload_index_iterations":100000,"live_binding_reload_index_spawn_count":799200,"live_binding_reload_index_ops_per_sec":147599,"live_binding_reload_index_p95_ns":7386,"network_state":"not_modified"}
```

## What Is Not Closed

- This does not close broad WEAVE datapath performance.
- This does not prove the changed reload/index path is the last remaining hot
  metadata path.
- Real-world runtime/load/datapath checks remain SSH-stand work.

## Rollback

- Restore `replace_live_transit_lane_snapshot()` to owned document cloning.
- Restore `reconcile_live_transit_lane_workers()` to a fully owned desired map.
- Remove the changed-reload perf smoke and the two `justfile` targets.
