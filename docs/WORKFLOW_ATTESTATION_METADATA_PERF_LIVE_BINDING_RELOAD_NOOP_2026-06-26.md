# CHIMERA Metadata Performance Attestation: Live Binding Reload No-Op Fast Path

## Scope

- Date: 2026-06-26
- Hot path: `live_binding_reload_noop_fast_path`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Council Result

- Architect: keep the optimization local to the reload loop; do not move
  binding caches into the canonical lane document.
- Senior Rust: use a no-op snapshot equality fast-path before snapshot replace
  and worker reconcile.
- Tester: require regressions for identical reload no churn, changed reload
  replacement, and reload error fail-closed.
- Security: accept only if the change stays metadata-only, does not inspect
  payload bytes, and keeps redacted diagnostics.
- DevOps: add a dedicated smoke so the reload path is measurable separately
  from the other metadata hot paths.
- Critic: reject any cache that leaks payload or makes the document itself a
  second data model.

## Change

- `crates/chimera-carrier/src/peer_egress/live_bindings.rs`
  - adds `apply_live_transit_lane_reload()` to own one reload iteration;
  - adds `live_transit_lane_snapshot_matches_document()` and
    `live_transit_lane_snapshot_matches_error()`;
  - skips `replace_live_transit_lane_snapshot()` and worker reconcile when the
    new document or error matches the current snapshot;
  - adds regression tests for:
    - identical reload no churn;
    - changed reload replaces stale workers and spawns new bindings;
    - reload error clears workers and leaves a fail-closed error snapshot;
  - adds an ignored no-op reload performance smoke test that emits redacted
    JSON.
- `justfile`
  - adds `live-binding-reload-perf-smoke`;
  - adds `live-binding-reload-perf-smoke-selfcheck`.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-carrier live_bindings`
- `cargo test -q -p chimera-carrier lane_document`
- `cargo clippy -q -p chimera-carrier --all-targets -- -D warnings`
- `just live-binding-reload-perf-smoke`
- `just live-binding-reload-perf-smoke-selfcheck`

`cargo test -q -p chimera-carrier peer_egress::live_bindings::tests::reload_noop_fast_path_perf_smoke -- --ignored --exact --nocapture --test-threads=1`
output:

```json
{"status":"ok","kind":"live_binding_reload_perf_smoke","iterations":10000,"spawn_count":0,"ops_per_sec":5984620,"p95_ns":166,"network_state":"not_modified"}
```

## What Is Not Closed

- This does not close broad WEAVE datapath performance.
- This does not yet prove the changed-reload path is the only remaining hot
  spot under real load.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- A further local binding-index cache may still be worth profiling later, but it
  is not required for this no-op fast-path slice.

## Rollback

- Remove `apply_live_transit_lane_reload()` and restore the direct
  replace+reconcile reload loop.
- Remove the three reload regressions and the ignored smoke test.
- Remove the two `justfile` live-binding smoke targets.
