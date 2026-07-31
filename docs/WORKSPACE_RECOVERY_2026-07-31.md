# Workspace Recovery 2026-07-31

Status: partial recovery proof.

## Restored

- Active source worktree: `chimera-pq`.
- Git remote and `main` are aligned.
- Release tag history is present.
- Parent workspace control files were reconstructed as working copies.
- Repo-level `AGENTS.md` now preserves the essential recovered rules inside the
  repository, so a fresh clone still carries the session contract.

## Truth Boundary

- The recovered parent workspace files are not claimed to be byte-for-byte
  copies of the lost originals.
- Source code tracked by git is the authoritative project source.
- Backup directories and recovery bundles are evidence archives, not the active
  worktree.
- Live three-node mesh proof after the recovery is still a separate required
  step.

## Recovery Evidence To Keep

- `git status --short --branch`
- `git rev-list --left-right --count HEAD...origin/main`
- `git verify-commit -v HEAD`
- `git verify-tag -v <latest-release-tag>`
- `just session-process-guard`
- `just handoff-process-check`
- published release asset checks for `chimera.sh`, release archive, and checksum

## Public Artifact Rule

Historical proof and handoff files are public artifacts. They must not contain
raw stand addresses, SSH logins, local paths, tokens, keys, payload bytes, or
unredacted diagnostic targets.

Use:

```bash
just public-artifact-redaction-guard
```

