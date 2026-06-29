# Workflow Attestation: DPS Payload Explain Tail Snapshot Scope

Status: partial_metadata_perf_slice_pass
Date: 2026-06-28

## Scope

- Hot path: `live_dps_plan_path_from_payload`
- Secondary affected path: DPS `status_explain_with_dps_payload`
- Scope boundary: hot metadata only
- Transit payload policy: opaque sealed payload untouched

This slice does not remove `DpsPayloadExplainSnapshot::capture()` completely.
The council rejected a broad typed-sidecar rewrite as too wide for this step.
The accepted change narrows the scan scope and removes one temporary summary
tail buffer while preserving explain output shape.

## Council Result

- Architect: do not perform a broad typed sidecar rewrite; only narrow the
  existing scan scope.
- Tester: add focused order/redaction tests for the DPS tail.
- Security: do not read, log, classify, or export raw DPS payload strings,
  transit bytes, destinations, endpoints, or route binding values.
- DevOps/guard: use only cargo/unit/lab metadata checks; do not touch local
  network, TUN, DNS, firewall, routes, or SSH stand.
- Critic: accept only as a small guarded metadata slice; do not treat it as
  broad WEAVE datapath performance work.
- Senior developer: did not return before shutdown, so no additional senior
  objection was available.

## Changes

- `crates/chimera-mesh/src/runtime/dps_payload_explain.rs`
  - Removes stale hint lines before summary capture.
  - Captures `DpsPayloadExplainSnapshot` from the cleaned pre-existing explain
    lines before appending the new DPS tail.
  - Builds DPS metadata, hints, decision summaries, and standby summaries into
    one pre-sized `dps_lines` buffer, then appends it to `explain`.
- `crates/chimera-mesh/src/runtime/dps_payload_explain_summary.rs`
  - Converted into a thin facade.
- `crates/chimera-mesh/src/runtime/dps_payload_explain_summary/`
  - Added `snapshot.rs`, `decision.rs`, and `standby.rs` to remove the
    overgrown summary monolith.
- `crates/chimera-mesh/src/tests_dps_explain/metadata_tail_contract.rs`
  - Adds order checks for the DPS metadata/hints/decision/standby tail.
  - Adds redaction checks for non-mesh payload notes, raw endpoints, and raw
    route-binding values.

## Validation

PASS:

- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `cargo check -q -p chimera-mesh`
- `cargo test -q -p chimera-mesh tests_dps_explain -- --nocapture`
- `cargo test -q -p chimera-mesh runtime_status_explain -- --nocapture`
- `cargo test -q -p chimera-mesh tests_standby_shadow -- --nocapture`
- `cargo test -q -p chimera-mesh runtime_planning -- --nocapture`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output after this slice:

```json
{"status":"ok","kind":"metadata_perf_smoke","hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint","lane_document_plan_snapshot_access","live_binding_reload_index","live_dps_plan_path_from_payload","status_explain"],"scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","iterations":100000,"active_bindings":16,"fast_sorted_ops_per_sec":1177374,"slow_sorted_fallback_ops_per_sec":152950,"fast_p95_ns":937,"slow_sorted_fallback_p95_ns":7156,"fast_vs_fallback_speedup_pct":669.78,"path_planner_iterations":10000,"path_planner_peer_count":64,"path_planner_candidate_snapshot_ops_per_sec":7055,"path_planner_candidate_snapshot_p95_ns":156284,"discovery_rebuild_iterations":10000,"discovery_rebuild_peer_count":64,"discovery_rebuild_fingerprint_ops_per_sec":3751,"discovery_rebuild_fingerprint_p95_ns":276749,"lane_document_plan_snapshot_iterations":10000,"lane_document_plan_snapshot_borrowed_ops_per_sec":66409000,"lane_document_plan_snapshot_borrowed_p95_ns":14,"lane_document_plan_snapshot_owned_ops_per_sec":133615,"lane_document_plan_snapshot_owned_p95_ns":7813,"live_dps_plan_path_from_payload_iterations":10000,"live_dps_plan_path_from_payload_peer_count":64,"live_dps_plan_path_from_payload_ops_per_sec":4656,"live_dps_plan_path_from_payload_p95_ns":223174,"status_explain_iterations":10000,"status_explain_peer_count":64,"status_explain_ops_per_sec":16848,"status_explain_p95_ns":59964,"live_binding_reload_index_iterations":100000,"live_binding_reload_index_spawn_count":799200,"live_binding_reload_index_ops_per_sec":150111,"live_binding_reload_index_p95_ns":6683,"network_state":"not_modified"}
```

Compared with the previous handoff snapshot:

- `live_dps_plan_path_from_payload_ops_per_sec`: `4610 -> 4656`
- `live_dps_plan_path_from_payload_p95_ns`: `231307 -> 223174`
- `status_explain_ops_per_sec`: `16392 -> 16848`
- `status_explain_p95_ns`: `63423 -> 59964`

## Guard Limitations

These project-wide guards were run but did not pass due existing unrelated
debt outside this slice:

- `just anti-monolith-guard`
  - Still fails on pre-existing oversized files including
    `crates/chimera-mesh/src/runtime.rs`,
    `crates/chimera-mesh/src/runtime/multipath_flow.rs`,
    `crates/chimera-carrier/src/peer_egress/options.rs`,
    `crates/chimera-carrier/src/peer_egress/live_bindings.rs`, and
    `crates/chimera-carrier/src/peer_egress/options_tests/mod.rs`.
  - The touched `dps_payload_explain_summary.rs` is no longer listed as a
    failure after the split.
- `just rust-no-hardcode-guard-selfcheck`
- `just rust-no-hardcode-guard`
  - Both still fail on pre-existing stand-specific markers in older docs such
    as `WORKFLOW_ATTESTATION_REAL_WORLD_RELEASE_UPDATE_V0_1_135_2026-06-26.md`
    and earlier metadata handoffs.

These failures mean the full repository guard state is not clean. They do not
contradict the local metadata slice validation above, but they remain blockers
for any broad `done/pass` claim.

## Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- `DpsPayloadExplainSnapshot::capture()` still exists; only its scan scope was
  narrowed.
- Project-wide anti-monolith and hardcode guards remain blocked by older
  unrelated files/docs.

## Rollback

- Revert `dps_payload_explain.rs` to capture after appending hints.
- Remove `runtime/dps_payload_explain_summary/`.
- Restore the single-file `dps_payload_explain_summary.rs` implementation.
- Remove `tests_dps_explain/metadata_tail_contract.rs` and its module entry.
