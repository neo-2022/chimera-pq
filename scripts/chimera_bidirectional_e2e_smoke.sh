#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${1:-$ROOT_DIR/docs}"
TS="$(date +%Y-%m-%d_%H-%M-%S)"
OUT_FILE="$OUT_DIR/CHIMERA_BIDIRECTIONAL_E2E_SMOKE_${TS}.md"

require_env() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    echo "error: required env is not set: $name" >&2
    exit 2
  fi
}

require_env CHIMERA_SIDE_B_HOST
require_env CHIMERA_SIDE_B_USER
require_env CHIMERA_SIDE_B_PASS
require_env CHIMERA_SIDE_A_HOST
require_env CHIMERA_SIDE_A_USER
require_env CHIMERA_SIDE_A_PASS
require_env CHIMERA_UPSTREAM_USER
require_env CHIMERA_UPSTREAM_HOST
require_env CHIMERA_UPSTREAM_PASS
require_env CHIMERA_UPSTREAM_PORT
require_env CHIMERA_UPSTREAM_TRANSPORTS_CSV

SIDE_B_HOST="${CHIMERA_SIDE_B_HOST}"
SIDE_B_USER="${CHIMERA_SIDE_B_USER}"
SIDE_B_PASS="${CHIMERA_SIDE_B_PASS}"

SIDE_A_HOST="${CHIMERA_SIDE_A_HOST}"
SIDE_A_USER="${CHIMERA_SIDE_A_USER}"
SIDE_A_PASS="${CHIMERA_SIDE_A_PASS}"

mkdir -p "$OUT_DIR"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: missing command: $1" >&2
    exit 2
  }
}

need_cmd sshpass
need_cmd jq

side_b_json="$(
  sshpass -p "$SIDE_B_PASS" ssh -o StrictHostKeyChecking=no "$SIDE_B_USER@$SIDE_B_HOST" \
    "bash -s -- '$CHIMERA_UPSTREAM_USER' '$CHIMERA_UPSTREAM_HOST' '$CHIMERA_UPSTREAM_PASS' '$CHIMERA_UPSTREAM_PORT' '$CHIMERA_UPSTREAM_TRANSPORTS_CSV'" <<'EOS'
  set -euo pipefail
  CHIMERA_UPSTREAM_USER="$1"
  CHIMERA_UPSTREAM_HOST="$2"
  CHIMERA_UPSTREAM_PASS="$3"
  CHIMERA_UPSTREAM_PORT="$4"
  CHIMERA_UPSTREAM_TRANSPORTS_CSV="$5"
  cd "$HOME/.local/share/chimera-pq"
  mkdir -p ~/.config/chimera
  cat > ~/.config/chimera/upstream_proxy.env <<EOF
CHIMERA_UPSTREAM_USER=${CHIMERA_UPSTREAM_USER}
CHIMERA_UPSTREAM_HOST=${CHIMERA_UPSTREAM_HOST}
CHIMERA_UPSTREAM_PASS=${CHIMERA_UPSTREAM_PASS}
CHIMERA_UPSTREAM_PORT=${CHIMERA_UPSTREAM_PORT}
CHIMERA_UPSTREAM_TRANSPORTS_CSV=${CHIMERA_UPSTREAM_TRANSPORTS_CSV}
EOF
  bash scripts/chimera-control.sh start >/tmp/ch_bi_start_side_b.log 2>&1 || true
  sleep 2
  bash scripts/chimera_runtime_verification.sh >/tmp/ch_bi_verify_side_b.log 2>&1 || true
  bash scripts/chimera_e2e_channel_gate.sh docs/CHIMERA_E2E_CHANNEL_GATE_SIDE_B.json >/tmp/ch_bi_e2e_side_b.log 2>&1 || true
  path_status="$(jq -r '.status // "unknown"' docs/CHIMERA_PATH_PROOF.json 2>/dev/null || echo unknown)"
  path_reason="$(jq -r '.reason // "unknown"' docs/CHIMERA_PATH_PROOF.json 2>/dev/null || echo unknown)"
  gate_status="$(jq -r '.status // "unknown"' docs/CHIMERA_E2E_CHANNEL_GATE_SIDE_B.json 2>/dev/null || echo unknown)"
  gate_reason="$(jq -r '.reason // "unknown"' docs/CHIMERA_E2E_CHANNEL_GATE_SIDE_B.json 2>/dev/null || echo unknown)"
  printf '{"path_status":"%s","path_reason":"%s","gate_status":"%s","gate_reason":"%s"}\n' "$path_status" "$path_reason" "$gate_status" "$gate_reason"
EOS
)"

side_a_json="$(
  sshpass -p "$SIDE_A_PASS" ssh -o StrictHostKeyChecking=no "$SIDE_A_USER@$SIDE_A_HOST" \
    "bash -s -- '$CHIMERA_UPSTREAM_USER' '$CHIMERA_UPSTREAM_HOST' '$CHIMERA_UPSTREAM_PASS' '$CHIMERA_UPSTREAM_PORT' '$CHIMERA_UPSTREAM_TRANSPORTS_CSV'" <<'EOS'
  set -euo pipefail
  CHIMERA_UPSTREAM_USER="$1"
  CHIMERA_UPSTREAM_HOST="$2"
  CHIMERA_UPSTREAM_PASS="$3"
  CHIMERA_UPSTREAM_PORT="$4"
  CHIMERA_UPSTREAM_TRANSPORTS_CSV="$5"
  apt-get update -y >/tmp/ch_bi_apt_upd.log 2>&1 || true
  apt-get install -y sshpass >/tmp/ch_bi_apt_inst.log 2>&1 || true
  cd "$HOME/.local/share/chimera-pq"
  mkdir -p ~/.config/chimera
  cat > ~/.config/chimera/upstream_proxy.env <<EOF
CHIMERA_UPSTREAM_USER=${CHIMERA_UPSTREAM_USER}
CHIMERA_UPSTREAM_HOST=${CHIMERA_UPSTREAM_HOST}
CHIMERA_UPSTREAM_PASS=${CHIMERA_UPSTREAM_PASS}
CHIMERA_UPSTREAM_PORT=${CHIMERA_UPSTREAM_PORT}
CHIMERA_UPSTREAM_TRANSPORTS_CSV=${CHIMERA_UPSTREAM_TRANSPORTS_CSV}
EOF
  bash scripts/chimera-control.sh start >/tmp/ch_bi_start_side_a.log 2>&1 || true
  sleep 2
  CHIMERA_PATH_PROOF_ALLOW_SAME_IP=1 bash scripts/chimera_runtime_verification.sh >/tmp/ch_bi_verify_side_a.log 2>&1 || true
  CHIMERA_PATH_PROOF_ALLOW_SAME_IP=1 CHIMERA_E2E_ALLOW_WARN_AUDIT=1 bash scripts/chimera_e2e_channel_gate.sh docs/CHIMERA_E2E_CHANNEL_GATE_SIDE_A.json >/tmp/ch_bi_e2e_side_a.log 2>&1 || true
  path_status="$(jq -r '.status // "unknown"' docs/CHIMERA_PATH_PROOF.json 2>/dev/null || echo unknown)"
  path_reason="$(jq -r '.reason // "unknown"' docs/CHIMERA_PATH_PROOF.json 2>/dev/null || echo unknown)"
  gate_status="$(jq -r '.status // "unknown"' docs/CHIMERA_E2E_CHANNEL_GATE_SIDE_A.json 2>/dev/null || echo unknown)"
  gate_reason="$(jq -r '.reason // "unknown"' docs/CHIMERA_E2E_CHANNEL_GATE_SIDE_A.json 2>/dev/null || echo unknown)"
  printf '{"path_status":"%s","path_reason":"%s","gate_status":"%s","gate_reason":"%s"}\n' "$path_status" "$path_reason" "$gate_status" "$gate_reason"
EOS
)"

cat >"$OUT_FILE" <<EOF
# CHIMERA Bidirectional E2E Smoke

- generated_at: $(date -u +%Y-%m-%dT%H:%M:%SZ)
- side_b_host: ${SIDE_B_USER}@${SIDE_B_HOST}
- side_a_host: ${SIDE_A_USER}@${SIDE_A_HOST}

## Side B
${side_b_json}

## SIDE_A
${side_a_json}
EOF

echo "$OUT_FILE"
