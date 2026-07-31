# CHIMERA Repo Session Rules

Status: repo-level recovery copy created on 2026-07-31 after workspace-root
control files were lost once.

Use this file when the parent workspace `AGENTS.md` is missing or when a fresh
clone starts directly in this repository. If the parent workspace file exists,
read it first; the stricter rule wins.

## Startup

1. Identify the active `chimera-pq` worktree before editing or proving anything.
2. Read `README.md`, `docs/EXECUTION_MODE_NO_TIMELINES.md`,
   `docs/AI_ARCHITECT_LIFECYCLE_GUARD.md`,
   `docs/AI_ARCHITECT_ALGORITHM_COVERAGE.json`, and the newest
   `docs/MESH_SESSION_HANDOFF_*.md`.
3. Run safe local guards before broad work:
   - `just session-process-guard`
   - `just handoff-process-check`

## Hard Rules

- Do not claim `done`, `pass`, or `готово` without direct evidence for the
  exact requested outcome.
- Lab proof is not live stand proof.
- Release/install proof must use the published release path only: no `scp`,
  `rsync`, local tarball copy, target `cargo`, or target `git clone`.
- Runtime/network proof must run on authorized SSH stand nodes, not by mutating
  the controlling PC.
- Do not change local routes, DNS, firewall, proxy, VPN, or desktop networking
  on the controlling PC unless the user explicitly orders it.
- Do not expose or commit stand addresses, hostnames, usernames, local machine
  paths, ports, tokens, passwords, keys, or payload bytes.
- Do not hardcode stand-specific values into product logic or defaults.
- CHIMERA is a symmetric WEAVE mesh node. Do not describe the product as a VPN
  or proxy normal path.

## Serious Work

For non-trivial CHIMERA work, follow the AI Architect lifecycle in
`docs/AI_ARCHITECT_LIFECYCLE_GUARD.md` and the tactical cycle:

```text
ANALYSIS -> PLAN -> TEAM_CRITIQUE -> IMPLEMENTATION -> TEAM_CHECK -> FIX
-> RECHECK -> FINAL_AUDIT -> REPORT
```

When real subagents are available, use them for independent critique/checks.
Their review must include interdisciplinary checks from
`docs/AI_ARCHITECT_ALGORITHM_COVERAGE.json`; a software-only opinion is not
enough for a serious pass.

## Closeout

Before final status, run the relevant truth gates and report:

- Status
- Evidence
- What is not covered
- Risks or blocker

