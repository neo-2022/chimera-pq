# Performance Engineering Guidelines

## Scope

This guide covers CPU and memory efficiency on named hot paths in CHIMERA-PQ.
It does not change the product model: transit payload stays opaque and sealed.

## Transferable ideas

Reference study: LatticeLab showed a useful pattern set for hot compute paths.
The parts that transfer to Chimera are:

- SoA for small, bounded, hot metadata.
- Dense active-first storage for live peers, lanes, and flow state.
- Candidate-lane sets with rebuild thresholds, similar to a neighbor-list
  rebuild gate, not a persistent graph model.
- Profiling before and after each optimization.
- SIMD only after profiler proof, and only on contiguous fixed-width metadata.
- Compiler and build hints only as opt-in, measured profiles.

## What does not transfer

- Do not optimize sealed transit payload by parsing, sampling, caching, or
  logging it.
- Do not use SoA or active-first layouts for raw payload bytes or for any
  structure that would expose third-party traffic state.
- Do not keep unbounded caches keyed by secrets, raw endpoints, or payload
  material.
- Do not assume host-specific compiler flags or release defaults without an
  evidence trail.

## Hot-path targets

Keep the focus on routing and control metadata:

- route decisions;
- lane scores and admission state;
- peer health and queue state;
- path-plan snapshots;
- explain/diagnostic summaries.

If a change touches payload handling, it is not a metadata optimization and
must go through the security and privacy gates first.

## Adoption gate

Before landing a performance change:

1. Name the hot path.
2. Record a direct baseline and a CHIMERA baseline.
3. Compare latency, throughput, p95, CPU, and RSS.
4. Run a negative-path and security regression.
5. Record the rollback plan.
6. Keep all outputs redacted.

## Gates and artifacts

Use these checks and artifacts together:

- `just perf-smoke`
- `just metadata-perf-smoke`
- `just benchmark-regression-selfcheck`
- `just benchmark-regression-check`
- `just ship-readiness`
- `docs/benchmark_baseline.json`
- `docs/benchmark_latest.json`
- `docs/BENCHMARK_REGRESSION_GATE.json`
- `docs/SHIP_READINESS_REPORT.json`
- `docs/REPORT_PACK.json`

## Current applied slices

- `multipath_flow_lane_selection`: normal ordered active lane metadata uses a
  streaming scan instead of per-flow `Vec` allocation and `sort_by_key`; unsorted
  active bindings keep the sorted slow fallback for deterministic parity. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_MULTIPATH_FLOW_2026-06-26.md`.
- `path_planner_candidate_snapshot`: path planning now keeps per-call candidate
  slots that borrow peer state, cache normalized region keys, and materialize
  owned peers only after selection. This reduces repeated peer clones and region
  normalization scans while preserving deterministic selection, explain output,
  fail-closed behavior, and opaque sealed transit payload boundaries. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_PATH_PLANNER_CANDIDATES_2026-06-26.md`.
- `path_planner_selection_metrics_strings`: path-planner selection explain
  strings now build selected peer and stability summaries directly instead of
  collecting intermediate `Vec`s. This trims explain overhead inside
  `path_planner_candidate_snapshot`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_PATH_PLANNER_SELECTION_METRICS_2026-06-26.md`.
- `path_planner_selection_metrics_capacity`: path-planner selection explain
  and metrics builders now pre-reserve their String buffers and format the
  selection-pressure summaries directly into pre-sized buffers. This trims
  additional explain overhead inside `path_planner_candidate_snapshot`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_PATH_PLANNER_SELECTION_METRICS_CAPACITY_2026-06-27.md`.
- `path_planner_selection_metrics_peer_summary`: path-planner peer selection
  summary now gathers ids, regions, endpoints, scores, sums, and region counts
  in one pass, with shared redacted label helpers. This trims selected-peer
  explain overhead inside `path_planner_candidate_snapshot`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_PATH_PLANNER_SELECTION_METRICS_PEER_SUMMARY_2026-06-27.md`.
- `path_planner_selection_explain_capacity`: path-planner selection finalization
  now reserves room for the full explain tail before appending the selection
  and candidate lines. This trims Vec growth inside
  `path_planner_candidate_snapshot`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_PATH_PLANNER_SELECTION_EXPLAIN_CAPACITY_2026-06-27.md`.
- `path_planner_selection_explain_push_lines`: path-planner selection explain
  sections now push formatted lines through small direct helpers instead of
  repeated `format!` calls. This trims formatting overhead inside
  `path_planner_candidate_snapshot`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_PATH_PLANNER_SELECTION_EXPLAIN_PUSH_LINES_2026-06-27.md`.
- `path_planner_selection_finalize_capacity`: path-planner selection finalizer
  now reserves enough room for the current explain tail and materializes
  selected peers with exact capacity. This trims one more growth step inside
  `path_planner_candidate_snapshot`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_PATH_PLANNER_SELECTION_FINALIZE_CAPACITY_2026-06-27.md`.
- `path_planner_selection_finalize_distinct_region_hashset`: path-planner
  selection finalizer now counts distinct candidate and selected regions
  through borrowed HashSets instead of tree-based sets. This trims distinct
  region bookkeeping inside `path_planner_candidate_snapshot`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_PATH_PLANNER_SELECTION_FINALIZE_DISTINCT_REGION_HASHSET_2026-06-27.md`.
- `selection_policy_region_lookup`: region diversity and resilient spread now
  look up normalized regions by borrowed `&str` and only clone normalized
  region keys on first insert. This trims region-cap bookkeeping inside
  `path_planner_candidate_snapshot`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_SELECTION_POLICY_REGION_LOOKUP_2026-06-27.md`.
- `selection_policy_region_scan`: region-cap bookkeeping and resilient spread
  now use small pre-reserved linear region lists instead of tree-based maps and
  sets. This trims short-region bookkeeping inside
  `path_planner_candidate_snapshot`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_SELECTION_POLICY_REGION_SCAN_2026-06-27.md`.
- `standby_shadow_explain_snapshot`: standby render/adapt now capture the
  preemptive-shadow fields once per call and reuse that snapshot instead of
  rescanning `explain`. This trims explain overhead inside
  `path_planner_candidate_snapshot` and `live_dps_plan_path_from_payload`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_STANDBY_SHADOW_EXPLAIN_SNAPSHOT_2026-06-27.md`.
- `path_planner_setup_discovery_explain`: `plan_path` setup now reserves explain
  capacity up front, formats `join_mode` with a static label, and joins
  discovery source names without cloning the source set first. This trims setup
  overhead inside `path_planner_candidate_snapshot`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_PATH_PLANNER_SETUP_DISCOVERY_2026-06-27.md`.
- `standby_shadow_redaction`: standby shadow explain now redacts switch targets
  and standby targets directly from selected peers instead of cloning a separate
  peer-id list first. This trims explain overhead inside
  `path_planner_candidate_snapshot`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_STANDBY_SHADOW_REDACTION_2026-06-27.md`.
- `connect_retry_profile`: connect priority, retry-plan, and backoff-profile
  strings now build directly into a single buffer instead of collecting
  intermediate vectors and joining them. This trims selection summary overhead
  inside `path_planner_candidate_snapshot`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_CONNECT_RETRY_PROFILE_2026-06-27.md`.
- `connect_backoff_profile_prefix`: the selected-peer backoff profile now uses
  a constant metadata prefix plus direct decimal `fanout` append, with golden
  tests for exact output. This keeps the diagnostic field metadata-only and
  avoids production `write!` formatting in this small helper. The metadata
  smoke passed with `network_state=not_modified`, but this slice does not prove
  a broad planner/datapath speedup. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_CONNECT_BACKOFF_PROFILE_PREFIX_2026-06-28.md`.
- `path_planner_selection_metrics_peer_single_pass`: path-planner peer summary
  now folds identity, connectivity, region counts, and stability counters into
  one pass over selected peers instead of running a separate stability scan.
  This trims explain overhead inside `path_planner_candidate_snapshot`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_PATH_PLANNER_SELECTION_METRICS_PEER_SINGLE_PASS_2026-06-27.md`.
- `path_planner_selection_region_counts_small_vec`: selected-region counts in
  the path-planner peer summary now use a small vector with one final
  normalized-region sort instead of tree bookkeeping on every selected peer.
  This preserves lexicographic explain output while trimming short-region
  metadata overhead inside `path_planner_candidate_snapshot` and
  `live_dps_plan_path_from_payload`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_PATH_PLANNER_SELECTION_REGION_COUNTS_SMALL_VEC_2026-06-28.md`.
- `live_dps_plan_path_from_payload`: DPS explain summary now captures the
  needed keys once, passes that snapshot into both summary appenders, and the
  live plan path now reuses that same parse snapshot for policy adaptation and
  multipath schedule binding instead of rescanning the payload. `chimera-lab
  metadata-perf-smoke` now includes a benchmark for the live DPS plan path.
  See
  `WORKFLOW_ATTESTATION_METADATA_PERF_LIVE_DPS_PLAN_PATH_FROM_PAYLOAD_2026-06-27.md`.
- `dps_payload_explain_summary`: DPS payload hints and decision/standby
  summaries now use direct line builders, reuse one hints summary string across
  both hint branches, and keep the hot explain path on the live plan path.
  This trims formatting overhead inside `live_dps_plan_path_from_payload`.
  See `WORKFLOW_ATTESTATION_METADATA_PERF_DPS_PAYLOAD_EXPLAIN_SUMMARY_2026-06-27.md`.
- `preemptive_helpers_hints`: the shared hints summary formatter now builds
  directly into sized buffers and appends the source label without temporary
  string churn. This trims shared status/DPS summary overhead inside
  `status_explain` and `live_dps_plan_path_from_payload`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_PREEMPTIVE_HELPERS_HINTS_2026-06-27.md`.
- `status_report_builder_shadow`: status report shadow summaries now build
  directly into sized buffers, and the shared compact-consistency path keeps
  using the tuple-return helper instead of a second summary scan. This trims
  report-builder overhead inside `status_explain`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_STATUS_REPORT_BUILDER_SHADOW_2026-06-27.md`.
- `dps_payload_explain_capacity`: DPS payload explain now reserves room for
  the long decision/standby tail before appending summary lines. This trims Vec
  growth inside `live_dps_plan_path_from_payload` and related DPS explain
  paths. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_DPS_PAYLOAD_EXPLAIN_CAPACITY_2026-06-27.md`.
- `dps_payload_snapshot_fast_flags`: the parsed DPS snapshot now stores the
  mesh-keys fingerprint once, returns it by borrow, keeps fast presence flags
  for the hot adaptation keys, and reuses copied route-binding ids in the live
  DPS path. This trims repeated lookup and clone work inside
  `live_dps_plan_path_from_payload`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_DPS_PAYLOAD_SNAPSHOT_FAST_FLAGS_2026-06-28.md`.
- `dps_standby_shadow_cleanup_single_pass`: DPS standby-shadow adaptation now
  removes old standby lines and redacts the preemptive switch target in one
  retained explain pass. This trims one rescan inside
  `live_dps_plan_path_from_payload` while preserving redaction and explain
  ordering. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_DPS_STANDBY_SHADOW_CLEANUP_SINGLE_PASS_2026-06-28.md`.
- `dps_payload_explain_tail_snapshot_scope`: DPS payload explain annotation now
  removes stale hint lines first, captures the summary snapshot from the
  cleaned pre-existing plan explain, then appends the new DPS metadata, hint,
  decision, and standby tail from one pre-sized buffer. This keeps the same
  explain tail order and redaction shape while reducing the snapshot scan
  scope inside `live_dps_plan_path_from_payload`; the oversized summary file
  was also split into snapshot/decision/standby modules. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_DPS_PAYLOAD_EXPLAIN_TAIL_SNAPSHOT_SCOPE_2026-06-28.md`.
- `status_explain`: status explain formatting now reuses a direct region
  formatter, skips redundant summary self-scans, and pre-reserves the
  preemptive status buffer. `chimera-lab metadata-perf-smoke` now includes a
  benchmark for the status explain path. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_STATUS_EXPLAIN_2026-06-27.md`.
- `status_explain_region_distribution_counts`: status region distribution now
  formats directly from the counts map instead of collecting an intermediate
  `Vec` first. This trims one allocation stage inside `status_explain`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_STATUS_EXPLAIN_REGION_DISTRIBUTION_COUNTS_2026-06-27.md`.
- `status_explain_tightening`: status region distribution now builds directly,
  the table-policy and table-enforcement summary self-checks no longer redo
  redundant substring scans, the remaining status explain summary blocks now
  use direct sized builders, and the status explain orchestrator appends the
  preemptive shadow block directly. This trims the remaining explain churn
  inside `status_explain`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_STATUS_EXPLAIN_TIGHTENING_2026-06-28.md`.
- `status_preemptive_shadow_lines_append`: status preemptive/standby lines now
  append directly into the caller buffer instead of building a temporary
  standby Vec first. This trims allocation churn inside `status_explain`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_STATUS_PREEMPTIVE_SHADOW_LINES_APPEND_2026-06-27.md`.
- `auto_recovery_candidate_collection`: auto-recovery candidate gathering now
  pre-reserves the candidate Vec and explain buffer, while the auto-recovery
  summary, candidate counter explain paths, and recovery explain lines avoid
  temporary string growth.
  This trims planner recovery overhead inside `path_planner_candidate_snapshot`
  and `live_dps_plan_path_from_payload`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_AUTO_RECOVERY_CANDIDATE_COLLECTION_2026-06-27.md`.
- `auto_recovery_selection_metrics_peer`: auto-recovery peer selection metrics
  now gather ids, regions, endpoints, scores, sums, averages, and region
  counts in one pass with direct String builders and pre-sized buffers. This
  trims selection-metadata overhead inside `build_selected_peer_metrics`.
  See
  `WORKFLOW_ATTESTATION_METADATA_PERF_AUTO_RECOVERY_SELECTION_METRICS_PEER_2026-06-27.md`.
- `table_consistency`: setup compact consistency now parses the compact string
  directly instead of building temporary `format!` needles, and the runtime
  consistency summary/warn gate now build direct strings without temp vectors;
  the shared `setup_compact_consistency()` helper now returns the summary and
  match result together so call sites avoid parsing the summary twice. The same
  `status_explain` benchmark covers this path because status/report explain
  pulls it in directly. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_TABLE_CONSISTENCY_2026-06-27.md`.
- `discovery_rebuild_fingerprint`: rebuild-trigger fingerprinting now reuses a
  single normalized region distribution snapshot inside `rebuild_trigger_fingerprint`
  instead of rebuilding it twice per trigger, and `chimera-lab metadata-perf-smoke`
  now includes a discovery-rebuild fingerprint benchmark. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_REBUILD_FINGERPRINT_2026-06-26.md`.
- `discovery_update_noop_dirty_set`: repeated identical discovery records now
  refresh peer liveness without changing rebuild-relevant metadata counters or
  raising a pending multipath rebuild. Changes to endpoint, region, load score,
  or reliability score remain dirty and raise `peer_table_changed`.
  `chimera-lab metadata-perf-smoke` now measures this no-op merge path directly
  and keeps the JSON output aggregate-only. This is lab/control-plane evidence,
  not a runtime or Real-World PASS. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_DISCOVERY_UPDATE_NOOP_DIRTY_SET_2026-06-28.md`.
- `affected_peer_dirty_invalidation`: pending multipath rebuild signals now
  carry aggregate dirty metadata: `dirty_scope=unknown|peer_set` and
  `affected_peer_count=N`. Discovery, health, and performance updates count
  only peers whose rebuild-relevant metadata actually changed; repeated no-op
  records stay out of the count, while stale eviction or table enforcement
  falls back to `unknown/0`. Explain output shows only aggregate scope/count,
  with no peer id, endpoint, route key, payload, or stand details. This is
  lab/control-plane evidence and does not claim selective planner rebuild,
  runtime behavior, SSH-stand, or Real-World PASS. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_AFFECTED_PEER_DIRTY_INVALIDATION_2026-06-28.md`.
- `lane_document_plan_snapshot_access`: hot carrier paths now borrow the plan
  snapshot instead of cloning it, and `chimera-lab metadata-perf-smoke` now
  compares borrowed versus owned access. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_LANE_DOCUMENT_PLAN_SNAPSHOT_ACCESS_2026-06-26.md`.
- `lane_document_render_parse`: lane document metadata render/parse now has a
  direct smoke metric, builds CSV rows and plan snapshot tab comments without
  per-row temporary `format!`/`Vec` allocations, and parses row fields with
  bounded split helpers instead of collecting intermediate vectors or joining
  row buffers. This preserves the lane document format, round-trip behavior,
  redaction, and sealed opaque transit payload boundary. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_LANE_DOCUMENT_RENDER_PARSE_2026-06-28.md`.
- `peer_update_state_publish_generation`: peer update runtime state now carries
  an `endpoint_generation`, upgrades fresh legacy state into the generation
  contract, and skips fresh no-op rewrites only when the same advertisement is
  already private and generation-tagged. This reduces needless state-file churn
  for unchanged auto-bound update endpoints while preserving rebind visibility.
  CLI advertise now rejects invalid zero generations and has a contract test
  proving runtime/private endpoint state and update state publish together
  without a manual port. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_PEER_UPDATE_STATE_PUBLISH_GENERATION_2026-06-28.md`.
- `peer_update_state_publish_metric`: peer update state publish now has a
  shared in-memory decision API that separates no-op from changed-generation
  decisions before the file writer performs I/O. `chimera-lab
  metadata-perf-smoke` measures both decision paths directly and keeps the JSON
  output limited to counters/latencies plus `network_state=not_modified` and
  `transit_payload_policy=opaque_sealed_payload_untouched`. This proves the
  metadata publish decision cost without measuring temp files, sockets, or local
  runtime behavior. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_PEER_UPDATE_STATE_PUBLISH_METRIC_2026-06-28.md`.
- `live_binding_reload_noop_fast_path`: reload loop now skips snapshot replace
  and worker reconcile when the reloaded live document or reload error matches
  the current snapshot, and an ignored carrier smoke covers the no-op path.
  See `WORKFLOW_ATTESTATION_METADATA_PERF_LIVE_BINDING_RELOAD_NOOP_2026-06-26.md`.
- `live_binding_reload_index`: changed reloads now borrow the live document
  during snapshot replacement and build a borrowed desired-binding index so the
  reconcile path clones registrations only when a worker actually needs a new
  copy. The main `chimera-lab metadata-perf-smoke` now includes it, and an
  ignored carrier smoke exercises the changed reload path. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_LIVE_BINDING_RELOAD_INDEX_2026-06-26.md`.
- `live_binding_reload_retain`: changed reload reconcile now evicts stale
  workers directly with `BTreeMap::retain()` instead of collecting a temporary
  stale-binding vector first. This keeps worker cancellation, dispatcher clear,
  duplicate desired binding behavior, fail-closed reload handling, and opaque
  sealed transit payload boundaries unchanged while trimming allocation churn
  inside `live_binding_reload_index`. See
  `WORKFLOW_ATTESTATION_METADATA_PERF_LIVE_BINDING_RELOAD_RETAIN_2026-06-28.md`.

## Wording to keep

- hot metadata only
- opaque sealed payload
- candidate-lane sets
- opt-in compiler flags

## Related docs

- `ARCHITECTURE.md`
- `OPERATIONS.md`
- `SPEED_ROOT_CAUSE_2026-05-27.md`
- `AEAD_PERFORMANCE_AND_PQ_CRYPTO_2026-05-27.md`
- `WORKFLOW_ATTESTATION_MULTIPATH_REBUILD_CONTROL_2026-06-18.md`
- `WORKFLOW_ATTESTATION_METADATA_PERF_REBUILD_FINGERPRINT_2026-06-26.md`
