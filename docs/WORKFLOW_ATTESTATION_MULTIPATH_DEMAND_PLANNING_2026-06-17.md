# Workflow Attestation: Multipath Demand Planning

Date: 2026-06-17

Scope: source-level WEAVE multipath planner update in `chimera-mesh`.

## ANALYSIS

Status: done

- MVP scope confirmed from `CHIMERA-PQ_MVP_SPEC.md`: symmetric WEAVE node, peer
  transit/forwarding, CLI diagnostics, route explanation, tests, and transit
  payload opacity are in MVP.
- The existing multipath scheduler already enforced `local_traffic_reserve_pct=10`
  and `transit_capacity_budget_pct=90`, but active lane count was driven only by
  mode and selected peer count.
- The missing source-level behavior was explicit, privacy-safe demand planning
  over control-plane hints.

## PLAN

Status: done

- Add a coarse `mesh_multipath_demand=low|normal|high|bulk` policy hint.
- Keep raw Mbps, endpoint-derived, destination-derived, and payload-derived
  signals out of the planner.
- Add a dedicated `runtime/multipath_demand.rs` module.
- Add aggregate schedule diagnostics only.
- Keep carrier/datapath runtime untouched in this slice.

## TEAM_CRITIQUE

Status: done

- Architect: approved only a source-level planner slice; no Real-World/datapath
  claim.
- Senior Rust: approved a small demand module plus thin schedule wiring; warned
  against schedule monolith growth.
- Tester: required unit, redaction, negative parser, failover/reselection, and
  guard checks.
- Security: required opaque/sealed transit invariant and redacted diagnostics.
- DevOps: required no stand claim before GitHub one-command release/update proof.
- Critic: blocked any throughput/datapath PASS based only on route-explain or
  source tests.

## IMPLEMENTATION

Status: done

Files changed:

- `crates/chimera-mesh/src/policy.rs`
- `crates/chimera-mesh/src/policy_hints.rs`
- `crates/chimera-mesh/src/multipath_model.rs`
- `crates/chimera-mesh/src/runtime/multipath_demand.rs`
- `crates/chimera-mesh/src/runtime/multipath_schedule.rs`
- `crates/chimera-mesh/src/runtime/multipath_schedule_tests.rs`
- `crates/chimera-mesh/src/runtime/multipath_lane_admission.rs`
- `crates/chimera-mesh/src/runtime/path_planner.rs`
- `crates/chimera-mesh/src/runtime/plan_ops_dps_eval.rs`
- `crates/chimera-mesh/src/runtime/dps_payload_explain_hints.rs`
- focused tests under `crates/chimera-mesh/src/tests_multipath_schedule/`
  and `crates/chimera-mesh/src/tests_policy_parsers/`

Implemented behavior:

- Coarse demand policy values: `low`, `normal`, `high`, `bulk`.
- Demand planner computes requested/planned active lane count within the 90%
  transit budget.
- Schedule explain exports aggregate demand fields:
  - `multipath_schedule_demand_policy`
  - `multipath_schedule_demand_policy_source`
  - `multipath_schedule_demand_requested_active_lanes`
  - `multipath_schedule_demand_planned_active_lanes`
  - `multipath_schedule_demand_admitted_lane_capacity_pct`
  - `multipath_schedule_demand_unmet_lanes`
  - `multipath_schedule_demand_status`
  - `multipath_schedule_demand_rebuild_recommended`
- Transit payload policy remains `sealed_opaque_only`.

## TEAM_CHECK

Status: done

Initial check found a real defect:

- `scripts/anti_monolith_guard.sh` failed because `policy.rs` and
  `runtime/multipath_schedule.rs` exceeded line limits after the first patch.

Final tester audit found additional proof gaps before a source-level claim:

- explicit state/explain equality proof for new demand fields;
- explicit `requested = admitted + rejected` arithmetic proof;
- explicit demand preservation through failover/reselection replan paths;
- explicit truth boundary for telemetry/debounce/live local+transit behavior
  that this source slice does not implement.

## FIX

Status: done

- Split DPS hint parsing from `policy.rs` to `policy_hints.rs`.
- Split schedule internal tests from `multipath_schedule.rs` to
  `runtime/multipath_schedule_tests.rs`.
- Kept runtime behavior and public API shape unchanged except the new demand
  hint fields.
- Added focused tests for demand arithmetic, explain/state agreement, and
  demand preservation through failover/reselection schedule replacement.
- Marked telemetry/debounce/live traffic behavior as not implemented/not
  verified for this source-level slice instead of claiming it.

## RECHECK

Status: done

Commands run from `<repo-root>`:

- `cargo fmt --all -- --check` PASS
- `cargo check -q --workspace` PASS
- `cargo test -q -p chimera-mesh tests_multipath_schedule` PASS
- `cargo test -q -p chimera-mesh tests_policy_parsers` PASS
- `cargo test -q -p chimera-mesh tests_dps_policy` PASS
- `cargo test -q -p chimera-mesh tests_dps_runtime_flow` PASS
- `cargo test -q -p chimera-mesh multipath_schedule` PASS
- `cargo test -q -p chimera-mesh multipath_demand` PASS
- `cargo test -q -p chimera-mesh` PASS
- `cargo clippy -q --workspace --all-targets -- -D warnings` PASS
- `cargo test -q --workspace` PASS
- `bash scripts/anti_monolith_guard.sh` PASS
- `just rust-no-hardcode-guard` PASS
- `bash scripts/chimera_installer_gate.sh` PASS
- `bash scripts/chimera_update_contract_smoke.sh` PASS
- `bash scripts/chimera_start_contract_smoke.sh` PASS
- `bash scripts/chimera_stop_contract_smoke.sh` PASS
- `git diff --check` PASS

Focused proof points now covered by tests:

- `tests_multipath_schedule::demand` checks
  `demand_planned_active_lane_count + demand_unmet_lane_count ==
  demand_requested_active_lane_count`.
- `tests_multipath_schedule::demand` checks
  `lane_admission_admitted_active_lane_count +
  lane_admission_rejected_active_lane_count ==
  lane_admission_requested_active_lane_count`.
- `tests_multipath_schedule::demand` checks admitted transit lane capacity stays
  within `100 - local_traffic_reserve_pct`.
- `tests_multipath_schedule::demand` checks route-explain demand fields match
  scheduler state.
- `tests_dps_runtime_flow::{failover,reselection}` check demand survives
  schedule replacement on replan paths.
- `tests_policy_parsers` reject raw numeric/throughput-like demand values such
  as `mesh_multipath_demand=100mbps`.
- redaction tests check payload sentinels, raw peer ids, endpoints, and route
  binding ids do not appear in public schedule diagnostics/debug output.

## FINAL_AUDIT

Status: done

The main source-level gates pass. Initial sub-agent audit found tester gaps;
source tests were strengthened. Sub-agent re-audit result:

- Architect: approved.
- Senior Rust: approved.
- Tester: approved for `Source-level multipath demand planning PASS`.
- Security: approved.
- DevOps: approved to proceed to GitHub Release/Latest and remote stand proof,
  but not installed release/update PASS yet.
- Critic: approved only the limited source-level claim.

## REPORT

Status: partial

Done:

- Source-level demand-aware multipath lane planning is implemented and tested.
- Diagnostics expose only aggregate demand planning fields.
- Anti-monolith guard regression was found and fixed.

Not done / not verified:

- No new GitHub Release/Latest has been published for this source slice yet.
- Side B/SIDE_A one-command update proof for this slice is not done yet.
- Live carrier traffic, real transit forwarding, TUN/OS routing, DNS binding,
  rollback, browser/IDE workflow, throughput, and long-run behavior remain not
  verified for this slice.
- Telemetry freshness, rebuild debounce/storm control, and live recovered-lane
  no-flap behavior are not implemented or verified by this source slice.
- Local+transit no-starvation is verified only as a source-level budget math
  invariant; live mixed traffic behavior is not verified.

Truth-first status:

- Source-level planner PASS.
- Real-World datapath PASS: not verified.
