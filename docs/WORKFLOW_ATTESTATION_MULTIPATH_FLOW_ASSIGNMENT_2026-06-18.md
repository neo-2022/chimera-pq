# Workflow Attestation: Multipath Flow Assignment

Date: 2026-06-18

Scope: source-level WEAVE multipath flow assignment contract in
`chimera-mesh`.

This is a planner/control-plane contract. It is not live carrier traffic proof,
not TUN/OS routing proof, not DNS binding proof, and not Real-World datapath
PASS.

## ANALYSIS

Status: done

- MVP scope confirmed from `CHIMERA-PQ_MVP_SPEC.md`: CHIMERA-PQ is a symmetric
  WEAVE mesh-node where each node can accept local traffic, accept peer traffic,
  egress traffic and forward sealed transit traffic.
- Transit payload opacity remains mandatory: transit payload is closed third
  party information and must stay opaque/sealed to transit nodes.
- Existing code had multipath schedule, demand, lane admission and carrier
  lane binding contracts, but lacked a small source-level contract that maps an
  opaque flow key to an active carrier lane.

## PLAN

Status: done

- Add a pure Rust `chimera-mesh` flow assignment module.
- Use only `MeshMultipathSchedule` plus an opaque flow key/hash.
- Select only active carrier lane bindings with nonzero capacity.
- Fail closed when route binding is missing, carrier bindings are missing,
  duplicate lanes are present, capacity is missing, local reserve is invalid,
  route binding mismatches are present, active capacity exceeds transit budget,
  or transit payload policy is not `sealed_opaque_only`.
- Keep explain output aggregate and redacted.
- Do not modify carrier runtime or local PC network/runtime behavior in this
  slice.

## TEAM_CRITIQUE

Status: done

- Architect recommended a narrow source-level flow/rebuild contract and warned
  that Real-World datapath status must remain unverified until laptop/VPS proof.
- Senior Rust recommended a small `multipath_flow` module, stable flow-to-lane
  assignment, fail-closed behavior and no carrier runtime changes in the first
  slice.
- Tester required focused deterministic, spread, standby-exclusion, fail-closed,
  rebuild and redaction tests plus full source gates.
- Security required payload opacity, no route decision by payload contents and
  no peer/endpoint/route-id/payload leakage in public diagnostics.
- DevOps required local source checks only, then GitHub Release/Latest
  one-command laptop/VPS proof before installed proof.
- Critic blocked any claim that this source slice proves real transit,
  TUN/routing, DNS binding, browser workflow or throughput.

## IMPLEMENTATION

Status: done

Files changed:

- `crates/chimera-mesh/src/runtime/multipath_flow.rs`
- `crates/chimera-mesh/src/runtime.rs`
- `crates/chimera-mesh/src/lib.rs`
- `crates/chimera-mesh/src/tests_multipath_schedule/mod.rs`
- `crates/chimera-mesh/src/tests_multipath_schedule/flow_assignment.rs`

Implemented behavior:

- `MeshMultipathFlowKey` accepts an opaque flow id or a precomputed stable hash.
- `plan_multipath_flow()` maps the flow key to an active carrier lane using
  stable weighted selection over `capacity_weight_pct`.
- Assignment is deterministic for the same flow key.
- Different flow keys spread across multiple active lanes when the schedule has
  multiple active bindings.
- Standby lanes are never selected for flow assignment.
- A schedule without explicit route binding fails closed instead of falling back
  to a peer pool.
- A stale/malformed schedule where active carrier bindings do not match the
  schedule route binding fails closed.
- A stale/malformed schedule where active carrier binding capacity exceeds the
  transit budget fails closed.
- Flow explain lines expose only aggregate markers:
  - `multipath_flow_action`
  - `multipath_flow_reason`
  - `multipath_flow_selected_lane`
  - `multipath_flow_active_bindings`
  - `multipath_flow_total_capacity_weight_pct`
  - `multipath_flow_route_binding_configured`
  - `multipath_flow_rebuild_recommended`
  - `multipath_flow_rebuild_reason`
  - `multipath_flow_fairness_policy`
  - `multipath_flow_privacy`

## TEAM_CHECK

Status: done

Initial final sub-agent audit found two real security blockers:

- active carrier bindings were not checked against the schedule route binding;
- active carrier binding capacity over the transit budget could still assign a
  lane and only mark rebuild pressure.

DevOps also blocked release while new source files were still untracked.

Critic re-audit found two additional source acceptance blockers:

- weighted selection still had a silent `first binding` fallback;
- several fail-closed reasons lacked direct tests.

## FIX

Status: done

- `plan_multipath_flow()` now fail-closes on `route_binding_mismatch`.
- `plan_multipath_flow()` now fail-closes on
  `active_binding_capacity_over_budget`.
- Removed silent first-binding fallback from weighted selection; if no weighted
  binding is selected, the flow plan fails closed with
  `weighted_selection_no_match`.
- Added negative tests for both malformed schedule paths.
- Added a separate fail-closed matrix for:
  - `transit_payload_policy_not_opaque`;
  - `local_reserve_invalid`;
  - `active_binding_missing`;
  - `duplicate_active_lane`;
  - `active_binding_capacity_missing`.
- DevOps release blocker remains open until commit/tag/release steps include
  all new files.

## RECHECK

Status: done

Commands run from `/home/art/Archives/VPN/chimera-pq`:

- `cargo fmt --all -- --check` PASS
- `cargo check -q --workspace` PASS
- `cargo test -q -p chimera-mesh tests_multipath_schedule::flow_assignment` PASS
  - 10 tests passed
- `cargo test -q -p chimera-mesh tests_multipath_schedule::flow_fail_closed`
  PASS
  - 5 tests passed
- `cargo test -q -p chimera-mesh tests_multipath_schedule` PASS
  - 32 tests passed
- `cargo test -q -p chimera-mesh multipath_flow` PASS
  - 1 focused module-level test match passed
- `cargo test -q -p chimera-mesh` PASS
  - 206 tests passed
- `cargo test -q --workspace` PASS
- `cargo clippy -q --workspace --all-targets -- -D warnings` PASS
- `bash scripts/anti_monolith_guard.sh` PASS
- `just rust-no-hardcode-guard` PASS
- `bash scripts/chimera_installer_gate.sh` PASS
- `bash scripts/chimera_update_contract_smoke.sh` PASS
- `bash scripts/chimera_start_contract_smoke.sh` PASS
- `bash scripts/chimera_stop_contract_smoke.sh` PASS
- `git diff --check` PASS

Anti-monolith status:

- `crates/chimera-mesh/src/runtime/multipath_flow.rs`: 298 lines.
- `crates/chimera-mesh/src/runtime/multipath_flow.rs`: 321 lines after
  security fail-closed fix.
- `crates/chimera-mesh/src/runtime/multipath_flow.rs`: 334 lines after removing
  weighted fallback.
- `crates/chimera-mesh/src/tests_multipath_schedule/flow_assignment.rs`: 280
  lines.
- `crates/chimera-mesh/src/tests_multipath_schedule/flow_fail_closed.rs`: 86
  lines.
- `crates/chimera-mesh/src/runtime.rs`: 384 lines, guard PASS.
- `crates/chimera-mesh/src/tests_multipath_schedule/mod.rs`: 393 lines, guard
  PASS.

## FINAL_AUDIT

Status: pending re-audit

Sub-agent final audit requested from:

- architect;
- senior Rust;
- tester;
- security;
- DevOps;
- critic-skeptic.

Re-audit requested after closing the security and critic blockers.

## REPORT

Status: partial

Closed:

- Source-level multipath flow assignment contract.
- Deterministic same-flow assignment.
- Multi-flow spread across active lanes.
- Standby lane exclusion.
- Missing route-binding fail-closed behavior.
- Route-binding mismatch fail-closed behavior.
- Active capacity over transit budget fail-closed behavior.
- Source-level rebuild recommendation marker for planner pressure.
- Redacted aggregate explain/debug behavior for the new flow plan.
- Source gates and guard checks listed above.

Not closed:

- GitHub Release/Latest for this slice.
- Laptop/VPS one-command update proof for this slice.
- Live carrier traffic between laptop and VPS.
- Real sealed transit forwarding of third-party traffic.
- Transparent TUN/OS routing.
- DNS-to-route runtime binding.
- Crash/forced-stop rollback on stand.
- Browser/IDE normal workflow without proxy/manual workaround.
- Real multipath throughput, fairness and long-run behavior.

Truth-first status:

- Source-level flow assignment contract: PASS after local source gates.
- Installed release/update proof: not done for this slice.
- Real-World datapath PASS: not verified.
