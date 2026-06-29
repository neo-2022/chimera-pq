# CHIMERA Metadata Performance Attestation: Multipath Flow Lane Selection

## Scope

- Date: 2026-06-26
- Hot path: `multipath_flow_lane_selection`
- Scope boundary: hot metadata only
- Transit payload policy: opaque sealed payload untouched

## Council Result

- Architect: prioritize per-flow lane selection because it is close to the
  route/lane hot path and can be isolated from payload handling.
- Senior Rust: remove per-flow `Vec<&MeshCarrierLaneBinding>` allocation and
  `sort_by_key` from the normal ordered path.
- Tester: add explicit parity/fail-closed tests and a targeted metadata
  benchmark because the generic `perf-smoke` does not measure this hot path.
- Security: accepted only if the fast path keeps fail-closed behavior and
  falls back to sorted selection for unsorted bindings.
- DevOps/perf: local code/perf gates are allowed because they do not change
  routes, DNS, firewall, VPN or runtime network state.
- Critic: do not claim broad performance closure; this is one measured slice,
  not full WEAVE datapath performance completion.

## Change

- `crates/chimera-mesh/src/runtime/multipath_flow.rs`
  - normal ordered active lane bindings now use a streaming scan;
  - unsorted active bindings use the previous sorted slow fallback;
  - fail-closed reasons are preserved:
    - `transit_payload_policy_not_opaque`
    - `route_binding_missing`
    - `active_binding_missing`
    - `route_binding_mismatch`
    - `duplicate_active_lane`
    - `active_binding_capacity_missing`
    - `capacity_overflow`
    - `active_binding_capacity_over_budget`
    - `weighted_selection_no_match`
- `crates/chimera-lab/src/metadata_perf.rs`
  - adds `metadata-perf-smoke`;
  - compares fast sorted metadata selection with the slow sorted fallback;
  - emits redacted JSON only, without node IDs, endpoints, route IDs or flow
    material.
- `justfile`
  - adds `metadata-perf-smoke`;
  - adds `metadata-perf-smoke-selfcheck`.
- `docs/PERFORMANCE.md`
  - lists `just metadata-perf-smoke` as a performance gate.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh multipath_flow`
- `cargo test -q -p chimera-mesh tests_multipath_schedule`
- `cargo test -q -p chimera-carrier live_lane_selection`
- `cargo test -q -p chimera-carrier transit`
- `cargo test -q -p chimera-lab metadata_perf`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo clippy -q -p chimera-lab --all-targets -- -D warnings`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`
- `just perf-smoke`
- `just benchmark-regression-selfcheck`
- `just benchmark-regression-check`

`just metadata-perf-smoke` output:

```json
{"status":"ok","kind":"metadata_perf_smoke","hot_path":"multipath_flow_lane_selection","scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","iterations":100000,"active_bindings":16,"fast_sorted_ops_per_sec":1117097,"slow_sorted_fallback_ops_per_sec":152462,"fast_p95_ns":1254,"slow_sorted_fallback_p95_ns":8650,"fast_vs_fallback_speedup_pct":632.71,"network_state":"not_modified"}
```

`just benchmark-regression-check` refreshed:

- `docs/benchmark_latest.json`
- `docs/BENCHMARK_REGRESSION_GATE.json`

The gate reported:

```json
{"status":"ok","kind":"benchmark_regression_gate","message_en":"Benchmark regression gate passed.","message_ru":"Гейт регрессии производительности пройден.","attempt":1,"max_attempts":2,"max_regression_pct":20,"baseline_profile":"local","baseline_file":"docs/benchmark_baseline.json","output_file":"docs/benchmark_latest.json"}
```

## What Is Not Closed

- This is not a broad WEAVE datapath performance PASS.
- Real-world runtime/datapath/load behavior still requires the SSH stand.
- Path planner candidate snapshot optimization remains open.
- Discovery/rebuild fingerprint optimization remains open.
- Live binding reload/index optimization remains open.

## Rollback

- Revert the streaming scan in `multipath_flow.rs` to always use the sorted
  `active_carrier_bindings` vector path.
- Remove `metadata_perf.rs`, the `metadata-perf-smoke` CLI entry and the
  `justfile` targets if the new benchmark path itself causes trouble.
