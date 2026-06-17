# Workflow Attestation: Policy-Driven Multipath Runtime Planning

Scope: make `MeshPathPolicy` carry the WEAVE multipath mode into the normal
runtime planner so direct `plan_path`, failover and health reselection can
rebuild a multipath schedule without changing carrier datapath or inspecting
transit payload.

## ANALYSIS

Status: done.

Evidence:

- `AGENTS.md`, `CHIMERA-PQ_MVP_SPEC.md`, `Agent.md`, and
  `docs/EXECUTION_MODE_NO_TIMELINES.md` were reread before implementation.
- Current gap was confirmed in
  `crates/chimera-mesh/src/runtime/path_planner.rs`: direct `plan_path`
  always built `MeshMultipathMode::Off`.
- DPS-only replacement already existed in
  `crates/chimera-mesh/src/runtime/plan_ops_dps_eval.rs`.

Conclusion:

- This is MVP scope because `PathPlan`, failover/recovery, peer transit and
  route explanation are part of the WEAVE mesh-node MVP.
- This is not a Real-World PASS or carrier-bound runtime proof.

## PLAN

Status: done.

Plan:

1. Add `multipath_mode: Option<MultipathMode>` to `MeshPathPolicy`.
2. Preserve `mesh_multipath_mode` during DPS policy parsing.
3. Let direct `plan_path` build its initial multipath schedule from policy.
4. Keep direct `plan_path` route binding closed (`None`).
5. Keep DPS route binding behavior in `apply_dps_multipath_schedule`.
6. Add regression tests for direct planning, parser, failover, reselection and
   auto health filtering.
7. Run source gates before commit/release.

## TEAM_CRITIQUE

Status: done.

Roles:

- Architect: agreed only for planner/policy scope; rejected DHT/DPS fabric,
  generated route ids and any Real-World PASS claim.
- Senior developer: found a blocker after implementation: counting
  `multipath_mode` as a manual override would disable auto health filtering.
- Tester: accepted source proof for commit only if the new
  `direct_planning.rs` test file is committed; Real-World PASS still requires
  release and SSH stand proof.
- Security engineer: accepted the control-plane policy approach; transit
  payload must remain opaque/sealed and must not choose multipath mode.
- DevOps: release delivery remains blocked until commit, tag, GitHub
  Release/Latest and one-command install/update on laptop and VPS.
- Critic: forbade `Real-World PASS`, `M4/M5 closed`, `ship-ready` or runtime
  claims without remote evidence.

Consensus:

- Accepted: policy-driven schedule selection with route binding closed by
  default.
- Rejected: route id derivation from peers/endpoints/topology, public DPS/DHT,
  proxy/SOCKS proof, payload inspection and any source-only Real-World PASS.

## IMPLEMENTATION

Status: done.

Changed files:

- `crates/chimera-mesh/src/policy.rs`
- `crates/chimera-mesh/src/runtime/path_planner.rs`
- `crates/chimera-mesh/src/tests_multipath_schedule/direct_planning.rs`
- `crates/chimera-mesh/src/tests_multipath_schedule/mod.rs`
- `crates/chimera-mesh/src/tests_failover_health/failover_reselection.rs`
- `crates/chimera-mesh/src/tests_policy_parsers/policy_core.rs`
- existing `MeshPathPolicy` struct literals in mesh tests and lab code.

Result:

- `MeshPathPolicy` stores `multipath_mode`.
- DPS parsing preserves `mesh_multipath_mode`.
- Direct `plan_path` applies policy multipath mode to the schedule.
- Direct `plan_path` does not create carrier lane bindings or route binding.
- DPS route binding behavior remains explicit and unchanged.

## TEAM_CHECK

Status: done.

First check found one release-blocking regression:

- `multipath_mode` was initially added to `manual_override_fields`.
- `manual_override_fields().is_empty()` controls planner auto mode.
- A pure multipath policy would have disabled auto health filtering.

Resolution:

- Removed `multipath_mode` from `manual_override_fields`.
- Added regression coverage to keep auto health filtering active when only
  `multipath_mode` is set.

## FIX

Status: done.

Fix:

- `MeshPathPolicy::manual_override_fields()` no longer treats
  `multipath_mode` as a selection/manual override.
- Added
  `policy_multipath_mode_does_not_disable_auto_health_filtering`.

## RECHECK

Status: done.

Commands:

- `cargo fmt --all -- --check`: PASS.
- `cargo test -q -p chimera-mesh multipath_schedule`: PASS, 22 tests.
- `cargo test -q -p chimera-mesh tests_auto_profile`: PASS, 10 tests.
- `cargo test -q -p chimera-mesh tests_failover_health`: PASS, 8 tests.
- `cargo test -q -p chimera-mesh tests_policy_parsers::policy_core`: PASS, 9 tests.
- `cargo check -q --workspace`: PASS.
- `cargo test -q --workspace`: PASS.
- `cargo clippy -q --workspace --all-targets -- -D warnings`: PASS.
- `bash scripts/anti_monolith_guard.sh`: PASS.
- `just rust-no-hardcode-guard`: PASS.
- `bash scripts/chimera_installer_gate.sh`: PASS.
- `bash scripts/chimera_update_contract_smoke.sh`: PASS.

## FINAL_AUDIT

Status: source-level accepted, Real-World proof not done.

Accepted:

- Source-level planner/policy fix.
- Privacy boundary: no transit payload path change, no payload-based
  scheduling, no new carrier fallback.
- Anti-monolith split via `tests_multipath_schedule/direct_planning.rs`.

Not closed:

- GitHub Release/Latest for this source.
- One-command install/update on laptop `art@192.168.31.21`.
- One-command install/update on VPS `root@91.124.19.180`.
- Remote installed-binary multipath/replan proof.
- Remote Real-World datapath proof and rollback proof.

## REPORT

Status: partial.

This source change is ready for commit/release pipeline, subject to the release
steps and SSH stand verification. It must not be reported as Real-World PASS or
as MVP/M4/M5 completion until the remote proof bundle exists.
