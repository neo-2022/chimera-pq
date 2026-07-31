# Research Debt

Status: active

This file records non-blocking research questions discovered during the
AI Architect lifecycle. It is not a place to hide blockers. Anything that blocks
MVP safety, correctness, release, rollback, secrecy, or real-runtime evidence
must remain a blocker in the workflow attestation.

## Current Items

- `2026-07-31`: GitHub, Gitvers, and peer mirrors still lack a signed shared
  release manifest that binds the same release tuple to source identity across
  the trust ladder. Current protection is checksum + source-order + same-origin
  metadata validation, which is strong enough for bounded fallback but not full
  cross-source provenance equivalence. Evidence: `docs/UPDATE_SOURCE_DECISION_MATRIX.md`,
  `docs/OPERATIONS.md`, `scripts/chimera-update.sh`.
