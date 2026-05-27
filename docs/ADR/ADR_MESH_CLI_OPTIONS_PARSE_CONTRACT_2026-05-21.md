# ADR: Mesh CLI `options_parse` Contract Hardening

Date: 2026-05-21
Status: Accepted
Scope: CHIMERA-PQ MVP (`chimera-cli mesh route-explain`)

## Context

`mesh route-explain` is the operator/debug entrypoint for mesh planning in MVP.
When input is malformed, weak parser behavior causes ambiguous diagnostics and
breaks automation that expects stable JSON error envelopes.

## Decision

Harden and freeze parser behavior for `options_parse` stage:

1. Unknown/positional/duplicate/singleton errors must be deterministic.
2. Identity fallback in JSON error envelope must be safe:
   - unresolved `namespace` -> `unknown`;
   - unresolved `node` -> `unknown`.
3. `--json` is idempotent and only exact `--json` is accepted as JSON mode.
4. Helper extraction must not treat another flag token as value.
5. Error envelopes must preserve stable contract fields:
   `stage/action/category/retriable/backoff/resolution/signature/route_key`.

## Implemented Evidence

Code and tests:
- `crates/chimera-cli/src/mesh_cli/options.rs`
- `crates/chimera-cli/src/mesh_cli/tests_json_error_options_parse*.rs`
- `crates/chimera-cli/src/mesh_cli/tests_options_parse*.rs`
- `crates/chimera-cli/src/mesh_cli/tests_options_helpers.rs`

Validation:
- `cargo test -q -p chimera-cli` PASS
- `cargo clippy -q -p chimera-cli --all-targets -- -D warnings` PASS
- `bash scripts/anti_monolith_guard.sh` PASS

## Consequences

- Positive:
  - deterministic operator-facing errors;
  - stable machine-readable JSON for automation;
  - lower risk of silent parser regressions.
- Trade-off:
  - broader test surface to maintain (intentional for contract stability).

## Next Practical Step

Move from parser hardening to runtime auto-connect behavior:
- endpoint candidate preference;
- retry/backoff orchestration;
- persisted last-good endpoint hints;
- explain fields proving why auto-connect selected/fallbacked.
