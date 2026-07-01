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

## Boundaries

- This improves proof discipline and blocks the found practical bypass.
- It still cannot cryptographically prove what happened inside an external
  model service beyond the recorded tool ids, summaries and hashes.
