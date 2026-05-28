#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${1:-$ROOT_DIR/docs/CHIMERA_AUTONOMOUS_NAT_GUARD.json}"
CONTROL="$ROOT_DIR/scripts/chimera-control.sh"

now_utc() { date -u +"%Y-%m-%dT%H:%M:%SZ"; }
esc() { printf '%s' "${1:-}" | sed 's/\\/\\\\/g; s/"/\\"/g'; }

mkdir -p "$(dirname "$OUT")"

started_at="$(now_utc)"

# 1) Local autonomy preflight: must not require manual route/proxy toggles.
"$CONTROL" start >/tmp/chimera_autonomous_start.log 2>&1 || true
proxy_status="$("$CONTROL" proxy-status 2>/dev/null || true)"
route_status="$("$CONTROL" route-status 2>/dev/null || true)"

listener_up="false"
if printf '%s\n' "$proxy_status" | grep -q '^chimera_proxy_listener=up$'; then
  listener_up="true"
fi

# 2) Path proof as autonomous reachability signal.
CHIMERA_QUIET=1 bash "$ROOT_DIR/scripts/chimera-path-proof.sh" "$ROOT_DIR/docs/CHIMERA_PATH_PROOF.json" >/dev/null 2>&1 || true
path_status="$(jq -r '.status // "unknown"' "$ROOT_DIR/docs/CHIMERA_PATH_PROOF.json" 2>/dev/null || echo unknown)"
path_reason="$(jq -r '.reason // "unknown"' "$ROOT_DIR/docs/CHIMERA_PATH_PROOF.json" 2>/dev/null || echo unknown)"

# 3) Egress adaptation capability signal (must be >=2 candidates for autonomous geo adaptation).
upstream_audit="$("$CONTROL" upstream-audit 20 2>/dev/null || true)"
candidates_total="$(printf '%s\n' "$upstream_audit" | awk -F= '/^upstream_candidates_total=/{print $2; exit}')"
adaptation_possible="$(printf '%s\n' "$upstream_audit" | awk -F= '/^upstream_adaptation_possible=/{print $2; exit}')"

if [[ -z "${candidates_total:-}" ]]; then candidates_total="0"; fi
if [[ -z "${adaptation_possible:-}" ]]; then adaptation_possible="false"; fi

status="fail"
reason="autonomy_guard_failed"
if [[ "$listener_up" == "true" && "$path_status" == "pass" && "$adaptation_possible" == "true" ]]; then
  status="pass"
  reason="autonomous_path_and_adaptation_ready"
elif [[ "$listener_up" == "true" && "$path_status" == "pass" ]]; then
  status="partial"
  reason="path_ready_but_multi_egress_missing"
fi

finished_at="$(now_utc)"

cat >"$OUT" <<EOF
{"kind":"chimera_autonomous_nat_guard","status":"$status","reason":"$reason","started_at":"$started_at","finished_at":"$finished_at","network_state":"not_modified","signals":{"listener_up":$listener_up,"path_status":"$(esc "$path_status")","path_reason":"$(esc "$path_reason")","upstream_candidates_total":$candidates_total,"upstream_adaptation_possible":"$(esc "$adaptation_possible")"}}
EOF

cat "$OUT"
[[ "$status" == "pass" ]]
