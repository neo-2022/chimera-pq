#!/usr/bin/env bash
set -euo pipefail

REPORT_PATH="${1:-docs/COMMAND_EXECUTION_LOCK.json}"

fail() {
  echo "absolute completion lock guard: FAIL $1" >&2
  exit 1
}

if [[ ! -f "$REPORT_PATH" ]]; then
  fail "missing_report path=$REPORT_PATH"
fi

if ! command -v jq >/dev/null 2>&1; then
  fail "jq_required"
fi

status="$(jq -r '.status // "missing"' "$REPORT_PATH" 2>/dev/null || echo "missing")"
cmd_exh="$(jq -r '.command_exhaustive // "missing"' "$REPORT_PATH" 2>/dev/null || echo "missing")"
ver_exh="$(jq -r '.verification_exhaustive // "missing"' "$REPORT_PATH" 2>/dev/null || echo "missing")"
evd_exh="$(jq -r '.evidence_exhaustive // "missing"' "$REPORT_PATH" 2>/dev/null || echo "missing")"

[[ "$cmd_exh" == "true" ]] || fail "command_exhaustive_not_true value=$cmd_exh path=$REPORT_PATH"
[[ "$ver_exh" == "true" ]] || fail "verification_exhaustive_not_true value=$ver_exh path=$REPORT_PATH"
[[ "$evd_exh" == "true" ]] || fail "evidence_exhaustive_not_true value=$evd_exh path=$REPORT_PATH"

if [[ "$status" == "done" || "$status" == "pass" || "$status" == "completed" ]]; then
  :
elif [[ "$status" == "ok" ]]; then
  :
else
  fail "status_not_final value=$status path=$REPORT_PATH"
fi

echo "absolute completion lock guard: PASS path=$REPORT_PATH status=$status"
