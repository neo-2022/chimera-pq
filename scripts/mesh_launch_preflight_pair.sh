#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# Required environment:
# - CHIMERA_MESH_LOCAL_ROLE      (side_a|side_b)
# - CHIMERA_MESH_NAMESPACE
# - CHIMERA_MESH_LOCAL_NODE
# - CHIMERA_MESH_REMOTE_NODE
# - CHIMERA_MESH_REMOTE_ENDPOINT   (host:port)
# - CHIMERA_MESH_LOCAL_OUT         (path to local preflight json)
# - CHIMERA_MESH_REMOTE_OUT        (path to remote preflight json)
# Optional:
# - CHIMERA_MESH_POLICY_PAYLOAD
# - CHIMERA_MESH_TRAFFIC_PROFILE
# - CHIMERA_MESH_TIMEOUT_MS
# - CHIMERA_MESH_VERIFY_OUT
# - CHIMERA_MESH_ALLOW_REMOTE_MISSING (0|1, default: 0)
# - CHIMERA_MESH_EXTRA_PEERS (comma or newline separated peer specs:
#   node@endpoint#region@load@reliability)

: "${CHIMERA_MESH_NAMESPACE:?missing CHIMERA_MESH_NAMESPACE}"
: "${CHIMERA_MESH_LOCAL_NODE:?missing CHIMERA_MESH_LOCAL_NODE}"
: "${CHIMERA_MESH_REMOTE_NODE:?missing CHIMERA_MESH_REMOTE_NODE}"
: "${CHIMERA_MESH_REMOTE_ENDPOINT:?missing CHIMERA_MESH_REMOTE_ENDPOINT}"
: "${CHIMERA_MESH_LOCAL_OUT:?missing CHIMERA_MESH_LOCAL_OUT}"
: "${CHIMERA_MESH_REMOTE_OUT:?missing CHIMERA_MESH_REMOTE_OUT}"
: "${CHIMERA_MESH_LOCAL_ROLE:?missing CHIMERA_MESH_LOCAL_ROLE}"

if [[ "$CHIMERA_MESH_LOCAL_ROLE" != "side_a" && "$CHIMERA_MESH_LOCAL_ROLE" != "side_b" ]]; then
  echo "[mesh-launch-preflight-pair] invalid CHIMERA_MESH_LOCAL_ROLE: $CHIMERA_MESH_LOCAL_ROLE (expected side_a|side_b)"
  exit 1
fi

POLICY_PAYLOAD="${CHIMERA_MESH_POLICY_PAYLOAD:-}"
TRAFFIC_PROFILE="${CHIMERA_MESH_TRAFFIC_PROFILE:-high_speed_anonymous}"
TIMEOUT_MS="${CHIMERA_MESH_TIMEOUT_MS:-1200}"
VERIFY_OUT="${CHIMERA_MESH_VERIFY_OUT:-docs/MESH_LAUNCH_PREFLIGHT_VERIFY.json}"
ALLOW_REMOTE_MISSING="${CHIMERA_MESH_ALLOW_REMOTE_MISSING:-0}"

if [[ "$ALLOW_REMOTE_MISSING" != "0" && "$ALLOW_REMOTE_MISSING" != "1" ]]; then
  echo "[mesh-launch-preflight-pair] invalid CHIMERA_MESH_ALLOW_REMOTE_MISSING: $ALLOW_REMOTE_MISSING (expected 0|1)"
  exit 1
fi

if [[ -n "$POLICY_PAYLOAD" && -n "$TRAFFIC_PROFILE" ]]; then
  echo "[mesh-launch-preflight-pair] invalid env: set either CHIMERA_MESH_POLICY_PAYLOAD or CHIMERA_MESH_TRAFFIC_PROFILE, not both"
  exit 1
fi

if [[ -n "$TRAFFIC_PROFILE" ]]; then
  case "$TRAFFIC_PROFILE" in
    high_speed_anonymous|privacy_first|speed_first|low_latency_private) ;;
    *)
      echo "[mesh-launch-preflight-pair] invalid CHIMERA_MESH_TRAFFIC_PROFILE: $TRAFFIC_PROFILE (expected high_speed_anonymous|privacy_first|speed_first|low_latency_private)"
      exit 1
      ;;
  esac
fi

REMOTE_PEER="${CHIMERA_MESH_REMOTE_NODE}@${CHIMERA_MESH_REMOTE_ENDPOINT}@eu@20@90"
EXTRA_PEERS_RAW="${CHIMERA_MESH_EXTRA_PEERS:-}"

trim_ascii() {
  local value="$1"
  # Trim leading whitespace.
  value="${value#"${value%%[![:space:]]*}"}"
  # Trim trailing whitespace.
  value="${value%"${value##*[![:space:]]}"}"
  printf '%s' "$value"
}

validate_peer_spec() {
  local peer="$1"
  local context="$2"
  if ! [[ "$peer" =~ ^[^@[:space:]]+@[^@[:space:]]+:[0-9]+@[^@[:space:]]+@[0-9]+@[0-9]+$ ]]; then
    echo "[mesh-launch-preflight-pair] invalid peer spec in ${context}: $peer"
    echo "[mesh-launch-preflight-pair] expected format: node@endpoint:port#region@load@reliability"
    exit 1
  fi
}

declare -A seen_peers=()
peer_args=()
add_peer_arg() {
  local peer="$1"
  if [[ -n "${seen_peers[$peer]+x}" ]]; then
    return
  fi
  seen_peers["$peer"]=1
  peer_args+=("--peer" "$peer")
}

validate_peer_spec "$REMOTE_PEER" "CHIMERA_MESH_REMOTE_NODE/CHIMERA_MESH_REMOTE_ENDPOINT"
add_peer_arg "$REMOTE_PEER"
if [[ -n "$EXTRA_PEERS_RAW" ]]; then
  while IFS= read -r peer; do
    peer_trimmed="$(trim_ascii "$peer")"
    if [[ -n "$peer_trimmed" ]]; then
      validate_peer_spec "$peer_trimmed" "CHIMERA_MESH_EXTRA_PEERS"
      add_peer_arg "$peer_trimmed"
    fi
  done < <(printf '%s' "$EXTRA_PEERS_RAW" | tr ',' '\n')
fi

echo "[mesh-launch-preflight-pair] local preflight -> ${CHIMERA_MESH_LOCAL_OUT}"
policy_args=()
if [[ -n "$POLICY_PAYLOAD" ]]; then
  policy_args=("--policy-payload" "$POLICY_PAYLOAD")
else
  policy_args=("--traffic-profile" "$TRAFFIC_PROFILE")
fi

cargo run -q -p chimera-cli -- mesh launch-preflight \
  --namespace "$CHIMERA_MESH_NAMESPACE" \
  --node "$CHIMERA_MESH_LOCAL_NODE" \
  "${policy_args[@]}" \
  "${peer_args[@]}" \
  --timeout-ms "$TIMEOUT_MS" \
  --json \
  --out "$CHIMERA_MESH_LOCAL_OUT"

if [[ ! -f "$CHIMERA_MESH_REMOTE_OUT" ]]; then
  if [[ "$ALLOW_REMOTE_MISSING" == "1" ]]; then
    echo "[mesh-launch-preflight-pair] remote artifact missing, verify skipped by CHIMERA_MESH_ALLOW_REMOTE_MISSING=1"
    echo "[mesh-launch-preflight-pair] local preflight artifact is ready: $CHIMERA_MESH_LOCAL_OUT"
    exit 0
  fi
  echo "[mesh-launch-preflight-pair] missing remote artifact: $CHIMERA_MESH_REMOTE_OUT"
  echo "[mesh-launch-preflight-pair] run the same script on the remote host with swapped LOCAL/REMOTE variables."
  exit 1
fi

if [[ "$CHIMERA_MESH_LOCAL_ROLE" == "side_a" ]]; then
  VERIFY_VPS_REPORT="$CHIMERA_MESH_LOCAL_OUT"
  VERIFY_LAPTOP_REPORT="$CHIMERA_MESH_REMOTE_OUT"
else
  VERIFY_VPS_REPORT="$CHIMERA_MESH_REMOTE_OUT"
  VERIFY_LAPTOP_REPORT="$CHIMERA_MESH_LOCAL_OUT"
fi

echo "[mesh-launch-preflight-pair] verify pair -> ${VERIFY_OUT}"
cargo run -q -p chimera-cli -- mesh launch-preflight-verify \
  --vps-report "$VERIFY_VPS_REPORT" \
  --laptop-report "$VERIFY_LAPTOP_REPORT" \
  --json \
  --out "$VERIFY_OUT"

echo "[mesh-launch-preflight-pair] done"
