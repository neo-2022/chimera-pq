#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  cat <<'EOF'
Usage:
  chimera-runner.sh <target> [args...]

Targets:
  cli       run chimera-cli with args
  gateway   run chimera-gateway with args
  peer-egress run chimera-peer-egress with args
  transparent-runtime run chimera-transparent-runtime with args
EOF
}

run_with_fallback() {
  local bin_path="$1"
  local runtime_name="$2"
  shift 2

  if [[ -x "$bin_path" ]]; then
    "$bin_path" "$@"
    return $?
  fi

  echo "error: missing shipped runtime binary: $runtime_name ($bin_path)" >&2
  return 1
}

prepare_transparent_runtime_env() {
  if [[ "${CHIMERA_RUNNER_USE_SUDO:-0}" == "1" && -z "${CHIMERA_NFT_PRIVILEGE_MODE:-}" ]]; then
    export CHIMERA_NFT_PRIVILEGE_MODE="sudo"
  fi
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
  peer-egress)
    peer_egress_mode="${CHIMERA_PEER_EGRESS_MODE:-}"
    if [[ -z "$peer_egress_mode" ]]; then
      echo "error: missing CHIMERA_PEER_EGRESS_MODE" >&2
      exit 1
    fi
    run_with_fallback "$ROOT_DIR/bin/chimera-peer-egress" "chimera-carrier" --mode "$peer_egress_mode" "$@"
    ;;
  transparent-runtime)
    prepare_transparent_runtime_env
    run_with_fallback "$ROOT_DIR/bin/chimera-transparent-runtime" "chimera-capture" "$@"
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
