#!/usr/bin/env bash
# Phase 3 sealed multi-hop transit runtime harness.
# Reads all stand coordinates from environment. Does nothing permanent.
# This script is intentionally a skeleton; required CLI RPCs for bound-route
# advertisement are not yet implemented.

set -euo pipefail

: "${PHASE3_SOURCE_NODE?source node user@host}"
: "${PHASE3_TRANSIT_NODE?transit node user@host}"
: "${PHASE3_DEST_NODE?destination node user@host}"
: "${PHASE3_DEST_DOMAIN?domain that resolves to destination}"
: "${PHASE3_DEST_PORT?port of echo server on destination}"

check_ssh() {
  local spec="$1"
  local user="${spec%%@*}"
  local host="${spec#*@}"
  ssh -o BatchMode=yes -o ConnectTimeout=5 -o StrictHostKeyChecking=accept-new \
      "${user}@${host}" 'echo ssh_ok' || { echo "SSH failed for $spec"; return 1; }
}

echo "=== Phase 3 multi-hop transit harness ==="
echo "Source : ${PHASE3_SOURCE_NODE}"
echo "Transit: ${PHASE3_TRANSIT_NODE}"
echo "Dest   : ${PHASE3_DEST_NODE}"
echo "Target : ${PHASE3_DEST_DOMAIN}:${PHASE3_DEST_PORT}"

check_ssh "${PHASE3_SOURCE_NODE}"
check_ssh "${PHASE3_TRANSIT_NODE}"
check_ssh "${PHASE3_DEST_NODE}"

echo "All nodes reachable. Skeleton harness complete; no live state changed."
echo "Next: implement bound-route advertisement RPC or planner hint."
