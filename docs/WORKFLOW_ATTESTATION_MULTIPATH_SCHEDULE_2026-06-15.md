# Workflow Attestation: Multipath Schedule Model

Scope: add a planner/model-only WEAVE multipath schedule contract in
`chimera-mesh` without changing carrier/runtime datapath, installer, release
scripts, local CHIMERA runtime, or host network settings.

Stages:

- ANALYSIS: done
  - Evidence: `AGENTS.md`, `CHIMERA-PQ_MVP_SPEC.md`, `Agent.md`,
    `docs/EXECUTION_MODE_NO_TIMELINES.md`, `README.md`, and latest handoff were
    read.
  - Result: the task is MVP scope because WEAVE requires a symmetric mesh node
    that can accept local traffic, accept peer traffic, egress traffic, and
    transit sealed third-party traffic. Current gap is planner/model schedule
    structure, not a proven carrier-bound multipath transfer.
- PLAN: done
  - Result: add typed schedule/lane structs to `model.rs`, build the schedule
    in a dedicated `runtime/multipath_schedule.rs` module, attach privacy-safe
    explain lines, add focused unit tests, and run source-only checks.
- TEAM_CRITIQUE: done
  - Architect: agreed with typed schedule after peer selection; no carrier hack.
  - Senior developer: agreed with separate runtime module and tests; warned not
    to change carrier datapath in this slice.
  - Tester: agreed; require mode, weights, local reserve and privacy tests.
  - Security engineer: agreed only if explain avoids payload, destination and
    raw endpoint leakage and remains sealed-transit safe.
  - DevOps: agreed; planner/model-only change does not require GitHub release
    or laptop/VPS runtime proof before source checks.
  - Critic: agreed only if reported as planner/model progress, not real
    multipath transfer or MVP/pass.
- IMPLEMENTATION: done
  - Evidence: `crates/chimera-mesh/src/model.rs`,
    `crates/chimera-mesh/src/runtime/multipath_schedule.rs`,
    `crates/chimera-mesh/src/runtime/path_planner.rs`,
    `crates/chimera-mesh/src/runtime/plan_ops_dps_eval.rs`,
    `crates/chimera-mesh/src/runtime/plan_ops.rs`,
    `crates/chimera-mesh/src/runtime/plan_dps_adaptation.rs`,
    `crates/chimera-mesh/src/tests_multipath_schedule/mod.rs`,
    `crates/chimera-mesh/src/tests_dps_runtime_flow/failover.rs`,
    `crates/chimera-mesh/src/tests_dps_runtime_flow/reselection.rs`.
  - Result: `MeshPathPlan` now carries a typed
    `MeshMultipathSchedule`. DPS `mesh_multipath_mode` updates schedule mode
    for plan, failover and health-reselection entrypoints. Explain output is
    marked `planner_only_not_carrier_bound` and does not include lane endpoints.
    `MeshMultipathLane` debug output redacts peer identity.
- TEAM_CHECK: done
  - Architect: blocker found and fixed; no remaining architecture blocker for
    planner/model-only scope.
  - Senior developer: no remaining code/API/test blocker found.
  - Tester: planner/model-only coverage is sufficient after regression tests
    for failover and health-reselection DPS paths.
  - Security engineer: no new privacy/security blocker; new schedule explain
    does not expose payload, destination, peer id or endpoint.
  - DevOps: no release or laptop/VPS stand proof required for this source-only
    planner/model change.
  - Critic: accepted only as planner/model progress; not accepted as real
    carrier-bound multipath or MVP completion.
- FIX: done
  - Issue found: `aggregate_buffered` could not form three selected lanes in one
    region because DPS adaptation raised per-region selection capacity only to
    two.
  - Fix: `aggregate_buffered` now raises `max_selected_per_region` to three
    unless the payload explicitly overrides it.
  - Issue found: DPS failover/reselection entrypoints would keep the default
    `off` schedule even when the payload carried `mesh_multipath_mode`.
  - Fix: all DPS planning entrypoints now apply the same multipath schedule
    replacement helper.
  - Issue found: the new lane type could reveal `peer_node_id` through derived
    debug formatting.
  - Fix: `MeshMultipathLane` now has a manual debug implementation that redacts
    `peer_node_id`, with a regression test.
  - Issue found during architect audit: DPS failover/reselection applied
    `mesh_multipath_mode` to the schedule after peer selection, but did not
    apply traffic-hint policy adaptation before peer selection.
  - Fix: all DPS planning entrypoints now use the same
    policy-with-traffic-hints helper before selecting peers. Regression tests
    cover `flow_shard` and `aggregate_buffered` without explicit peer limits in
    failover and health-reselection paths.
- RECHECK: done
  - `cargo fmt --all -- --check`: PASS.
  - `cargo test -q -p chimera-mesh multipath_schedule`: PASS.
  - `cargo test -q -p chimera-mesh tests_dps_policy`: PASS.
  - `cargo test -q -p chimera-mesh tests_dps_runtime_flow`: PASS.
  - `cargo test -q -p chimera-mesh`: PASS, 159 tests.
  - `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`: PASS.
  - `cargo check -q --workspace`: PASS.
  - `cargo test -q -p chimera-session sealed_transit`: PASS.
  - `cargo test -q -p chimera-carrier transit`: PASS.
  - `bash scripts/anti_monolith_guard.sh`: PASS.
- FINAL_AUDIT: done
  - No carrier/runtime datapath execution was added.
  - No local CHIMERA runtime start/stop or host network mutation was performed.
  - Remaining limitation: real carrier-bound multipath transfer still needs a
    later runtime implementation and laptop/VPS stand proof through GitHub
    one-command release/update flow.
- REPORT: done

Runtime/network statement:

- Local CHIMERA runtime start/stop is not part of this change.
- Local DNS, routes, firewall, proxy, Happ, MYVPN, VPN, router, laptop and VPS
  settings are out of scope for this source-only planner/model update.
