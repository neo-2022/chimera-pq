# CHIMERA Mesh Session Handoff: Connect Backoff Profile Prefix

## Saved At

- Timestamp: 2026-06-28

## Active Objective

- Keep speeding hot metadata/status paths that help nodes publish state,
  explain route and peer decisions, and avoid wasted CPU/RAM.
- Keep sealed transit payload opaque and untouched.

## What Was Done

- Tightened `build_connect_backoff_profile()` in
  `crates/chimera-mesh/src/runtime/connect_retry_profile.rs`.
- The production formatter now appends a constant backoff-profile prefix and a
  direct decimal `fanout`, instead of formatting the whole string.
- Added golden tests for `fanout=0`, `1`, `9`, `10`, and `100000`.
- Kept the exact public diagnostic output:
  `initial=0ms;retry1=250ms;retry2=1000ms;jitter_step=50ms;fanout=N`.
- Kept the field metadata-only: no peer id, endpoint, port list, destination,
  route-binding value, secret, or payload was added.

## Validation

PASS:

- `cargo fmt --all -- --check`
- `cargo check -q -p chimera-mesh`
- `cargo check -q -p chimera-cli`
- `cargo test -q -p chimera-mesh connect_retry_profile --lib -- --nocapture`
- `cargo test -q -p chimera-mesh runtime_planning -- --nocapture`
- `cargo test -q -p chimera-mesh tests_selection_behavior -- --nocapture`
- `cargo test -q -p chimera-mesh tests_discovery_merge -- --nocapture`
- `cargo test -q -p chimera-mesh runtime_status_explain -- --nocapture`
- `cargo test -q -p chimera-mesh tests_dps_explain -- --nocapture`
- `cargo test -q -p chimera-cli mesh_route_explain_json_success_snapshot_core -- --nocapture`
- `cargo test -q -p chimera-cli tests_json_success_presence -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo clippy -q -p chimera-cli --all-targets -- -D warnings`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

Not counted:

- `cargo test -q -p chimera-cli selected_peer_connect -- --nocapture`
  matched zero tests and is not evidence.

## Current Metadata Snapshot

- Before this turn baseline:
  - `path_planner_candidate_snapshot_ops_per_sec=7469`
  - `path_planner_candidate_snapshot_p95_ns=142462`
  - `live_dps_plan_path_from_payload_ops_per_sec=4689`
  - `live_dps_plan_path_from_payload_p95_ns=226801`
  - `status_explain_ops_per_sec=16895`
  - `status_explain_p95_ns=60052`
  - `network_state=not_modified`
- Final `just metadata-perf-smoke` after the micro-slice:
  - `path_planner_candidate_snapshot_ops_per_sec=7211`
  - `path_planner_candidate_snapshot_p95_ns=149161`
  - `live_dps_plan_path_from_payload_ops_per_sec=4713`
  - `live_dps_plan_path_from_payload_p95_ns=213738`
  - `status_explain_ops_per_sec=16832`
  - `status_explain_p95_ns=59817`
  - `network_state=not_modified`

## Interpretation

- Behavior, exact diagnostics, and redaction are verified for this micro-slice.
- A broad planner speedup is not proven by this run.
- The earlier broader helper-heavy refactor of
  `path_planner_selection_metrics_peer.rs` showed worse path-planner smoke
  numbers and was removed from the active implementation path.

## Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Project-wide guard debt remains outside this slice.

## Next Step

- Continue with measured active metadata paths only.
- Prefer future slices that have a direct benchmark or an isolated microbench
  before changing broader planner loops.
- Do not claim speedup from formatter-only changes unless before/after evidence
  proves it on the same workload.
