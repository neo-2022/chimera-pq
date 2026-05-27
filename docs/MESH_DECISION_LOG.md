# MESH Decision Log

Purpose: compact chronological index of practical implementation decisions for
CHIMERA-PQ mesh MVP.

## 2026-05-21
- Decision: add operator cheat-sheet for traffic-profile selection in first-launch execution gate.
- Why: reduce profile-selection guesswork during staged launch and make expected behavior explicit per usage class.
- Evidence:
  - `docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md` section `Profile cheat-sheet (operator quick choice)`
- Follow-up: adjust profile descriptions only after real multi-host evidence indicates mismatch between expected and observed behavior.

- Decision: add profile smoke gate to validate wrapper/preset contract for all traffic profiles in staged mode.
- Why: launch gate needs deterministic proof that `--traffic-profile` and env wrapper path stay runnable for all supported presets, even when real connectivity is partially blocked.
- Evidence:
  - `scripts/mesh_launch_preflight_profile_smoke.sh`
  - `justfile` targets:
    - `mesh-launch-preflight-profile-smoke`
    - `mesh-launch-preflight-profile-smoke-selfcheck`
    - `mesh-launch-gate-selfcheck` includes both
  - `docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md` profile smoke PASS semantics
- Follow-up: keep profile list in smoke script synchronized with CLI parser accepted values.

- Decision: switch preflight wrapper/operator runbook to traffic-profile-first contract and keep policy payload as optional override.
- Why: operators should launch with stable named presets instead of manually composing long payload strings; this reduces input errors and keeps launch behavior reproducible.
- Evidence:
  - `scripts/mesh_launch_preflight_pair.sh`
  - `scripts/mesh_launch_preflight_env_guard.sh`
  - `configs/mesh_launch_preflight.vps.env.example`
  - `configs/mesh_launch_preflight.laptop.env.example`
  - `docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md`
  - `justfile` profile targets (`mesh-launch-preflight-side-{a,b}-profile`)
- Follow-up: add profile-specific expected readiness guidance (when to prefer privacy/speed/low-latency preset) after first real multi-node evidence run.

- Decision: isolate `mesh-launch-preflight-evidence-smoke` into `/tmp` artifacts and add docs-preserve selfcheck.
- Why: smoke runs must not overwrite real launch evidence in `docs/MESH_LAUNCH_PREFLIGHT_{VPS,LAPTOP,VERIFY}.json`.
- Evidence:
  - `scripts/mesh_launch_preflight_evidence_smoke.sh` uses `/tmp/chimera_mesh_launch_preflight_*_smoke.json`.
  - `justfile` target `mesh-launch-preflight-evidence-smoke-docs-preserve-selfcheck` proves checksums of real `docs/*` artifacts stay unchanged before/after smoke.
  - `mesh-launch-gate-selfcheck` includes preserve selfcheck.
- Follow-up: keep smoke scripts isolated from real evidence paths by default; only real staged/preflight commands should write `docs/*` launch artifacts.

- Decision: add preflight artifact freshness guard and include it in evidence-gate chain.
- Why: prevent false launch evidence from stale `VPS/LAPTOP/VERIFY` artifacts left from earlier runs.
- Evidence:
  - `scripts/mesh_launch_preflight_freshness_guard.sh`
  - `justfile` targets:
    - `mesh-launch-preflight-freshness-guard`
    - `mesh-launch-preflight-freshness-guard-selfcheck`
  - `mesh-launch-preflight-evidence-guard` now starts with freshness check.
- Follow-up: if lab window changes, tune `CHIMERA_MESH_PREFLIGHT_MAX_AGE_SEC` rather than weakening guard logic.

- Decision: add staged preflight mode with `CHIMERA_MESH_ALLOW_REMOTE_MISSING=1` and `just mesh-launch-preflight-{vps,laptop}-staged`.
- Why: first peer execution must be able to generate local launch-preflight evidence without hard-failing on missing remote artifact; verify runs only after both artifacts are present.
- Evidence:
  - `scripts/mesh_launch_preflight_pair.sh`
  - `scripts/mesh_launch_preflight_env_guard.sh`
  - `justfile` staged targets
- Follow-up: use staged commands for first pass on each host, then run full evidence gate.

- Decision: add per-peer `launch-preflight` artifact guard (`mesh_launch_preflight_report_guard`) and evidence bundle command `just mesh-launch-preflight-evidence-guard`.
- Why: first-launch proof must validate raw VPS/laptop preflight reports before aggregate verify to avoid false confidence from only final merged artifact checks.
- Evidence:
  - `crates/chimera-lab/src/bin/mesh_launch_preflight_report_guard.rs`
  - `justfile` targets:
    - `mesh-launch-preflight-report-guard-vps`
    - `mesh-launch-preflight-report-guard-laptop`
    - `mesh-launch-preflight-report-guard-selfcheck`
    - `mesh-launch-preflight-evidence-guard`
- Follow-up: use `mesh-launch-preflight-evidence-guard` as required artifact gate after real VPS/laptop runs.

- Decision: make pair-preflight verify role-explicit via `CHIMERA_MESH_LOCAL_ROLE` (`vps|laptop`) and map local/remote artifacts to `--vps-report/--laptop-report` deterministically.
- Why: prevent false `launch-preflight-verify` failures caused by swapped report semantics when wrapper is executed from laptop side.
- Evidence:
  - `scripts/mesh_launch_preflight_pair.sh`
  - `scripts/mesh_launch_preflight_env_guard.sh`
  - `configs/mesh_launch_preflight.vps.env.example`
  - `configs/mesh_launch_preflight.laptop.env.example`
- Follow-up: keep env templates and wrapper contract synchronized for first real VPS↔laptop runbook.

- Decision: enforce strict env-file validation gate before any `mesh launch-preflight` pair run.
- Why: first-launch readiness must fail fast on malformed namespace/node/endpoint/output parameters instead of burning cycles in runtime probe attempts.
- Evidence:
  - `scripts/mesh_launch_preflight_env_guard.sh`
  - `justfile` targets:
    - `mesh-launch-preflight-env-guard-vps`
    - `mesh-launch-preflight-env-guard-laptop`
    - `mesh-launch-preflight-env-guard-selfcheck`
    - `mesh-launch-preflight-vps`/`mesh-launch-preflight-laptop` now call env guard first.
  - `mesh-launch-gate-selfcheck` now includes `mesh-launch-preflight-env-guard-selfcheck`.
- Follow-up: keep env guard and pair wrapper in lockstep when preflight env contract evolves.

- Decision: activate strict first-launch execution gate (`docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md`) and freeze workline on real VPS↔laptop mesh startup readiness.
- Why: prevent drift into non-blocking contract/test work before first factual mesh launch closure.
- Evidence:
  - `docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md` added with objective, DoD, and hard execution rules.
  - `docs/SHIP_READINESS_REPORT.md` truth boundary confirms remaining gap:
    `real_world_datapath_closed=false`.
- Follow-up: execute only launch-blocker tasks until DoD is proven.

- Decision: freeze `mesh route-explain` options parser/error-envelope contract.
- ADR: `docs/ADR/ADR_MESH_CLI_OPTIONS_PARSE_CONTRACT_2026-05-21.md`
- Why: deterministic operator diagnostics + stable machine-readable errors.
- Evidence:
  - `cargo test -q -p chimera-cli`
  - `cargo clippy -q -p chimera-cli --all-targets -- -D warnings`
  - `bash scripts/anti_monolith_guard.sh`
- Follow-up: runtime auto-connect endpoint preference/retry/backoff + explain.

- Decision: add explicit mesh auto-connect endpoint plan explain fields.
- ADR: `docs/ADR/ADR_MESH_AUTO_CONNECT_ENDPOINT_PLAN_2026-05-21.md`
- Why: deterministic connection order + retry/backoff visibility for operators.
- Evidence:
  - `cargo test -q -p chimera-mesh`
  - `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
  - `bash scripts/anti_monolith_guard.sh`
- Follow-up: surface endpoint-plan exhaustion in CLI route-explain action/reason.

- Decision: in warn+no-selected state, route-explain now emits explicit connect retry action.
- ADR: `docs/ADR/ADR_MESH_CLI_CONNECT_PLAN_OPERATOR_ACTION_2026-05-21.md`
- Why: operator gets practical reconnect instruction instead of generic warning.
- Evidence:
  - `cargo test -q -p chimera-cli route_explain_health`
  - `cargo test -q -p chimera-cli`
  - `cargo clippy -q -p chimera-cli --all-targets -- -D warnings`
  - `bash scripts/anti_monolith_guard.sh`
- Follow-up: wire runtime retry-budget counters into CLI action escalation.

- Decision: route-explain action now escalates using runtime retry-budget exhaustion signals.
- ADR: `docs/ADR/ADR_MESH_CLI_RETRY_BUDGET_ESCALATION_2026-05-21.md`
- Why: operator action must reflect factual recovery outcome, not only generic warn state.
- Evidence:
  - `cargo test -q -p chimera-cli route_explain_health`
  - `cargo test -q -p chimera-cli`
  - `cargo clippy -q -p chimera-cli --all-targets -- -D warnings`
  - `bash scripts/anti_monolith_guard.sh`
- Follow-up: expose retry counters as first-class route-explain JSON fields.

- Decision: expose recovery status as first-class route-explain JSON fields.
- ADR: `docs/ADR/ADR_MESH_CLI_RECOVERY_JSON_FIELDS_2026-05-21.md`
- Why: automation should read recovery state without parsing free-form explain.
- Evidence:
  - `cargo test -q -p chimera-cli`
  - `cargo clippy -q -p chimera-cli --all-targets -- -D warnings`
  - `bash scripts/anti_monolith_guard.sh`
- Follow-up: consume these fields in cross-envelope tests and operator escalation policy.

- Decision: enforce cross-envelope invariants for recovery first-class fields.
- ADR: `docs/ADR/ADR_MESH_CLI_RECOVERY_CROSS_ENVELOPE_CONTRACT_2026-05-21.md`
- Why: preserve strict boundary (success has recovery projection, error does not).
- Evidence:
  - `cargo test -q -p chimera-cli tests_json_operator_cross_contract`
  - `cargo test -q -p chimera-cli`
  - `cargo clippy -q -p chimera-cli --all-targets -- -D warnings`
  - `bash scripts/anti_monolith_guard.sh`
- Follow-up: add compact schema version/checksum for recovery field set.

- Decision: add recovery schema marker and field-set checksum in success envelope.
- ADR: `docs/ADR/ADR_MESH_CLI_RECOVERY_SCHEMA_MARKER_2026-05-21.md`
- Why: consumers need explicit compatibility marker for recovery contract evolution.
- Evidence:
  - `cargo test -q -p chimera-cli tests_json_operator_cross_contract`
  - `cargo test -q -p chimera-cli`
  - `cargo clippy -q -p chimera-cli --all-targets -- -D warnings`
  - `bash scripts/anti_monolith_guard.sh`
- Follow-up: enforce schema-marker checks in release-readiness integration scripts.

- Decision: synchronize ship/readiness guard contracts with evolved mesh
  route-explain/recovery truth and enforce post-write reality sync check order.
- ADR: `docs/ADR/ADR_MESH_SHIP_PIPELINE_CONTRACT_SYNC_2026-05-21.md`
- Why: prevent false FAIL caused by stale guard expectations instead of real regressions.
- Evidence:
  - `just truth-contract-check`
  - `just ship-report-contract-check`
  - `just ship-readiness`
- Follow-up: keep ship `steps` schema and mesh artifact checks versioned together.

- Decision: add first-class recovery projection fields in route-explain success
  envelope:
  - `connect_recovery_needed`
  - `connect_recovery_strategy`
  - `connect_recovery_projection_consistency`
  - `connect_recovery_projection_key`
  and bump recovery schema to `mesh_recovery_v5`.
- ADR: `docs/ADR/ADR_MESH_CLI_CONNECT_RECOVERY_NEEDED_FLAG_2026-05-21.md`
- Why: automation needs explicit reconnect-required signal without parsing text summaries.
- Evidence:
  - `cargo test -q -p chimera-cli tests_json_success_presence`
  - `cargo test -q -p chimera-cli tests_json_operator_cross_contract`
  - `just truth-contract-check`
  - `just ship-report-contract-check`
- Follow-up: keep downstream guards and report contracts pinned to recovery schema/version.

- Decision: align error envelope with success envelope for recovery contract parity:
  error JSON now also carries
  - `auto_recovery_attempts`
  - `auto_recovery_final_result`
  - `connect_retry_budget_exhausted`
  - `connect_recovery_needed`
  - `connect_recovery_strategy`
  - `connect_recovery_projection_consistency`
  - `connect_recovery_projection_key`
  - `route_explain_recovery_schema_version`
  - `route_explain_recovery_fields_checksum`
  with deterministic error-state values and action-derived projection key.
- Why: downstream tooling gets one stable schema in both success/error flows, with
  explicit not-applicable markers instead of missing fields.
- Evidence:
  - `cargo test -q -p chimera-cli tests_json_error_contract`
  - `cargo test -q -p chimera-cli tests_json_operator_cross_contract`
  - `cargo test -q -p chimera-cli tests_route_explain_error_internal_matrix`
  - `cargo test -q -p chimera-cli`
- Follow-up: tighten ship-level schema guard to assert recovery parity in error envelopes too.

- Decision: ship/truth guard now enforces recovery-schema parity for both
  `mesh_route_explain` (ok) and `mesh_route_explain_error` (error) envelopes.
- Why: prevent silent drift where error-path recovery fields diverge from the
  success contract or disappear from release artifacts.
- Evidence:
  - `just mesh-cli-recovery-schema-guard-selfcheck`
  - `just mesh-cli-recovery-schema-guard`
  - `just ship-report-contract-check`
- Follow-up: keep parity checks pinned when recovery schema/checksum evolves next time.

- Decision: refactor `mesh_cli_recovery_schema_guard` validation into testable core
  (`validate_obj`) and add unit tests for:
  - success envelope acceptance;
  - error envelope acceptance;
  - error-envelope mismatch rejection.
- Why: schema guard logic itself is now regression-tested and not only exercised
  via shell pipeline.
- Evidence:
  - `cargo test -q -p chimera-lab --bin mesh_cli_recovery_schema_guard`
  - `just mesh-cli-recovery-schema-guard-selfcheck`
  - `just mesh-cli-recovery-schema-guard`
- Follow-up: keep adding targeted negative tests whenever new recovery fields are introduced.

- Decision: add dedicated `mesh_route_explain_error_guard` and wire it into
  `truth-contract-check` with explicit `mesh-route-explain-error-smoke`.
- Why: error envelope contract must be validated by its own artifact guard,
  not only indirectly through recovery schema checks.
- Evidence:
  - `just mesh-route-explain-error-smoke`
  - `just mesh-route-explain-error-guard`
  - `just truth-contract-check`
- Follow-up: if error envelope gets new mandatory fields, extend this guard first, then ship gates.

- Decision: refactor `mesh_route_explain_error_guard` to a testable core
  (`validate_obj`) and add unit tests for valid error contract and projection-key
  mismatch rejection.
- Why: error guard behavior is now protected against silent regressions at Rust
  test level, not only through shell/selfcheck execution.
- Evidence:
  - `cargo test -q -p chimera-lab --bin mesh_route_explain_error_guard`
  - `just mesh-route-explain-error-guard-selfcheck`
  - `just mesh-route-explain-error-guard`
- Follow-up: add one more negative test when `route_explain_recovery_schema_version`
  changes to keep checksum/schema coupling explicit.

- Decision: make `mesh-route-explain-error-guard-selfcheck` assert and run
  `mesh_route_explain_error_guard` unit tests directly.
- Why: prevents accidental removal of guard tests while preserving green shell checks.
- Evidence:
  - `just mesh-route-explain-error-guard-selfcheck`
  - `just truth-contract-selfcheck`
- Follow-up: keep selfcheck regex list in sync whenever test function names change.

- Decision: add extra negative unit tests for `mesh_cli_recovery_schema_guard`:
  - reject `ok` envelope without `explain`;
  - reject `kind/status` mismatch.
- Why: catches structural envelope regressions before shell gates and report checks.
- Evidence:
  - `cargo test -q -p chimera-lab --bin mesh_cli_recovery_schema_guard`
  - `just mesh-cli-recovery-schema-guard-selfcheck`
  - `just mesh-cli-recovery-schema-guard`
- Follow-up: expand with checksum-evolution negative case on next recovery schema bump.

- Decision: require `mesh_cli_recovery_schema_guard` unit tests inside
  `mesh-cli-recovery-schema-guard-selfcheck` (name checks + direct `cargo test` run).
- Why: protects against silent deletion of negative guard tests while shell checks remain green.
- Evidence:
  - `just mesh-cli-recovery-schema-guard-selfcheck`
  - `just truth-contract-selfcheck`
- Follow-up: keep selfcheck test-name anchors updated if test functions are renamed.

- Decision: extend `mesh_route_explain_error_guard` with additional negative unit
  tests for:
  - recovery schema version mismatch;
  - blank `error_stage`.
- Why: these failures are high-value contract breaks and now fail fast at unit-test level.
- Evidence:
  - `cargo test -q -p chimera-lab --bin mesh_route_explain_error_guard`
  - `just mesh-route-explain-error-guard-selfcheck`
  - `just mesh-route-explain-error-guard`
- Follow-up: add checksum-mismatch negative test when recovery field-set changes again.

- Decision: remove recovery schema/checksum string drift in `chimera-cli` JSON contract tests by switching assertions to `route_explain_meta` constants via shared test helpers.
- Why: future recovery schema bumps should require one source-of-truth update and fail deterministically in tests, instead of leaving stale hardcoded literals across multiple files.
- Evidence:
  - `cargo test -q -p chimera-cli tests_json_success_presence`
  - `cargo test -q -p chimera-cli tests_json_operator_cross_contract`
  - `cargo test -q -p chimera-cli tests_json_error_contract`
  - `cargo test -q -p chimera-cli tests_json_contract`
  - `just truth-contract-selfcheck`
- Follow-up: apply the same de-duplication pattern to `chimera-lab` guard binaries when shared constants are exported cross-crate.

- Decision: centralize mesh connect-retry profile rendering into one runtime helper (`runtime/connect_retry_profile.rs`) and reuse it from both path-planner and auto-recovery selection metrics.
- Why: removes duplicated retry/backoff formatting logic and keeps route-explain diagnostics consistent across normal and recovery paths.
- Evidence:
  - `cargo test -q -p chimera-mesh tests::runtime_planning`
  - `cargo test -q -p chimera-mesh`
- Follow-up: if we introduce policy-driven retry budgets later, wire that policy into this single helper instead of per-domain string forks.

- Decision: enrich mesh connect retry-plan explain with endpoint port fallback chain (`ports=<current>|443|8443`) and keep logic centralized in `runtime/connect_retry_profile.rs`.
- Why: route-explain now reflects a realistic auto-connect strategy (retry + port fallback + next-peer fallback) useful for VPS/laptop deployment diagnostics.
- Evidence:
  - `cargo test -q -p chimera-mesh`
- Follow-up: when transport-level connector is wired, consume this same retry profile as execution input, not only explain output.

- Decision: make mesh connect retry port fallback policy-driven via `mesh_connect_fallback_ports` in `MeshPathPolicy` and feed it into runtime retry-plan explain generation.
- Why: removes residual hardcoded endpoint-port strategy from runtime behavior and lets operators tune fallback connect ports through mesh payload/config.
- Evidence:
  - `cargo test -q -p chimera-mesh`
  - `cargo check -q -p chimera-lab`
- Follow-up: expose `mesh_connect_fallback_ports` in CLI route-explain docs/examples and validate transport connector consumes the same order when execution path is wired.

- Decision: add `chimera-cli` route-explain contract test proving `mesh_connect_fallback_ports` payload affects final explain retry chain (`selected_peer_connect_retry_plan`).
- Why: ensures policy-driven connect fallback is validated end-to-end at CLI contract layer, not only within `chimera-mesh` internals.
- Evidence:
  - `cargo test -q -p chimera-cli tests_json_contract`
  - `cargo test -q -p chimera-cli tests_json_success_presence`
  - `cargo test -q -p chimera-cli tests_json_operator_cross_contract`
- Follow-up: add equivalent negative CLI error-contract case for malformed `mesh_connect_fallback_ports` payload value.

- Decision: add CLI error-contract negative case for malformed `mesh_connect_fallback_ports` payload (`mesh_connect_fallback_ports=abc`).
- Why: guarantees policy-parse failure for fallback-port misconfiguration is explicitly pinned at route-explain error-envelope layer (stage/action/category/backoff/resolution).
- Evidence:
  - `cargo test -q -p chimera-cli tests_json_error_contract`
  - `cargo test -q -p chimera-cli tests_json_contract`
  - `cargo test -q -p chimera-mesh`
- Follow-up: add shell-level artifact guard scenario for malformed fallback ports in report bundle once error artifact matrix grows.

- Decision: promote selected-peer connect retry/backoff profile from free-form `explain` text to dedicated route-explain JSON fields:
  - `selected_peer_connect_retry_plan`
  - `selected_peer_connect_backoff_profile`
- Why: makes mesh recovery/connect diagnostics machine-readable and stable for operator tooling and contract guards.
- Evidence:
  - `cargo test -q -p chimera-cli tests_json_success_presence`
  - `cargo test -q -p chimera-cli tests_json_contract`
  - `cargo test -q -p chimera-mesh`
- Follow-up: add ship-level guard assertions for new fields in JSON report checks.

- Decision: wire real mesh runtime connect probing into CLI via new `mesh connect-probe` command path and isolate command implementation into dedicated module (`mesh_cli/connect_probe_cmd.rs`) to avoid growing `mesh_cli/mod.rs` monolith.
- Why: moves mesh work from explain-only simulation toward executable first-launch diagnostics (actual endpoint connect attempts with policy-driven peer/port fallback), while preserving anti-monolith discipline.
- Evidence:
  - `cargo test -q -p chimera-cli`
  - `cargo test -q -p chimera-mesh`
  - `cargo clippy -q -p chimera-cli --all-targets -- -D warnings`
- Follow-up: add explicit `--timeout-ms` CLI flag and JSON-contract tests for `connect-probe` envelope/output schema before first real VPS↔laptop launch pass.

- Decision: add deterministic `chimera-mesh` runtime tests for real `connect_probe` execution order and fallback behavior.
- Why: first-launch gate requires proof that runtime connector follows the same policy-driven fallback strategy as explain diagnostics (`current endpoint -> fallback ports`).
- Evidence:
  - `cargo test -q -p chimera-mesh connect_probe`
  - `cargo test -q -p chimera-mesh`
  - `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
  - `bash scripts/anti_monolith_guard.sh`
- Follow-up: add a launch-preflight CLI report path that packages probe result + selected peer + rollback-safe status into one operator-facing artifact.

- Decision: add `mesh launch-preflight` CLI command as unified operator artifact for first-launch readiness checks.
- Why: first real VPS↔laptop launch needs one deterministic report combining connect probe result, selected peers, blockers, and rollback-safe network-state marker (`not_modified`) before live rollout.
- Evidence:
  - `cargo test -q -p chimera-cli tests_launch_preflight_json`
  - `cargo test -q -p chimera-cli`
  - `cargo clippy -q -p chimera-cli --all-targets -- -D warnings`
  - `bash scripts/anti_monolith_guard.sh`
- Follow-up: add docs snippet with exact launch-preflight command template for VPS/laptop pair and expected `ready`/`blocked` interpretation.

- Decision: add explicit VPS↔laptop `mesh launch-preflight` operator runbook and ready/blocked interpretation to first-launch execution gate doc.
- Why: reduce operator ambiguity and make first real preflight execution reproducible with concrete command templates and closure criteria.
- Evidence:
  - `docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md` now includes:
    - peer spec format,
    - VPS command template,
    - laptop command template,
    - deterministic `ready` vs `blocked` interpretation,
    - dual-artifact closure rule.
- Follow-up: run both preflight commands with real environment endpoints and attach produced JSON artifacts as launch evidence.

- Decision: add `mesh launch-preflight-verify` command to enforce dual-artifact readiness (`VPS` + `Laptop`) with one deterministic verdict.
- Why: first launch gate closure must be machine-checkable (`all_ready`) and not depend on manual JSON inspection.
- Evidence:
  - `cargo test -q -p chimera-cli tests_launch_preflight_verify_json`
  - `cargo test -q -p chimera-cli`
  - `cargo clippy -q -p chimera-cli --all-targets -- -D warnings`
  - `bash scripts/anti_monolith_guard.sh`
  - `docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md` updated with verify command and expected output.
- Follow-up: execute real environment preflight on both peers and then run `launch-preflight-verify` to produce `docs/MESH_LAUNCH_PREFLIGHT_VERIFY.json` as launch evidence.

- Decision: harden `mesh launch-preflight-verify` with cross-report namespace consistency gate and explicit blocker reasons (`vps_report_not_ready`, `laptop_report_not_ready`, `namespace_missing`, `namespace_mismatch`).
- Why: dual-host readiness must fail closed when reports are structurally good but semantically mismatched (different namespaces).
- Evidence:
  - `cargo test -q -p chimera-cli tests_launch_preflight_verify_json`
  - `cargo test -q -p chimera-cli`
  - `cargo clippy -q -p chimera-cli --all-targets -- -D warnings`
  - `bash scripts/anti_monolith_guard.sh`
- Follow-up: run real VPS/laptop preflight pair and archive resulting `MESH_LAUNCH_PREFLIGHT_VERIFY.json` as launch gate proof.

- Decision: add one-command smoke gate for launch-verify flow (`just mesh-launch-preflight-verify-smoke`) plus script selfcheck.
- Why: ensures `mesh launch-preflight-verify` remains runnable and deterministic in automation before real VPS↔laptop evidence collection.
- Evidence:
  - `just mesh-launch-preflight-verify-smoke-selfcheck`
  - `just mesh-launch-preflight-verify-smoke`
  - `cargo test -q -p chimera-cli tests_launch_preflight_verify_json`
  - `cargo test -q -p chimera-cli`
  - `cargo clippy -q -p chimera-cli --all-targets -- -D warnings`
  - `bash scripts/anti_monolith_guard.sh`
- Follow-up: execute real pair preflights and replace synthetic smoke evidence with environment artifacts (`MESH_LAUNCH_PREFLIGHT_VPS.json`, `MESH_LAUNCH_PREFLIGHT_LAPTOP.json`, `MESH_LAUNCH_PREFLIGHT_VERIFY.json`).

- Decision: add dedicated `chimera-lab` artifact guard for launch gate verify JSON (`mesh_launch_preflight_verify_guard`) and wire smoke/selfcheck targets in `justfile`.
- Why: launch gate evidence now has an explicit schema+invariant validator (`ready|blocked`, boolean coherence, blockers policy, namespace/network_state constraints).
- Evidence:
  - `just mesh-launch-preflight-verify-smoke-selfcheck`
  - `just mesh-launch-preflight-verify-smoke`
  - `just mesh-launch-preflight-verify-guard-selfcheck`
  - `just mesh-launch-preflight-verify-guard`
  - `cargo test -q -p chimera-lab --bin mesh_launch_preflight_verify_guard`
  - `cargo clippy -q -p chimera-lab --bin mesh_launch_preflight_verify_guard -- -D warnings`
- Constraint: workspace-wide `cargo clippy -p chimera-lab --all-targets -D warnings` is currently blocked by pre-existing `expect_err` usage in unrelated guard bins (`mesh_cli_recovery_schema_guard`, `mesh_route_explain_error_guard`).
- Follow-up: either (a) keep targeted clippy scope for mesh launch line, or (b) clean legacy clippy debt in those bins as separate quality task.

- Decision: add env-driven wrapper script `scripts/mesh_launch_preflight_pair.sh` and `just` targets for pair-preflight orchestration.
- Why: reduce manual command assembly and ensure local preflight + pair verify flow is reproducible with one operator entrypoint.
- Evidence:
  - `just mesh-launch-preflight-pair-selfcheck`
  - `cargo test -q -p chimera-cli`
  - `cargo clippy -q -p chimera-cli -p chimera-mesh -p chimera-lab --all-targets -- -D warnings`
  - `bash scripts/anti_monolith_guard.sh`
  - `docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md` updated with wrapper usage and env contract.
- Follow-up: run wrapper on each host with swapped LOCAL/REMOTE settings and collect real artifacts for final verify gate.

- Decision: add aggregated launch gate command `just mesh-launch-gate-selfcheck` (pair wrapper selfcheck + verify smoke + verify guard chain).
- Why: provides one deterministic preflight quality gate before attempting real VPS↔laptop evidence collection.
- Evidence:
  - `just mesh-launch-gate-selfcheck`
  - `cargo clippy -q -p chimera-cli -p chimera-mesh -p chimera-lab --all-targets -- -D warnings`
  - `bash scripts/anti_monolith_guard.sh`
- Follow-up: run real-host pair commands (`just mesh-launch-preflight-vps`, `just mesh-launch-preflight-laptop`) and archive final verify artifact.
