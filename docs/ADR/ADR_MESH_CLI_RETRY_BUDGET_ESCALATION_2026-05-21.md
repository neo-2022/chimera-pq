# ADR: Mesh CLI Retry-Budget Escalation from Runtime Explain

Date: 2026-05-21
Status: Accepted
Scope: CHIMERA-PQ MVP (`chimera-cli mesh route-explain`)

## Context

Route-explain already emitted `retry_connect_endpoints` in one warn scenario, but
did not consume explicit runtime recovery outcome. We needed action escalation
based on actual retry-budget exhaustion signals.

## Decision

Wire runtime recovery signals into CLI operator action selection:

- Parse from explain:
  - `auto_recovery_attempts`
  - `auto_recovery_final_result`
- Derive `connect_retry_budget_exhausted=true` when attempts > 0 and final
  result is one of:
  - `no_candidate_after_relax_health`
  - `no_candidate_after_relax`
  - `no_candidate_after_last_chance_health`
- In warn + no-selected context, force:
  - `route_explain_operator_action=retry_connect_endpoints`
  - `route_explain_operator_reason=connect_plan_exhausted_or_unreachable`

## Evidence

Code:
- `crates/chimera-cli/src/mesh_cli/route_explain_fields.rs`
- `crates/chimera-cli/src/mesh_cli/route_explain_health.rs`

Tests:
- new unit: `operator_summary_retries_connect_when_retry_budget_exhausted`

Validation:
- `cargo test -q -p chimera-cli route_explain_health` PASS
- `cargo test -q -p chimera-cli` PASS
- `cargo clippy -q -p chimera-cli --all-targets -- -D warnings` PASS
- `bash scripts/anti_monolith_guard.sh` PASS

## Consequences

- Positive:
  - operator action now reflects actual runtime recovery outcome.
  - less ambiguous behavior in degraded/no-selected states.
- Trade-off:
  - action policy depends on exact `auto_recovery_final_result` string contract.

## Next Step

Export retry budget counters directly as first-class JSON fields (not only via
`explain`) to make automation simpler and less parser-dependent.

