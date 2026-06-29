# Workflow Attestation: Metadata Perf Connect Backoff Profile Prefix

## Scope

- Date: 2026-06-28
- Hot path: `connect_retry_profile`
- Target: `crates/chimera-mesh/src/runtime/connect_retry_profile.rs`
- Objective: keep metadata/control-path diagnostics cheap while preserving exact
  explain output and redaction.

## ANALYSIS

- `build_connect_backoff_profile()` is used by path-planner and auto-recovery
  selected-peer metrics.
- The field contains only static retry/backoff metadata plus `fanout`.
- It does not read peer ids, endpoints, ports, route bindings, DNS data, or
  transit payload.

## PLAN

- Replace the production backoff-profile formatter with a constant prefix plus
  a local decimal append helper for `fanout`.
- Keep the exact output contract:
  `initial=0ms;retry1=250ms;retry2=1000ms;jitter_step=50ms;fanout=N`.
- Add golden tests for `fanout` values `0`, `1`, `9`, `10`, and `100000`.

## TEAM_CRITIQUE

- Accepted: narrow micro-slice only in `connect_retry_profile.rs`.
- Accepted: preserve byte-for-byte output and do not touch routing, DNS, TUN,
  carrier, payload, or runtime network state.
- Rejected: claiming broad planner/runtime/datapath speedup from this slice.
- Notes: some sub-agent responses marked process-only `blocker` because those
  agents did not run commands themselves; no technical blocker was raised for
  the micro-slice.

## IMPLEMENTATION

- `build_connect_backoff_profile()` now appends
  `BACKOFF_PROFILE_PREFIX` and decimal `fanout` directly.
- `std::fmt::Write` is now test-only in this module.
- Existing retry-priority and retry-plan test helpers remain test-only.
- Added exact-output golden coverage for several fanout values.

## TEAM_CHECK

- Security boundary preserved: no payload, endpoint, port, peer id, destination,
  route-binding, or secret is added to the diagnostic field.
- Explain contract preserved by unit and CLI regression tests.
- No local CHIMERA runtime was started.
- No DNS, route, firewall, VPN, TUN, or proxy state was changed.

## FIX

- Not needed for the final micro-slice.
- A previous broader helper-heavy refactor of
  `path_planner_selection_metrics_peer.rs` was tested, showed worse
  `path_planner_candidate_snapshot` results, and was removed from the active
  implementation path.

## RECHECK

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
- `git diff --check -- crates/chimera-mesh/src/runtime/connect_retry_profile.rs crates/chimera-mesh/src/runtime/path_planner_selection_metrics_peer.rs`

Not used as evidence:

- `cargo test -q -p chimera-cli selected_peer_connect -- --nocapture`
  matched zero tests.

## FINAL_AUDIT

- Architecture: no blocker for the narrow metadata formatter slice.
- Security: no new leakage surface identified.
- Testing: exact string and redaction-adjacent tests pass.
- Performance: `metadata-perf-smoke` passed with `network_state=not_modified`,
  but it does not prove a planner speedup for this slice.

## REPORT

- Narrow formatter work is implemented and checked.
- Broad WEAVE datapath/runtime performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
