# ADR: Mesh CLI `connect_recovery_needed` Recovery Flag (2026-05-21)

## Status
Accepted

## Context
`mesh route-explain` already exposes:
- `auto_recovery_attempts`
- `auto_recovery_final_result`
- `connect_retry_budget_exhausted`

But automation still had to infer "is reconnect action required now?" indirectly
from operator action/reason text.

## Decision
Add first-class success-envelope field:
- `connect_recovery_needed` (`"true" | "false"`)
- `connect_recovery_strategy` (`"none" | "retry_connect_endpoints"`)
- `connect_recovery_projection_consistency` (`"true" | "false"`)
- `connect_recovery_projection_key` (`needed:<bool>;strategy:<strategy>;action:<action>`)

Semantics:
- `true` when operator action is `retry_connect_endpoints`
- `false` otherwise

Also bump recovery schema:
- `route_explain_recovery_schema_version=mesh_recovery_v5`
- `route_explain_recovery_fields_checksum=auto_recovery_attempts|auto_recovery_final_result|connect_retry_budget_exhausted|connect_recovery_needed|connect_recovery_strategy|connect_recovery_projection_consistency|connect_recovery_projection_key`

Error envelope stays unchanged: recovery projection fields remain success-only.

## Consequences
Positive:
- Stable machine-readable signal for reconnection workflows.
- Less brittle downstream parsing of summary/action text.

Trade-off:
- Consumer contract update required due to schema bump `v1 -> v2`.

## Evidence
- `cargo test -q -p chimera-cli tests_json_success_presence`
- `cargo test -q -p chimera-cli tests_json_operator_cross_contract`
- `just truth-contract-check`
- `just ship-report-contract-check`
