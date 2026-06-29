#!/usr/bin/env bash
set -euo pipefail

mode="${1:-strict}"

case "$mode" in
  strict | allow-no-git)
    ;;
  *)
    echo "git tree hygiene guard: invalid mode: $mode" >&2
    echo "usage: $0 [strict|allow-no-git]" >&2
    exit 2
    ;;
esac

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  if [[ "$mode" == "allow-no-git" ]]; then
    echo "git tree hygiene guard: SKIP not_git_worktree mode=allow-no-git"
    exit 0
  fi
  echo "git tree hygiene guard: FAIL not_git_worktree" >&2
  exit 2
fi

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

if ! git diff --check >/dev/null; then
  echo "git tree hygiene guard: FAIL unstaged_whitespace_errors" >&2
  exit 1
fi

if ! git diff --cached --check >/dev/null; then
  echo "git tree hygiene guard: FAIL staged_whitespace_errors" >&2
  exit 1
fi

status_output="$(git status --porcelain=v1 --untracked-files=all)"
if [[ -n "$status_output" ]]; then
  staged_count=0
  unstaged_count=0
  untracked_count=0
  while IFS= read -r line; do
    [[ -n "$line" ]] || continue
    index_status="${line:0:1}"
    worktree_status="${line:1:1}"
    if [[ "$line" == "?? "* ]]; then
      untracked_count=$((untracked_count + 1))
      continue
    fi
    if [[ "$index_status" != " " && "$index_status" != "?" ]]; then
      staged_count=$((staged_count + 1))
    fi
    if [[ "$worktree_status" != " " && "$worktree_status" != "?" ]]; then
      unstaged_count=$((unstaged_count + 1))
    fi
  done <<< "$status_output"

  echo "git tree hygiene guard: FAIL dirty_tree staged=${staged_count} unstaged=${unstaged_count} untracked=${untracked_count}" >&2
  echo "git tree hygiene guard: hint review with git status --short --untracked-files=all" >&2
  exit 1
fi

upstream_summary="none"
if git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' >/dev/null 2>&1; then
  read -r behind_count ahead_count < <(git rev-list --left-right --count '@{upstream}...HEAD')
  upstream_summary="behind=${behind_count} ahead=${ahead_count}"
fi

echo "git tree hygiene guard: PASS dirty_tree=false ${upstream_summary}"
