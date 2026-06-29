# CHIMERA Metadata Performance Attestation: Status Explain

## Scope

- Date: 2026-06-27
- Hot path: `status_explain`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice targets the remaining heavy runtime explain bundle:
  `status_base_explain.rs`, `status_runtime.rs`, and
  `preemptive_status_lines*.rs`.
- It removes a repeated region-distribution temp vector, drops redundant
  summary self-scans, and pre-reserves the preemptive status buffer.
- It adds benchmark coverage for the status explain path in
  `chimera-lab metadata-perf-smoke`.

## Change

- `crates/chimera-mesh/src/runtime/status_base_explain.rs`
  - formats region distribution directly into one `String`;
  - removes redundant summary self-scans that were always tautological;
  - preserves output keys, ordering, and redaction.
- `crates/chimera-mesh/src/runtime/status_runtime.rs`
  - reserves extra output capacity before appending preemptive status lines.
- `crates/chimera-mesh/src/runtime/preemptive_status_lines.rs`
  - pre-reserves the preemptive/standby status line buffer.
- `crates/chimera-lab/src/metadata_perf.rs`
  - adds the `status_explain` benchmark;
  - surfaces it in the smoke output and JSON report.
- `chimera-pq/justfile`
  - extends `metadata-perf-smoke-selfcheck` to require the new status explain
    benchmark field.
- `docs/PERFORMANCE.md`
  - records the new `status_explain` slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh tests_preemptive_status`
- `cargo test -q -p chimera-mesh runtime_status_explain`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo clippy -q -p chimera-lab --all-targets -- -D warnings`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output after this slice:

```json
{"hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint","lane_document_plan_snapshot_access","live_binding_reload_index","live_dps_plan_path_from_payload","status_explain"],"status_explain_iterations":10000,"status_explain_peer_count":64,"status_explain_ops_per_sec":17757,"status_explain_p95_ns":57309}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Larger samples may still expose another hotspot.

## Rollback

- Restore the region-distribution `Vec`/`join()` builder in
  `status_base_explain.rs`.
- Restore the redundant summary self-scans if they are needed again.
- Remove the pre-reserve in `status_runtime.rs` and
  `preemptive_status_lines.rs`.
- Remove the `status_explain` benchmark and related `docs/PERFORMANCE.md`
  bullet.
