#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOAD_DIR="$ROOT_DIR/docs/load"
OUT_JSON="${1:-$ROOT_DIR/docs/CHIMERA_LOAD_GATE_LAPTOP.json}"
OUT_MD="${OUT_JSON%.json}.md"
MIN_SUCCESS_RATE="${CHIMERA_LOAD_GATE_MIN_SUCCESS_RATE:-0.95}"
MIN_TOTAL_REQUESTS="${CHIMERA_LOAD_GATE_MIN_TOTAL_REQUESTS:-100}"
RUN_IF_MISSING="${CHIMERA_LOAD_GATE_RUN_IF_MISSING:-1}"
FORCE_FRESH="${CHIMERA_LOAD_GATE_FORCE_FRESH:-0}"
MAX_AGE_SEC="${CHIMERA_LOAD_GATE_MAX_AGE_SEC:-3600}"

if ! command -v jq >/dev/null 2>&1; then
  echo "chimera-load-gate-laptop: jq is required" >&2
  exit 1
fi

mkdir -p "$LOAD_DIR"
mkdir -p "$(dirname "$OUT_JSON")"

latest_load="$(ls -1t "$LOAD_DIR"/CHIMERA_LOAD_*_LAPTOP_*.json 2>/dev/null | head -n 1 || true)"

if [[ "$FORCE_FRESH" == "1" ]]; then
  bash "$ROOT_DIR/scripts/chimera_load_5m_laptop.sh" "$LOAD_DIR" >/dev/null
  latest_load="$(ls -1t "$LOAD_DIR"/CHIMERA_LOAD_*_LAPTOP_*.json 2>/dev/null | head -n 1 || true)"
elif [[ -z "$latest_load" && "$RUN_IF_MISSING" == "1" ]]; then
  bash "$ROOT_DIR/scripts/chimera_load_5m_laptop.sh" "$LOAD_DIR" >/dev/null
  latest_load="$(ls -1t "$LOAD_DIR"/CHIMERA_LOAD_*_LAPTOP_*.json 2>/dev/null | head -n 1 || true)"
fi

if [[ -z "$latest_load" ]]; then
  echo "chimera-load-gate-laptop: no load artifact found" >&2
  exit 1
fi

if ! [[ "$MAX_AGE_SEC" =~ ^[0-9]+$ ]] || (( MAX_AGE_SEC < 1 )); then
  echo "chimera-load-gate-laptop: CHIMERA_LOAD_GATE_MAX_AGE_SEC must be positive integer" >&2
  exit 1
fi

now_epoch="$(date +%s)"
mtime_epoch="$(stat -c %Y "$latest_load")"
age_sec=$((now_epoch - mtime_epoch))
if (( age_sec < 0 )); then
  echo "chimera-load-gate-laptop: artifact mtime is in the future: $latest_load" >&2
  exit 1
fi
if (( age_sec > MAX_AGE_SEC )); then
  echo "chimera-load-gate-laptop: stale artifact (${age_sec}s > ${MAX_AGE_SEC}s): $latest_load" >&2
  exit 1
fi

min_rate="$(jq -r '[.sites[].success_rate] | min' "$latest_load")"
max_rate="$(jq -r '[.sites[].success_rate] | max' "$latest_load")"
total_ok="$(jq -r '[.sites[].ok] | add' "$latest_load")"
total_fail="$(jq -r '[.sites[].fail] | add' "$latest_load")"
total_req=$((total_ok + total_fail))

rate_pass="$(awk -v x="$min_rate" -v y="$MIN_SUCCESS_RATE" 'BEGIN{print (x>=y)?"true":"false"}')"
volume_pass="$(awk -v x="$total_req" -v y="$MIN_TOTAL_REQUESTS" 'BEGIN{print (x>=y)?"true":"false"}')"

status="fail"
reason="load_gate_failed"
if [[ "$rate_pass" == "true" && "$volume_pass" == "true" ]]; then
  status="pass"
  reason="load_gate_ok"
fi

cat >"$OUT_JSON" <<EOF
{
  "kind": "chimera_load_gate_laptop",
  "status": "$status",
  "reason": "$reason",
  "thresholds": {
    "min_success_rate": $MIN_SUCCESS_RATE,
    "min_total_requests": $MIN_TOTAL_REQUESTS,
    "max_age_sec": $MAX_AGE_SEC
  },
  "measured": {
    "artifact": "$latest_load",
    "artifact_age_sec": $age_sec,
    "min_success_rate": $min_rate,
    "max_success_rate": $max_rate,
    "total_ok": $total_ok,
    "total_fail": $total_fail,
    "total_requests": $total_req
  },
  "checks": {
    "success_rate_pass": $rate_pass,
    "volume_pass": $volume_pass
  }
}
EOF

cat >"$OUT_MD" <<EOF
# CHIMERA Load Gate (Laptop)

- status: $status
- reason: $reason
- artifact: $latest_load

Thresholds:
- min_success_rate: $MIN_SUCCESS_RATE
- min_total_requests: $MIN_TOTAL_REQUESTS

Measured:
- min_success_rate: $min_rate
- max_success_rate: $max_rate
- total_ok: $total_ok
- total_fail: $total_fail
- total_requests: $total_req
EOF

cat "$OUT_JSON"

[[ "$status" == "pass" ]]
