# ADR: Mesh CLI Recovery Cross-Envelope Contract Invariants

Date: 2026-05-21
Status: Accepted
Scope: CHIMERA-PQ MVP (`chimera-cli mesh route-explain` tests/contracts)

## Context

After adding first-class recovery fields to success JSON, we needed explicit
cross-envelope guarantees to avoid accidental drift between success/error
contracts.

## Decision

Enforce in cross-envelope tests:

- Success envelope:
  - `auto_recovery_attempts` is present and numeric string.
  - `auto_recovery_final_result` is present and non-empty.
  - `connect_retry_budget_exhausted` is present and `true|false`.
- Error envelope:
  - these recovery fields are absent (success-path only).

## Evidence

Code:
- `crates/chimera-cli/src/mesh_cli/tests_json_operator_cross_contract.rs`

Validation:
- `cargo test -q -p chimera-cli tests_json_operator_cross_contract` PASS
- `cargo test -q -p chimera-cli` PASS
- `cargo clippy -q -p chimera-cli --all-targets -- -D warnings` PASS
- `bash scripts/anti_monolith_guard.sh` PASS

## Consequences

- Positive:
  - deterministic envelope boundary for automation consumers.
  - lower risk of silent schema mixing between success and error paths.
- Trade-off:
  - stricter tests may require coordinated updates for future schema evolution.

## Next Step

Add a compact schema version/checksum marker for success recovery fields to
detect incompatible consumer expectations early.

