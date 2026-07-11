#!/usr/bin/env bash
# CHIMERA mesh-node stabilization harness.
# This script runs from the control PC and SSH-executes lightweight probes on
# each stand node. Stand hosts and expected peer IPs must be supplied via
# environment variables; no stand addresses are hardcoded in this file.
#
# Required environment:
#   CHIMERA_STAND_NL_HOST
#   CHIMERA_STAND_RU_HOST
#   CHIMERA_STAND_LAPTOP_HOST
#   CHIMERA_STAND_EXPECT_NL_IP   - IP that NL mesh traffic should exit through (RU WAN)
#   CHIMERA_STAND_EXPECT_RU_IP   - IP that RU mesh traffic should exit through (NL WAN)
#
# Optional:
#   CHIMERA_STAND_NL_USER
#   CHIMERA_STAND_RU_USER
#   CHIMERA_STAND_LAPTOP_USER
#   CHIMERA_STAND_PROBE_TARGET
#   CHIMERA_STAND_PROBE_TIMEOUT
#   CHIMERA_STAND_EVIDENCE_DIR

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

NL_HOST="${CHIMERA_STAND_NL_HOST:-}"
RU_HOST="${CHIMERA_STAND_RU_HOST:-}"
LAPTOP_HOST="${CHIMERA_STAND_LAPTOP_HOST:-}"
EXPECT_NL_IP="${CHIMERA_STAND_EXPECT_NL_IP:-}"
EXPECT_RU_IP="${CHIMERA_STAND_EXPECT_RU_IP:-}"
NL_USER="${CHIMERA_STAND_NL_USER:-root}"
RU_USER="${CHIMERA_STAND_RU_USER:-root}"
LAPTOP_USER="${CHIMERA_STAND_LAPTOP_USER:-art}"
PROBE_TARGET="${CHIMERA_STAND_PROBE_TARGET:-http://ifconfig.me}"
PROBE_TIMEOUT="${CHIMERA_STAND_PROBE_TIMEOUT:-10}"
SSH_OPTS="-o StrictHostKeyChecking=accept-new -o UserKnownHostsFile=/dev/null -o BatchMode=yes -o ConnectTimeout=10"

if [[ -z "$NL_HOST" || -z "$RU_HOST" || -z "$LAPTOP_HOST" || -z "$EXPECT_NL_IP" || -z "$EXPECT_RU_IP" ]]; then
  echo "error: CHIMERA_STAND_NL_HOST, CHIMERA_STAND_RU_HOST, CHIMERA_STAND_LAPTOP_HOST," >&2
  echo "       CHIMERA_STAND_EXPECT_NL_IP and CHIMERA_STAND_EXPECT_RU_IP must be set" >&2
  exit 2
fi

OUT_DIR="${CHIMERA_STAND_EVIDENCE_DIR:-${SCRIPT_DIR}/../docs}"
TIMESTAMP="$(date -u +%Y%m%d-%H%M%S)"
EVIDENCE_FILE="${OUT_DIR}/STABILIZATION_EVIDENCE_${TIMESTAMP}.json"
mkdir -p "$OUT_DIR"

timestamp_ms() { date +%s%3N; }

ssh_cmd() {
  local user="$1" host="$2"
  shift 2
  ssh $SSH_OPTS "$user@$host" "$@"
}

json_escape() {
  printf '%s' "$1" | tr '\n' ' ' | sed 's/\\/\\\\/g; s/"/\\"/g'
}

probe_node_mesh() {
  local user="$1" host="$2" label="$3" expect_ip="$4"
  local start_ms end_ms elapsed_ms result rc=0
  start_ms=$(timestamp_ms)
  if [[ "$user" == "root" ]]; then
    result=$(ssh_cmd "$user" "$host" "runuser -u nobody -- curl -sS --max-time ${PROBE_TIMEOUT} '${PROBE_TARGET}'" 2>&1) || rc=$?
  else
    result=$(ssh_cmd "$user" "$host" "sudo -n -u nobody curl -sS --max-time ${PROBE_TIMEOUT} '${PROBE_TARGET}'" 2>&1) || rc=$?
  fi
  end_ms=$(timestamp_ms)
  elapsed_ms=$((end_ms - start_ms))
  local status
  if [[ "$rc" -eq 0 && "$result" == "$expect_ip" ]]; then
    status="pass"
  elif [[ "$rc" -eq 0 && "$expect_ip" == "mesh_any_peer" && -n "$result" && "$result" != FAIL* && "$result" != curl:* ]]; then
    status="pass"
  else
    status="fail"
  fi
  cat <<JSON
{"node":"$label","mode":"mesh","result":"$status","elapsed_ms":$elapsed_ms,"peer_ip":"$(json_escape "$result")","target":"$PROBE_TARGET","timestamp":$start_ms}
JSON
}

probe_node_direct() {
  local user="$1" host="$2" label="$3"
  local start_ms end_ms elapsed_ms result rc=0
  start_ms=$(timestamp_ms)
  result=$(ssh_cmd "$user" "$host" "curl -sS --max-time ${PROBE_TIMEOUT} '${PROBE_TARGET}'" 2>&1) || rc=$?
  end_ms=$(timestamp_ms)
  elapsed_ms=$((end_ms - start_ms))
  cat <<JSON
{"node":"$label","mode":"direct","result":"$([ "$rc" -eq 0 ] && echo pass || echo fail)","elapsed_ms":$elapsed_ms,"response":"$(json_escape "$result")","target":"$PROBE_TARGET","timestamp":$start_ms}
JSON
}

# Build evidence array.
declare -a ENTRIES
ENTRIES+=("$(probe_node_mesh "$NL_USER" "$NL_HOST" "amai" "$EXPECT_RU_IP")")
ENTRIES+=("$(probe_node_mesh "$RU_USER" "$RU_HOST" "vdsina" "$EXPECT_NL_IP")")
ENTRIES+=("$(probe_node_mesh "$LAPTOP_USER" "$LAPTOP_HOST" "laptop" "mesh_any_peer")")
ENTRIES+=("$(probe_node_direct "$NL_USER" "$NL_HOST" "amai")")
ENTRIES+=("$(probe_node_direct "$RU_USER" "$RU_HOST" "vdsina")")
ENTRIES+=("$(probe_node_direct "$LAPTOP_USER" "$LAPTOP_HOST" "laptop")")

{
  echo '{"status":"running","remote_stand_used":true,"nodes":["amai","vdsina","laptop"],"probes":['
  local first=1
  for entry in "${ENTRIES[@]}"; do
    if [[ "$first" -eq 1 ]]; then
      first=0
    else
      echo ","
    fi
    echo "$entry"
  done
  echo '],"evidence_file":"'"$EVIDENCE_FILE"'"}'
} > "$EVIDENCE_FILE"

echo "stabilization_harness=done evidence_file=$EVIDENCE_FILE"
