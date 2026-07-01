# Workflow Attestation: AI Architect Guard Hardening

## Scope

- Workline: `ai-architect-guard-hardening-2026-07-01`.
- Purpose: close the bypass where an old canonical workflow attestation could be
  reused as proof for a new CHIMERA workline.
- Runtime boundary: local CHIMERA runtime was not started; network state was not
  modified.

## Council Findings

- Architect: existing lifecycle guard blocks ordinary stage skipping, but it
  needs a current-workline binding.
- Senior developer: Rust guard checks structure and fields, but JSON alone does
  not prove physical sub-agent execution without a separate trace.
- Tester: current-workline guard must have positive and negative checks.
- Security engineer: no secret leak was found in the checked process artifacts,
  but fake sub-agent records and template-like text remain risks unless traced.
- DevOps reviewer: `session-process-guard` must invoke the current-workline
  guard, not only the canonical workflow guard.
- Critic-skeptic: stale `WORKFLOW_ATTESTATION.json` reuse is a real bypass and
  must be blocked.

## Accepted Fixes

- Add `current_workline_attestation_guard`.
- Require `docs/CURRENT_WORKLINE_ATTESTATION.json`.
- Require `subagent_execution_log` for current workline pass.
- Require per-role prompt/output summary hashes.
- Require current handoff binding to the newest handoff.
- Wire the current-workline guard into `just session-process-guard`.
- Wire the current-workline guard into `handoff-check` and
  `ship_readiness.sh`, so old canonical proof cannot pass those paths alone.
- Require explicit current-workline `stage_trace` with the eight lifecycle
  stages in order.
- Require `workline_attestation_sha256`, binding the current JSON proof to the
  exact workline report file content.
- Add negative checks for old canonical kind mismatch and a fully missing
  `subagent_execution_log` field.
- Require a separate `SUBAGENT_EXECUTION_TRACE` artifact and verify its
  SHA-256 digest before accepting current workline proof.
- Require `latest_handoff_sha256`, so the current proof is bound to the exact
  handoff content as well as the newest handoff path.
- Require per-stage current-workline reports with gate decision, council review
  and red-team review flags for all eight lifecycle stages.
- Require per-role interdisciplinary checks, not only a global
  `interdisciplinary_trace=true` style flag.

## Boundaries

- This improves proof discipline and blocks the found practical bypass.
- It still cannot cryptographically prove what happened inside an external
  model service beyond the recorded tool ids, summaries and hashes.
