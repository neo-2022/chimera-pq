# ADR 0002: Full CEF Track Gates

## Status

Accepted (2026-05-18)

## Context

`CHIMERA.pdf` defines a broader CEF scope than current MVP/Lab contour.
Current repository has strong MVP verification gates, but Full CEF closure
needs separate measurable gates to avoid mixing statuses.

## Decision

Introduce a separate Full CEF track with independent gates and artifacts:

1. Keep existing MVP/Lab gates unchanged.
2. Add machine-readable CEF track snapshot artifacts:
   - `docs/CEF_TRACK_REPORT.json`
   - `docs/CEF_TRACK_REPORT.md`
3. Require explicit truth boundary in CEF track:
   - `mvp_lab_ready=true`
   - `full_cef_closed=false` until all CEF blocks are implemented and verified.
4. Define per-block closure criteria in dedicated gate document:
   - `docs/CEF_PHASE1_GATES.md`

## Consequences

- Prevents false statements where MVP PASS is interpreted as Full CEF closure.
- Enables incremental Full CEF execution with testable criteria per block.
- Adds reporting overhead, but improves correctness and auditability.
