# CHIMERA Metadata Performance Attestation: Live Binding Reload Retain

## Scope

- Date: 2026-06-28
- Hot path: `live_binding_reload_index`
- Scope boundary: hot metadata/control path only
- Status: Lab/Metadata PASS for this narrow slice
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Council Result

- Architect: keep the reload change local to `live_bindings.rs`; preserve
  deterministic worker lifecycle and do not change carrier/datapath behavior.
- Senior Rust: keep the `BTreeMap` worker model, remove the temporary stale
  binding `Vec`, and evict stale workers directly through `retain`.
- Tester: require before/after live-binding measurements, unchanged/changed
  reload correctness, duplicate desired binding behavior, and fail-closed
  reload error coverage.
- Security: accept only metadata-only worker bookkeeping changes; do not touch
  sealed transit payload, endpoint logging, secure peer streams, or dispatcher
  payload handling.
- DevOps: local checks are limited to build/unit/perf-smoke commands that report
  `network_state:"not_modified"`; runtime/start/stop/SSH stand checks are out
  of scope for this slice.
- Critic: reject any broad speedup claim unless the direct
  `live_binding_reload_index` before/after metric improves on the same workload.

## Change

- `crates/chimera-carrier/src/peer_egress/live_bindings.rs`
  - `reconcile_live_transit_lane_workers()` now evicts stale workers with
    `BTreeMap::retain()` instead of first collecting a temporary
    `stale_bindings` vector and then removing by key.
  - Stale workers still get `cancel=true`, their dispatcher binding is cleared,
    and unchanged workers remain in place.
  - Desired binding order and "last registration wins" behavior remain backed
    by the existing desired `BTreeMap`.
  - Added a regression test for duplicate desired bindings so the old
    `BTreeMap` behavior remains explicit.

## Evidence

Baseline before this slice:

```json
{"status":"ok","kind":"live_binding_reload_index_perf_smoke","iterations":100000,"spawn_count":799200,"ops_per_sec":149115,"p95_ns":6815,"network_state":"not_modified"}
```

Baseline `metadata-perf-smoke` before this slice:

```json
{"live_binding_reload_index_iterations":100000,"live_binding_reload_index_spawn_count":799200,"live_binding_reload_index_ops_per_sec":148542,"live_binding_reload_index_p95_ns":6753,"network_state":"not_modified"}
```

Commands passed after the change:

- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `cargo check -q -p chimera-carrier`
- `cargo test -q -p chimera-carrier live_bindings -- --nocapture`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `just live-binding-reload-index-perf-smoke-selfcheck`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`
- `cargo clippy -q -p chimera-carrier --all-targets -- -D warnings`
- `git diff --check -- crates/chimera-carrier/src/peer_egress/live_bindings.rs`

Targeted after run:

```json
{"status":"ok","kind":"live_binding_reload_index_perf_smoke","iterations":100000,"spawn_count":799200,"ops_per_sec":164909,"p95_ns":6646,"network_state":"not_modified"}
```

Main metadata smoke after run:

```json
{"status":"ok","kind":"metadata_perf_smoke","hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint","lane_document_plan_snapshot_access","live_binding_reload_index","live_dps_plan_path_from_payload","status_explain"],"scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","iterations":100000,"active_bindings":16,"fast_sorted_ops_per_sec":1163558,"slow_sorted_fallback_ops_per_sec":152016,"fast_p95_ns":992,"slow_sorted_fallback_p95_ns":7124,"fast_vs_fallback_speedup_pct":665.42,"path_planner_iterations":10000,"path_planner_peer_count":64,"path_planner_candidate_snapshot_ops_per_sec":7016,"path_planner_candidate_snapshot_p95_ns":152802,"discovery_rebuild_iterations":10000,"discovery_rebuild_peer_count":64,"discovery_rebuild_fingerprint_ops_per_sec":3632,"discovery_rebuild_fingerprint_p95_ns":296169,"lane_document_plan_snapshot_iterations":10000,"lane_document_plan_snapshot_borrowed_ops_per_sec":65283953,"lane_document_plan_snapshot_borrowed_p95_ns":14,"lane_document_plan_snapshot_owned_ops_per_sec":130450,"lane_document_plan_snapshot_owned_p95_ns":7962,"live_dps_plan_path_from_payload_iterations":10000,"live_dps_plan_path_from_payload_peer_count":64,"live_dps_plan_path_from_payload_ops_per_sec":4496,"live_dps_plan_path_from_payload_p95_ns":241114,"status_explain_iterations":10000,"status_explain_peer_count":64,"status_explain_ops_per_sec":16084,"status_explain_p95_ns":66866,"live_binding_reload_index_iterations":100000,"live_binding_reload_index_spawn_count":799200,"live_binding_reload_index_ops_per_sec":170329,"live_binding_reload_index_p95_ns":6045,"network_state":"not_modified"}
```

## Interpretation

- Direct targeted smoke improved from `149115` to `164909` ops/sec, with p95
  moving from `6815` ns to `6646` ns on the same workload.
- The main metadata smoke also improved the same field from `148542` to
  `170329` ops/sec, with p95 moving from `6753` ns to `6045` ns.
- `spawn_count` stayed `799200`, so the worker lifecycle workload did not get
  silently reduced.
- The result proves a narrow metadata/control-path improvement for
  `live_binding_reload_index`; it does not prove broad WEAVE datapath speedup.

## What Is Not Closed

- Real-world runtime/load/datapath checks are not covered.
- SSH stand install/update/start/stop checks are not covered.
- Broad planner and end-to-end VPN performance are not closed by this slice.

## Rollback

- Restore stale worker eviction to the old two-step path:
  collect `stale_bindings: Vec<_>` and remove each binding afterward.
- Remove the duplicate desired binding regression test added for this slice.
