# ADR: Mesh Auto-Connect Endpoint Plan Explain Contract

Date: 2026-05-21
Status: Accepted
Scope: CHIMERA-PQ MVP (`chimera-mesh` runtime path planning)

## Context

Operator asked for practical auto-connect behavior clarity: if internet exists and
peers are known, mesh should expose deterministic connection order and retry
intent instead of opaque "selected peers only" output.

## Decision

Add explicit auto-connect plan fields to runtime explain contract:

1. `selected_peer_connect_priority`:
   deterministic endpoint priority (`N:node@endpoint`) in current selection order.
2. `selected_peer_connect_retry_plan`:
   per-endpoint retry attempt shape (`try0|try1|try2`).
3. `selected_peer_connect_backoff_profile`:
   fixed MVP backoff contract (`initial=0ms;retry1=250ms;retry2=1000ms`).

Applied in both explain paths:
- main `path_planner_*` path;
- `auto_recovery/*` path.

## Evidence

Code:
- `crates/chimera-mesh/src/runtime/path_planner_selection_metrics_peer.rs`
- `crates/chimera-mesh/src/runtime/path_planner_selection_metrics.rs`
- `crates/chimera-mesh/src/runtime/path_planner_selection_metrics_build.rs`
- `crates/chimera-mesh/src/runtime/path_planner_selection_explain_sections.rs`
- `crates/chimera-mesh/src/runtime/auto_recovery/types.rs`
- `crates/chimera-mesh/src/runtime/auto_recovery/selection_metrics_peer.rs`
- `crates/chimera-mesh/src/runtime/auto_recovery/selection_explain_selected.rs`
- `crates/chimera-mesh/src/tests/runtime_planning.rs`

Validation:
- `cargo fmt --all` PASS
- `cargo test -q -p chimera-mesh` PASS
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings` PASS
- `bash scripts/anti_monolith_guard.sh` PASS

## Consequences

- Positive:
  - deterministic operator-level auto-connect intent;
  - easier debugging of why mesh did or did not connect;
  - stable explain keys for CLI/automation consumers.
- Trade-off:
  - backoff values are contract-level defaults in MVP and may require versioned
    update if adaptive logic is introduced.

## Next Step

Use this explain contract in CLI `route-explain` health/operator summary to emit
direct action hints when no peer becomes reachable after full retry plan.

