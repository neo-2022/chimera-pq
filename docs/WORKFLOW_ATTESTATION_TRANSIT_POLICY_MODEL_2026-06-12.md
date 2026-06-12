# Workflow Attestation: Transit Policy Model

Scope: align policy, CLI/lab route output and proof artifacts with WEAVE peer
transit wording. New product output must use `transit`; legacy `gateway` remains
only as accepted input for old policy files and as transitional binary/test
names.

Stages:

- ANALYSIS: done
  - Evidence: `AGENTS.md`, `CHIMERA-PQ_MVP_SPEC.md`, `Agent.md`,
    `docs/EXECUTION_MODE_NO_TIMELINES.md`, latest handoff and current `rg`
    audit were read.
  - Result: the task is MVP scope because WEAVE requires symmetric peer
    transit/forwarding, not a product-level gateway role.
- PLAN: done
  - Result: rename policy outbound model to `Transit`, keep parser
    compatibility for `gateway`, update route/datapath proof surfaces and run
    safe build/unit/static checks.
- TEAM_CRITIQUE: done
  - Architect: agreed; product model must not expose `gateway` as the route
    role.
  - Senior developer: agreed; avoid broad capture enum rename in this slice and
    keep transitional mapping contained.
  - Tester: agreed; require parser compatibility test plus route/datapath smoke
    proof output checks.
  - Security engineer: agreed; transit wording must reinforce opaque third-party
    payload handling.
  - DevOps: agreed; no local runtime/network mutation.
  - Critic: agreed; do not claim real-world WEAVE datapath completion from local
    compile/smoke artifacts.
- IMPLEMENTATION: done
  - Evidence: `crates/chimera-policy/src/lib.rs`,
    `crates/chimera-cli/src/main.rs`, `crates/chimera-lab/src/main.rs`,
    `crates/chimera-lab/src/artifact_checks.rs`,
    `scripts/runtime_policy_precedence_smoke.sh`,
    `scripts/runtime_datapath_multiflow_smoke.sh`,
    `scripts/chimera-autofix.sh`, `scripts/ship_readiness.sh`, `justfile`,
    `docs/datapath_latest.json`, `docs/route_explain_latest.json`,
    `docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json`.
  - Result: new generated/output policy path uses `transit`; `gateway` is kept
    as legacy policy token input only.
- TEAM_CHECK: done
  - Architect: no new product-level client/server/gateway split found in the
    changed output surface.
  - Senior developer: current blast radius is limited to policy/CLI/lab/scripts
    and proof artifacts.
  - Tester: targeted cargo checks passed for `chimera-policy`, `chimera-cli` and
    `chimera-lab` after the first cleanup.
  - Security engineer: no transit payload inspection/logging path was added.
  - DevOps: no local CHIMERA runtime start/stop and no host network mutation.
  - Critic: full remote laptop/VPS datapath proof remains not run in this slice.
- FIX: done
  - Issue found: smoke/readiness scripts and checked JSON artifacts still used
    `gateway_ok`, `gateway_explain`, `suffix-gateway` and `"outbound":"gateway"`.
  - Fix: updated them to `transit_ok`, `transit_explain`,
    `suffix-transit` and `"outbound":"transit"`.
- RECHECK: done
  - `cargo fmt --all -- --check`: PASS.
  - `cargo test -p chimera-policy --quiet`: PASS.
  - `cargo test -p chimera-mesh weave_contract --quiet`: PASS.
  - `cargo test -p chimera-session sealed_transit --quiet`: PASS.
  - `cargo test -p chimera-cli --quiet`: PASS.
  - `cargo test -p chimera-lab --quiet`: PASS.
  - `cargo check --workspace --all-targets --quiet`: PASS.
  - `git diff --check`: PASS.
  - `bash scripts/anti_monolith_guard.sh`: PASS.
  - `bash scripts/ship_structure_guard.sh`: PASS.
  - `cargo run -q -p chimera-lab --bin rust_no_hardcode_guard`: PASS.
  - `cargo clippy -p chimera-mesh --all-targets -- -D warnings`: PASS.
  - `bash scripts/runtime_policy_precedence_smoke.sh`: PASS and emitted
    `suffix-transit` / `"outbound":"transit"`.
  - `bash scripts/runtime_datapath_multiflow_smoke.sh`: PASS and emitted
    `transit_explain` / `transit_ok`.
  - `just runtime-policy-precedence-smoke-selfcheck`: PASS.
  - `just runtime-datapath-multiflow-smoke-selfcheck`: PASS.
- FINAL_AUDIT: done
  - `rg` audit found no product output references to `gateway_ok`,
    `gateway_explain`, `suffix-gateway`, `default-gateway`,
    `gateway_outbound_rules`, `OutboundMode::Gateway` or
    `"outbound":"gateway"` in the checked policy/CLI/lab/scripts/docs surface.
  - Remaining `gateway` references are accepted legacy/parser compatibility and
    transitional test naming:
    `parse_policy_text("legacy = default => gateway")` and
    `crates/chimera-lab/tests/fake_client_gateway.rs`.
  - No local runtime/network mutation was performed.
  - Remote laptop/VPS real-world WEAVE datapath proof was not run in this slice.
- REPORT: done
  - Final user report must state code/doc/proof-artifact status separately from
    remote real-world WEAVE datapath status.

Runtime/network statement:

- Local CHIMERA runtime start/stop is not part of this change.
- Local DNS, routes, firewall, proxy, Happ, MYVPN, VPN, router, and VPS settings
  are out of scope for this code/doc update.
