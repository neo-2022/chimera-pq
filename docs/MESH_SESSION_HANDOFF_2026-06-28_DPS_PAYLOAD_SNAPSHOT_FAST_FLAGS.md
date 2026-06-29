# CHIMERA Mesh Session Handoff: DPS Payload Snapshot Fast Flags

## Saved At

- Timestamp: 2026-06-28

## Active Objective

- Keep speeding the hot metadata/status paths that help nodes publish state,
  explain route and peer decisions, and avoid wasted CPU/RAM.
- Keep sealed transit payload opaque and untouched.

## What Was Done

- Cached the DPS mesh-key fingerprint inside the parsed payload snapshot
  instead of rebuilding it on demand.
- Added direct presence flags for the hot mesh policy keys used by DPS
  adaptation.
- Switched `MeshRouteBindingId` to `Copy` and removed clone churn from the live
  multipath/DPS metadata path.
- Kept the borrowed DPS summary capture scoped without explicit `drop()`.
- Preserved explain keys, redaction shape, and transit opaque/sealed handling.

## Validation

- PASS: `cargo fmt --all`
- PASS: `cargo check -q -p chimera-mesh`
- PASS: `cargo test -q -p chimera-mesh tests_dps_explain -- --nocapture`
- PASS: `cargo test -q -p chimera-mesh tests_dps_policy -- --nocapture`
- PASS: `cargo test -q -p chimera-mesh tests_multipath_schedule -- --nocapture`
- PASS: `cargo test -q -p chimera-mesh runtime_planning -- --nocapture`
- PASS: `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- PASS: `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- PASS: `just metadata-perf-smoke`
- PASS: `just metadata-perf-smoke-selfcheck`

## Current Snapshot

- `path_planner_candidate_snapshot_ops_per_sec=6924`
- `path_planner_candidate_snapshot_p95_ns=171533`
- `live_dps_plan_path_from_payload_ops_per_sec=4390`
- `live_dps_plan_path_from_payload_p95_ns=243589`
- `status_explain_ops_per_sec=15446`
- `status_explain_p95_ns=76070`

## Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- The live DPS path is improved but still remains one of the slower metadata
  paths.

## Next Step

- Profile and trim the remaining rescans around DPS explain cleanup and standby
  shadow adaptation on the same live DPS path before touching broader WEAVE
  datapath code.
