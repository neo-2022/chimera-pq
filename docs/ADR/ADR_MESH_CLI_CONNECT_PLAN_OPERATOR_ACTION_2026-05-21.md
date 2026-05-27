# ADR: Mesh CLI Operator Action for Connect-Plan Exhaustion

Date: 2026-05-21
Status: Accepted
Scope: CHIMERA-PQ MVP (`chimera-cli mesh route-explain`)

## Context

`route-explain` previously emitted generic warn action/reason when health gate was
warn and no peer was selected. This was less practical for real operator flow.

## Decision

When all of the following are true:
- `route_explain_health_gate != ok`
- no selected peer (`selected:none`)
- pressure action hint is `wait_or_recover_health`

then emit operator guidance:
- `route_explain_operator_action=retry_connect_endpoints`
- `route_explain_operator_reason=connect_plan_exhausted_or_unreachable`

Otherwise keep previous behavior.

## Evidence

Code:
- `crates/chimera-cli/src/mesh_cli/route_explain_health.rs`

Tests:
- added unit test `operator_summary_retries_connect_when_warn_and_no_selected_peer`

Validation:
- `cargo test -q -p chimera-cli route_explain_health` PASS
- `cargo test -q -p chimera-cli` PASS
- `cargo clippy -q -p chimera-cli --all-targets -- -D warnings` PASS
- `bash scripts/anti_monolith_guard.sh` PASS

## Consequences

- Positive:
  - route-explain gives concrete next action for endpoint reconnection loop.
  - less ambiguous diagnostics under degraded/no-selected state.
- Trade-off:
  - introduces one more explicit operator-action branch to maintain.

## Next Step

Expose retry-attempt counters from runtime into CLI envelope so action can switch
from "retry" to "check connectivity" after explicit budget exhaustion.

