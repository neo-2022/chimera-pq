# Workflow Attestation: Git Tree Hygiene

## Scope

- Date: 2026-06-29
- Component: repository hygiene
- Runtime started: false
- Local network state changed: false
- Remote stand used: false

## What Was Corrected

- The accumulated dirty tree was treated as agent-owned work, not ignored.
- A backup patch was written before committing:
  `/tmp/chimera-pq-dirty-worktree.patch`.
- The accumulated changes were staged, whitespace-checked, scanned for
  forbidden stand identifiers and private key markers, validated, then committed
  locally.
- Git signing was configured but the signing key was unavailable in this
  environment, so the local cleanup commit was created with `--no-gpg-sign`.

## Commit

- `b87831a Improve mesh metadata control paths`

## Validation

PASS:

- `git diff --cached --check`
- `cargo fmt --all -- --check`
- `cargo check -q --workspace --all-targets`
- `cargo test -q --workspace --all-targets`
- `cargo clippy -q --workspace --all-targets -- -D warnings`
- `just rust-no-hardcode-guard-selfcheck`
- `just release-pack-schema-guard-selfcheck`
- `just ship-structure-guard-selfcheck`
- `./scripts/release_pack_schema_guard.sh`
- `./scripts/ship_structure_guard.sh`
- staged scan for stand IP/user/path/private-key markers returned no matches
- final product repository status was clean

## Policy Update

- The persistent working rule was added to the outer project `AGENTS.md`:
  `Git Tree Hygiene`.
- The outer project directory is not a git repository, so that rule is stored
  as a workspace rule file rather than a product commit.

## Guard Automation

- Added a read-only product guard:
  `scripts/git_tree_hygiene_guard.sh`.
- Added `just git-tree-hygiene-guard` and
  `just git-tree-hygiene-guard-selfcheck`.
- Wired the guard into `just handoff-check`.
- Wired the guard into the start of `scripts/ship_readiness.sh`, before that
  script generates tracked report artifacts.
- The guard fails on staged, unstaged and untracked files, but reports
  branch ahead/behind state without treating local commits as dirty tree.
- The guard does not run `git reset`, `git clean`, `git checkout`, `git stash`,
  `git add`, `git commit`, `git push` or any network/SSH action.

## Limits

- No push was performed.
- No SSH stand run was performed.
- No Real-World PASS is claimed.
