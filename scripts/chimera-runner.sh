#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ALLOW_BUILD_FALLBACK="${CHIMERA_ALLOW_BUILD_FALLBACK:-0}"

usage() {
  cat <<'EOF'
Usage:
  chimera-runner.sh <target> [args...]

Targets:
  cli       run chimera-cli with args
  gateway   run chimera-gateway with args
EOF
}

run_with_fallback() {
  local bin_path="$1"
  local cargo_pkg="$2"
  shift 2

  if [[ -x "$bin_path" ]]; then
    if "$bin_path" "$@"; then
      return 0
    fi
    if [[ "$ALLOW_BUILD_FALLBACK" != "1" ]]; then
      return 1
    fi
  fi

  if [[ "$ALLOW_BUILD_FALLBACK" == "1" ]] && command -v cargo >/dev/null 2>&1; then
    (
      cd "$ROOT_DIR"
      cargo run -q -p "$cargo_pkg" -- "$@"
    )
    return $?
  fi

  echo "error: failed to run $cargo_pkg binary and cargo fallback is unavailable" >&2
  return 1
}

target="${1:-}"
shift || true
case "$target" in
  cli)
    run_with_fallback "$ROOT_DIR/bin/chimera-cli" "chimera-cli" "$@"
    ;;
  gateway)
    run_with_fallback "$ROOT_DIR/bin/chimera-gateway" "chimera-gateway" "$@"
    ;;
  -h|--help|help|"")
    usage
    ;;
  *)
    echo "error: unknown target: $target" >&2
    usage
    exit 2
    ;;
esac
