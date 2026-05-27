# ADR: Mesh Ship-Pipeline Contract Sync (2026-05-21)

## Status
Accepted

## Context
After mesh route-explain contract evolution (recovery schema fields + cooldown
selection semantics), several ship/readiness guards drifted from actual
artifacts. This produced false negative failures in end-to-end readiness:

- outdated expected `cooldown_selected_peer` in ship/report checks;
- stale CEF/ship structure assumptions after adding new mesh recovery guard
  steps;
- guard ordering race where sync validation consumed stale
  `SHIP_READINESS_REPORT.json`.

## Decision
Keep ship-pipeline contracts strictly synchronized with mesh runtime/CLI truth:

1. Align all mesh route-explain artifact checks to current factual contract.
2. Treat ship `steps` schema as versioned guard contract and update expected set
   together with new pipeline steps.
3. Run reality/ship sync guard only after writing fresh ship readiness report.
4. Keep CEF and ship checks compatible with anti-monolith test layout
   (`src/tests/` split).

## Consequences

Positive:
- `just ship-readiness` and `just ship-report-contract-check` reflect real
  regressions instead of contract drift noise.
- Mesh recovery schema guard is now first-class in readiness structure.
- Truth-first reporting remains stable across evolving mesh contracts.

Trade-offs:
- Any future mesh artifact field change must update multiple guard points
  (by design, for explicit contract governance).

## Evidence
- `just ship-report-contract-check`
- `just ship-readiness`
- `just truth-contract-check`
