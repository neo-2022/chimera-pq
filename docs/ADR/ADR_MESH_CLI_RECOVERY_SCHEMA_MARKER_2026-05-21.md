# ADR: Mesh CLI Recovery Schema Marker and Field-Set Checksum

Date: 2026-05-21
Status: Superseded by `ADR_MESH_CLI_CONNECT_RECOVERY_NEEDED_FLAG_2026-05-21.md`
Scope: CHIMERA-PQ MVP (`chimera-cli mesh route-explain`)

## Context

Recovery first-class fields were added, but consumers still needed an explicit
marker to detect schema compatibility safely.

## Decision

Initially added two success-envelope fields:

- `route_explain_recovery_schema_version=mesh_recovery_v1`
- `route_explain_recovery_fields_checksum=auto_recovery_attempts|auto_recovery_final_result|connect_retry_budget_exhausted`

Then evolved to current contract (v5):

- `route_explain_recovery_schema_version=mesh_recovery_v5`
- `route_explain_recovery_fields_checksum=auto_recovery_attempts|auto_recovery_final_result|connect_retry_budget_exhausted|connect_recovery_needed|connect_recovery_strategy|connect_recovery_projection_consistency|connect_recovery_projection_key`

Recovery fields remain success-path only and absent in error envelope.

## Evidence

Code:
- `crates/chimera-cli/src/mesh_cli/route_explain_meta.rs`
- `crates/chimera-cli/src/mesh_cli/route_explain_recovery.rs`
- `crates/chimera-cli/src/mesh_cli/route_explain_fields.rs`
- `crates/chimera-cli/src/mesh_cli/route_explain_json.rs`
- `crates/chimera-cli/src/mesh_cli/tests_json_success_presence.rs`
- `crates/chimera-cli/src/mesh_cli/tests_json_operator_cross_contract.rs`

Validation (current):
- `cargo test -q -p chimera-cli tests_json_operator_cross_contract` PASS
- `cargo test -q -p chimera-cli` PASS
- `cargo clippy -q -p chimera-cli --all-targets -- -D warnings` PASS
- `bash scripts/anti_monolith_guard.sh` PASS
- `cargo run -q -p chimera-lab --bin mesh_cli_recovery_schema_guard -- docs/MESH_ROUTE_EXPLAIN.json` PASS

## Consequences

- Positive:
  - consumers can pin to explicit recovery schema/version pair.
  - safer evolution path for future recovery-field changes with explicit version bump.
- Trade-off:
  - checksum is deterministic contract text and must be updated intentionally with each schema evolution.

## Next Step

Keep schema-marker checks in downstream integration scripts and ship-readiness
pipeline for automatic compatibility gating.
