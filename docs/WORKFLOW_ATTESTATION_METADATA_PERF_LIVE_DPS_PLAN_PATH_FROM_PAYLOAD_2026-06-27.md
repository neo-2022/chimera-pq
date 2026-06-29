# CHIMERA Metadata Performance Attestation: Live DPS Plan Path From Payload

## Scope

- Date: 2026-06-27
- Hot path: `live_dps_plan_path_from_payload`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice stays on the live `plan_path_from_dps_payload` path.
- It reuses one parsed payload snapshot for policy adaptation, route binding,
  and explain metadata while keeping the explain text unchanged.
- It adds benchmark coverage for the exact live path in
  `chimera-lab metadata-perf-smoke`.

## Change

- `crates/chimera-mesh/src/runtime/dps_payload_explain_summary.rs`
  - captures DPS explain metadata once into an owned snapshot;
  - both summary appenders read from that snapshot instead of rescanning
    `explain`;
  - preserves output keys and ordering.
- `crates/chimera-mesh/src/runtime/dps_payload_explain.rs`
  - captures the snapshot once after hints are appended and passes it to both
    summary appenders.
- `crates/chimera-mesh/src/dps_payload_snapshot.rs`
  - parses the DPS payload once into shared policy and metadata snapshot
    fields.
- `crates/chimera-mesh/src/runtime/plan_ops_dps_eval.rs`
  - reuses that snapshot for policy adaptation and multipath schedule binding.
- `crates/chimera-mesh/src/policy.rs`
  - routes `MeshPathPolicy::from_dps_payload()` through the shared parser.
- `crates/chimera-lab/src/metadata_perf.rs`
  - adds the `live_dps_plan_path_from_payload` benchmark;
  - surfaces it in the smoke output and JSON report.
- `docs/PERFORMANCE.md`
  - records the new `live_dps_plan_path_from_payload` slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh tests_dps_explain`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `just metadata-perf-smoke`

`just metadata-perf-smoke` output after this slice:

```json
{"hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint","lane_document_plan_snapshot_access","live_binding_reload_index","live_dps_plan_path_from_payload"],"live_dps_plan_path_from_payload_iterations":10000,"live_dps_plan_path_from_payload_peer_count":64,"live_dps_plan_path_from_payload_ops_per_sec":4361,"live_dps_plan_path_from_payload_p95_ns":243660}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Larger samples may still expose another hotspot.

## Rollback

- Restore repeated `explain_value` scans in `dps_payload_explain_summary.rs`.
- Remove the snapshot capture in `dps_payload_explain.rs`.
- Restore the `explain_value` helper/re-export if another caller needs it again.
- Remove the `live_dps_plan_path_from_payload` benchmark and the related
  `docs/PERFORMANCE.md` bullet.
