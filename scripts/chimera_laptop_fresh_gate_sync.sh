#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LAPTOP_HOST="${CHIMERA_LAPTOP_HOST:-}"
LAPTOP_USER="${CHIMERA_LAPTOP_USER:-}"
LAPTOP_PASS="${CHIMERA_LAPTOP_PASS:-}"
LAPTOP_REPO="${CHIMERA_LAPTOP_REPO:-/home/art/chimera-pq}"
LOAD_DURATION_SEC="${CHIMERA_LOAD_DURATION_SEC:-30}"
LOAD_MAX_AGE_SEC="${CHIMERA_LOAD_GATE_MAX_AGE_SEC:-300}"
TS="$(date +%Y%m%d_%H%M%S)"
OUT_DIR="${1:-$ROOT_DIR/docs/laptop_sync/$TS}"

mkdir -p "$OUT_DIR"

if [[ -z "$LAPTOP_HOST" || -z "$LAPTOP_USER" || -z "$LAPTOP_PASS" ]]; then
  echo "chimera-laptop-fresh-gate-sync: CHIMERA_LAPTOP_HOST/USER/PASS are required" >&2
  exit 2
fi

if ! command -v sshpass >/dev/null 2>&1; then
  echo "chimera-laptop-fresh-gate-sync: sshpass is required" >&2
  exit 1
fi

remote="$LAPTOP_USER@$LAPTOP_HOST"

sshpass -p "$LAPTOP_PASS" ssh -o StrictHostKeyChecking=no "$remote" "
  set -euo pipefail
  cd '$LAPTOP_REPO'
  chmod +x scripts/chimera*.sh
  test -f configs/chimera-app-routes.conf || cp configs/chimera-app-routes.example.conf configs/chimera-app-routes.conf
  bash scripts/chimera-path-proof.sh docs/CHIMERA_PATH_PROOF.json >/dev/null 2>&1 || true
  bash scripts/chimera_channel_audit.sh docs/CHIMERA_CHANNEL_AUDIT.json >/dev/null 2>&1 || true
  CHIMERA_QUIET=1 bash scripts/chimera_runtime_verification.sh >/dev/null 2>&1 || true
  CHIMERA_QUIET=1 bash scripts/upstream_resilience_smoke.sh docs/UPSTREAM_RESILIENCE_SMOKE.json >/dev/null 2>&1 || true
  CHIMERA_QUIET=1 bash scripts/chimera_e2e_channel_gate.sh docs/CHIMERA_E2E_CHANNEL_GATE.json >/dev/null 2>&1 || true
  bash scripts/chimera_e2e_channel_gate_guard.sh docs/CHIMERA_E2E_CHANNEL_GATE.json
  CHIMERA_LOAD_DURATION_SEC='$LOAD_DURATION_SEC' CHIMERA_LOAD_GATE_MAX_AGE_SEC='$LOAD_MAX_AGE_SEC' CHIMERA_LOAD_GATE_FORCE_FRESH=1 \
    bash scripts/chimera_load_gate_laptop.sh docs/CHIMERA_LOAD_GATE_LAPTOP.json >/dev/null
  bash scripts/chimera_fresh_gate_report.sh docs/CHIMERA_FRESH_GATE_REPORT.json >/dev/null
  ls -1t docs/load/CHIMERA_LOAD_*_LAPTOP_*.json | head -n 1
" >/tmp/chimera_laptop_latest_load_path.txt

latest_load_rel="$(tail -n 1 /tmp/chimera_laptop_latest_load_path.txt | tr -d '\r')"
rm -f /tmp/chimera_laptop_latest_load_path.txt

for f in CHIMERA_FRESH_GATE_REPORT.json CHIMERA_PATH_PROOF.json CHIMERA_CHANNEL_AUDIT.json CHIMERA_E2E_CHANNEL_GATE.json CHIMERA_LOAD_GATE_LAPTOP.json; do
  sshpass -p "$LAPTOP_PASS" scp -o StrictHostKeyChecking=no "$remote:$LAPTOP_REPO/docs/$f" "$OUT_DIR/$f" >/dev/null
done
sshpass -p "$LAPTOP_PASS" scp -o StrictHostKeyChecking=no "$remote:$LAPTOP_REPO/$latest_load_rel" "$OUT_DIR/$(basename "$latest_load_rel")" >/dev/null

echo "sync_dir=$OUT_DIR"
echo "fresh_report=$OUT_DIR/CHIMERA_FRESH_GATE_REPORT.json"
