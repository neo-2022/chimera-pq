#!/usr/bin/env bash
set -euo pipefail

resolve_self() {
  local src="${BASH_SOURCE[0]}"
  while [[ -L "$src" ]]; do
    local dir
    dir="$(cd "$(dirname "$src")" && pwd)"
    src="$(readlink "$src")"
    [[ "$src" != /* ]] && src="$dir/$src"
  done
  cd "$(dirname "$src")" && pwd
}

SCRIPT_DIR="$(resolve_self)"
exec "$SCRIPT_DIR/chimera-sh" "$@"
