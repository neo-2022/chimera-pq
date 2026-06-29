# CHIMERA Metadata Performance Attestation: Path Planner Candidate Snapshots

## Scope

- Date: 2026-06-26
- Hot path: `path_planner_candidate_snapshot`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Council Result

- Architect: use a per-call candidate snapshot/slot model only inside
  `plan_path`; do not create a persistent cache.
- Senior Rust: keep `MeshPathPlan.selected_peers` owned, but use
  `CandidateSlot<'a>` for filtering, recovery, spread bonus and selection.
- Tester: require parity tests for region caps, deterministic ordering,
  recovery, explain counters and fail-closed behavior.
- Security: accepted only if the change stays metadata-only, has no payload
  inspection, no unsafe code, and keeps redacted diagnostics.
- DevOps: classify this as Lab/Metadata PASS only; Real-World PASS still
  requires SSH runtime/datapath evidence.
- Critic: reject any change to `candidate#N`, raw-vs-normalized region explain
  semantics, selected peer order or fail-closed behavior.

## Change

- `crates/chimera-mesh/src/runtime.rs`
  - adds private `CandidateSlot<'a>` with borrowed peer state, normalized
    region key and current selection score;
  - adds redacted `Debug` for `CandidateFilter` and `CandidateSlot`.
- `crates/chimera-mesh/src/runtime/candidate_filter.rs`
  - `collect_candidates` now returns `Vec<CandidateSlot<'_>>`;
  - accepted peers are not cloned during filtering;
  - accepted region keys are normalized once and reused by later stages.
- `crates/chimera-mesh/src/runtime/path_planner_recovery*.rs`
  - auto-recovery keeps candidate slots through health/filter relax steps.
- `crates/chimera-mesh/src/runtime/selection_policy*.rs`
  - region diversity, region cap and resilient spread bonus use cached
    normalized region keys and slot scores.
- `crates/chimera-mesh/src/runtime/path_planner_finalize.rs`
  - selection sort keeps the previous comparator:
    score desc, load asc, reliability desc, node_id asc;
  - final `MeshPeerState` clone happens only for selected slots.
- `crates/chimera-lab/src/metadata_perf.rs`
  - `metadata-perf-smoke` now also measures
    `path_planner_candidate_snapshot`.
- `justfile`
  - adds `path-planner-candidate-snapshot-perf-smoke`;
  - adds `path-planner-candidate-snapshot-perf-smoke-selfcheck`.
- `docs/PERFORMANCE.md`
  - records the applied `path_planner_candidate_snapshot` slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo check -q -p chimera-mesh`
- `cargo test -q -p chimera-mesh path_planner_candidate_snapshot`
- `cargo test -q -p chimera-mesh tests_selection_behavior`
- `cargo test -q -p chimera-mesh tests_selection_policy`
- `cargo test -q -p chimera-mesh tests_auto_profile`
- `cargo test -q -p chimera-mesh tests_dps_runtime_flow`
- `cargo test -q -p chimera-mesh tests_failover_health`
- `cargo test -q -p chimera-mesh tests_multipath_schedule`
- `cargo test -q -p chimera-mesh runtime_planning`
- `cargo test -q -p chimera-cli tests_connect_probe_flow`
- `cargo test -q -p chimera-lab metadata_perf`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `cargo clippy -q -p chimera-lab --all-targets -- -D warnings`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`
- `just path-planner-candidate-snapshot-perf-smoke`
- `just path-planner-candidate-snapshot-perf-smoke-selfcheck`
- `just perf-smoke`
- `just benchmark-regression-selfcheck`
- `just benchmark-regression-check`

Static security checks returned no hits for the active planner/filter/selection
files:

- `unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!`
- logging macros and direct print macros
- `payload|plaintext|decrypt|decode|inspect|classif|frame|body`

`just metadata-perf-smoke` output:

```json
{"status":"ok","kind":"metadata_perf_smoke","hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot"],"scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","iterations":100000,"active_bindings":16,"fast_sorted_ops_per_sec":1121551,"slow_sorted_fallback_ops_per_sec":158176,"fast_p95_ns":1235,"slow_sorted_fallback_p95_ns":6676,"fast_vs_fallback_speedup_pct":609.05,"path_planner_iterations":10000,"path_planner_peer_count":64,"path_planner_candidate_snapshot_ops_per_sec":7128,"path_planner_candidate_snapshot_p95_ns":146506,"network_state":"not_modified"}
```

`just benchmark-regression-check` reported:

```json
{"status":"ok","message_en":"Benchmark checks passed and report is ready.","message_ru":"Проверки бенчмарка пройдены, отчет готов.","created_at_unix_sec":1782496631,"config_smoke":true,"smoke":true,"fuzz_smoke":true,"net_sim":true,"perf_smoke":true,"iterations":20000,"encode_ops_per_sec":7747551,"decode_ops_per_sec":8032390,"encoded_total_bytes":24280000,"decoded_total_payload_bytes":24000000,"min_encode_ops":null,"min_decode_ops":null,"net_sim_reconnect_events":1,"net_sim_dropped":18,"net_sim_attempts":100}
```

## What Is Not Closed

- This is not broad WEAVE datapath performance closure.
- This is not Real-World PASS.
- SSH runtime/datapath/load checks on the external stand remain separate.
- Discovery/rebuild fingerprint optimization remains open.
- Live binding reload/index optimization remains open.

## Rollback

- Revert `CandidateSlot<'a>` usage in `candidate_filter`, recovery, selection
  and finalize back to owned `Vec<MeshPeerState>`.
- Remove the path-planner fields from `metadata_perf.rs`.
- Remove the two path-planner `justfile` smoke targets.
