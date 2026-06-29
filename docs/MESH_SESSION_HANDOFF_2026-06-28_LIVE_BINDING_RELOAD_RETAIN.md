# CHIMERA Mesh Session Handoff: Live Binding Reload Retain

## Saved At

- Timestamp: 2026-06-28

## Active Objective

- Keep speeding hot metadata/control paths that help nodes find peers, choose
  paths, rebuild live lane/binding/route state, publish state, and avoid wasted
  CPU/RAM.
- Keep sealed transit payload opaque and untouched.

## What Was Done

- Narrow optimized `reconcile_live_transit_lane_workers()` in
  `crates/chimera-carrier/src/peer_egress/live_bindings.rs`.
- Replaced the temporary stale-binding `Vec` with direct stale worker eviction
  via `BTreeMap::retain()`.
- Kept unchanged worker retention, changed binding restart, dispatcher clear,
  cancel flag setting, and fail-closed reload behavior.
- Added a duplicate desired binding regression test to preserve the existing
  "last registration wins" behavior produced by the desired `BTreeMap`.
- No payload/datapath, TCP connect, secure peer stream, TUN, DNS, route,
  firewall, proxy, or runtime start/stop behavior was changed.

## Validation

PASS:

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

## Current Metadata Snapshot

- Targeted baseline before this slice:
  - `live_binding_reload_index_ops_per_sec=149115`
  - `live_binding_reload_index_p95_ns=6815`
  - `spawn_count=799200`
  - `network_state=not_modified`
- Targeted after this slice:
  - `live_binding_reload_index_ops_per_sec=164909`
  - `live_binding_reload_index_p95_ns=6646`
  - `spawn_count=799200`
  - `network_state=not_modified`
- Main metadata smoke after this slice:
  - `live_binding_reload_index_ops_per_sec=170329`
  - `live_binding_reload_index_p95_ns=6045`
  - `transit_payload_policy=opaque_sealed_payload_untouched`
  - `network_state=not_modified`

## Interpretation

- Narrow `live_binding_reload_index` metadata path improved in both targeted
  and main metadata smoke.
- Behavior and redaction coverage passed for the slice.
- The unchanged `spawn_count` shows the test still exercised the same worker
  churn workload.
- This is not a broad WEAVE datapath, SSH stand, or Real-World PASS.

## Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Project-wide dirty tree and unrelated guard debt remain outside this slice.

## Next Step

- Continue with measured metadata/control paths only.
- Prefer another direct hot-path metric with before/after evidence.
- Candidate next work: measure and tighten another lane/binding metadata path
  that is already represented in `metadata-perf-smoke`, or add an isolated
  microbench before touching broader planner loops.
