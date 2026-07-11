#!/usr/bin/env bash
# Standalone rollback cleanliness verifier for CHIMERA-PQ / WEAVE.
#
# This script is read-only by default.  It reports whether OS network state
# still contains CHIMERA-managed artifacts (TUN devices, nftables tables,
# ip rules/routes, and leftover DNS backups).
#
# Optional remote reachability checks are accepted only through environment
# variables; no stand addresses are hardcoded.
#
# Usage:
#   CHIMERA_VERIFY_REMOTE_HOSTS="host1 host2" ./scripts/chimera-rollback-verify.sh
#
# Exit codes:
#   0  system appears clean
#   1  CHIMERA-managed artifacts are still present
#   2  usage error

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
CONTROL_SCRIPT="$ROOT_DIR/scripts/chimera-control.sh"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  echo "Usage: $0"
  echo "Environment:"
  echo "  CHIMERA_VERIFY_REMOTE_HOSTS  space-separated hosts to ping (optional)"
  exit 0
fi

if [[ ! -x "$CONTROL_SCRIPT" ]]; then
  echo "verify=error reason=control_script_missing path=$CONTROL_SCRIPT" >&2
  exit 2
fi

exec "$CONTROL_SCRIPT" verify-rollback
