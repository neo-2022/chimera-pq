#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONTROL="$ROOT_DIR/scripts/chimera-control.sh"
OUT_FILE="${1:-$ROOT_DIR/docs/UPSTREAM_RESILIENCE_SMOKE.json}"
TMP_LOG="$(mktemp)"

now_epoch() {
  date +%s
}

json_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

read_kv() {
  local key="$1"
  local file="$2"
  awk -F'=' -v k="$key" '$1==k{print $2; exit}' "$file"
}

mkdir -p "$(dirname "$OUT_FILE")"

# Ensure channel is up.
"$CONTROL" start >/dev/null 2>&1 || true

# Reset sticky/health and collect baseline.
"$CONTROL" upstream-reset >/dev/null 2>&1 || true
"$CONTROL" upstream-probe >"$TMP_LOG"
"$CONTROL" proxy-status >>"$TMP_LOG"

best_endpoint="$(awk -F'endpoint=| latency_ms=' '/^upstream_best /{print $2; exit}' "$TMP_LOG")"
best_latency_ms="$(awk -F'latency_ms=| strategy=' '/^upstream_best /{print $2; exit}' "$TMP_LOG")"
strategy="$(awk -F'=' '/^upstream_strategy=/{print $2; exit}' "$TMP_LOG")"

pre_health_ok="$(awk -F'=' '/^upstream_health_ok=/{print $2; exit}' "$TMP_LOG")"
pre_degrade_fails="$(awk -F'=' '/^upstream_degrade_fails=/{print $2; exit}' "$TMP_LOG")"

"$CONTROL" proxy-status >"$TMP_LOG"
"$CONTROL" upstream-audit 20 >>"$TMP_LOG"

post_health_ok="$(awk -F'=' '/^upstream_health_ok=/{print $2; exit}' "$TMP_LOG")"
post_degrade_fails="$(awk -F'=' '/^upstream_degrade_fails=/{print $2; exit}' "$TMP_LOG")"
post_last_reason="$(awk -F'=' '/^upstream_last_reason=/{print $2; exit}' "$TMP_LOG")"
post_last_endpoint="$(awk -F'=' '/^upstream_last_endpoint=/{print $2; exit}' "$TMP_LOG")"

event_count="$(rg -c 'endpoint_probe|tunnel_up endpoint=|reason=' "$TMP_LOG" || true)"

ts="$(now_epoch)"

cat >"$OUT_FILE" <<EOF
{
  "ts": $ts,
  "smoke": "upstream_resilience",
  "best_endpoint": "$(json_escape "${best_endpoint:-unknown}")",
  "best_latency_ms": "$(json_escape "${best_latency_ms:-unknown}")",
  "strategy": "$(json_escape "${strategy:-unknown}")",
  "pre": {
    "health_ok": "$(json_escape "${pre_health_ok:-unknown}")",
    "degrade_fails": "$(json_escape "${pre_degrade_fails:-unknown}")"
  },
  "post": {
    "health_ok": "$(json_escape "${post_health_ok:-unknown}")",
    "degrade_fails": "$(json_escape "${post_degrade_fails:-unknown}")",
    "last_reason": "$(json_escape "${post_last_reason:-unknown}")",
    "last_endpoint": "$(json_escape "${post_last_endpoint:-unknown}")"
  },
  "event_count": ${event_count:-0},
  "outcome": "$( [ "${post_health_ok:-false}" = "true" ] && echo "pass" || echo "partial" )"
}
EOF

rm -f "$TMP_LOG"
echo "$OUT_FILE"
