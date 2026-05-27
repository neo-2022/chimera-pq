# ADR: Mesh CLI First-Class Recovery JSON Fields

Date: 2026-05-21
Status: Accepted
Scope: CHIMERA-PQ MVP (`chimera-cli mesh route-explain`)

## Context

Retry-budget and auto-recovery status previously lived only in the long `explain`
string, making machine parsing less robust for automation.

## Decision

Expose recovery state as first-class JSON fields in success envelope:

- `auto_recovery_attempts`
- `auto_recovery_final_result`
- `connect_retry_budget_exhausted`

Also apply anti-monolith split by extracting recovery parsing/projection logic
from `route_explain_fields.rs` into a dedicated module.

## Evidence

Code:
- `crates/chimera-cli/src/mesh_cli/route_explain_json.rs`
- `crates/chimera-cli/src/mesh_cli/route_explain_fields.rs`
- `crates/chimera-cli/src/mesh_cli/route_explain_recovery.rs`
- `crates/chimera-cli/src/mesh_cli/mod.rs`
- `crates/chimera-cli/src/mesh_cli/tests_json_success_presence.rs`

Validation:
- `cargo test -q -p chimera-cli` PASS
- `cargo clippy -q -p chimera-cli --all-targets -- -D warnings` PASS
- `bash scripts/anti_monolith_guard.sh` PASS

## Consequences

- Positive:
  - stable machine-readable recovery telemetry without parsing free-form explain.
  - cleaner domain split and smaller field-construction module.
- Trade-off:
  - error envelope intentionally does not carry these success-path fields.

## Next Step

Use first-class recovery fields in cross-envelope contract tests and in future
operator action escalations without relying on `explain` parsing.

