# Workflow Attestation: Real-World Release/Update Gate

Status: in_progress
Date: 2026-06-17

## Objective

Publish a new GitHub Release/Latest for CHIMERA-PQ and verify the documented
one-command install/update path over SSH on:

- laptop: `art@192.168.31.21`
- VPS: `root@91.124.19.180:22`

The stand install/update command must use GitHub Release/Latest:

```bash
curl --disable -fsSL https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh | bash -s -- -install
```

## Required Cycle

Required workflow:

```text
ANALYSIS -> PLAN -> TEAM_CRITIQUE -> IMPLEMENTATION -> TEAM_CHECK -> FIX
-> RECHECK -> FINAL_AUDIT -> REPORT
```

## Analysis

- MVP scope is the WEAVE symmetric mesh-node model from `CHIMERA-PQ_MVP_SPEC.md`.
- Real-World PASS cannot be claimed from source/lab artifacts alone.
- Current GitHub Latest before this cycle points to `v0.1.98`.
- Candidate next release tag: `v0.1.99`, only if source gates pass and the tag
  is still free at publish time.
- Local PC is source/control only. CHIMERA runtime proof must be remote-only.

## Team Critique Inputs

Real sub-agent roles were assigned for this cycle:

- architect
- senior developer
- tester
- security engineer
- DevOps engineer
- critic-skeptic

Known blocker themes from the council before this attestation:

- untracked Rust files must be committed before release;
- public diagnostics must not leak raw peer ids, endpoints, ports, tokens or
  payload details;
- GitHub Latest assets must be verified before stand update;
- SSH stand proof must be collected on both laptop and VPS;
- source/lab PASS is not Real-World PASS.

## Implementation Plan

1. Run source and release contract gates without local CHIMERA runtime launch.
2. Commit the coherent source state, including untracked Rust modules.
3. Push `main`.
4. Create and push `v0.1.99`.
5. Verify GitHub Actions release job and `releases/latest` assets.
6. Run one-command GitHub install/update on laptop and VPS over SSH.
7. Collect version, checksum, status/start/stop/rollback/log-redaction evidence.
8. Run final sub-agent audit before any PASS claim.

## Current Evidence

- `gh auth status`: local GitHub CLI is not logged in.
- GitHub API latest before release: `v0.1.98`.
- `git ls-remote https://github.com/neo-2022/chimera-pq.git refs/tags/v0.1.99`:
  no output, tag absent at the time of checking.
- Laptop SSH preflight: reachable, has `curl`, `tar`, `sha256sum`.
- VPS SSH preflight: reachable, has `curl`, `tar`, `sha256sum`.
- SSH to GitHub on default port 22 timed out in this environment.
- SSH authentication to GitHub over `ssh.github.com:443` succeeded.

## Not Closed Yet

- New commit is not pushed yet.
- New tag/release is not published yet.
- GitHub Latest does not yet point to the new source.
- Laptop/VPS one-command update proof is not collected yet.
- Real-World PASS is not claimed by this document.
