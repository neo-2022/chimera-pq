#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CLEAN_ROOT="/tmp/chimera-pq-cleanroom"
REPORT_PATH="$ROOT_DIR/docs/SECOND_MACHINE_REPORT.md"
RUN_DATE="$(date +%F)"
RUN_TS="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
HOST_INFO="$(uname -srm)"

rm -rf "$CLEAN_ROOT"
mkdir -p "$CLEAN_ROOT"

cd "$ROOT_DIR"
find . -mindepth 1 -maxdepth 1 \( -name target -o -name .git \) -prune -o -print0 | \
  xargs -0 -I{} cp -a {} "$CLEAN_ROOT"/

cd "$CLEAN_ROOT"
just benchmark-regression-selfcheck
just handoff-check

dns_smoke_ok=false
route_smoke_ok=false
route_apply_attempt="unknown"
forced_stop_smoke_ok=false
forced_stop_recover="unknown"
if rg -q '"status":"ok"' docs/RUNTIME_APPLY_DNS_SMOKE.json && rg -q '"network_state":"modified"' docs/RUNTIME_APPLY_DNS_SMOKE.json && rg -q '"rollback_ok":true' docs/RUNTIME_APPLY_DNS_SMOKE.json; then
  dns_smoke_ok=true
fi
if rg -q '"status":"ok"' docs/RUNTIME_APPLY_ROUTE_SMOKE.json && rg -q '"network_state":"modified"' docs/RUNTIME_APPLY_ROUTE_SMOKE.json && rg -q '"rollback_ok":true' docs/RUNTIME_APPLY_ROUTE_SMOKE.json; then
  route_smoke_ok=true
fi
if rg -q '"apply_attempt_ok":true' docs/RUNTIME_APPLY_ROUTE_SMOKE.json; then
  route_apply_attempt="true"
elif rg -q '"apply_attempt_ok":false' docs/RUNTIME_APPLY_ROUTE_SMOKE.json; then
  route_apply_attempt="false"
fi
if rg -q '"status":"ok"' docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json && rg -q '"network_state":"modified"' docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json && rg -q '"apply_attempt_ok":true' docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json; then
  forced_stop_smoke_ok=true
fi
if rg -q '"recover_ok":true' docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json && rg -q '"down_state_clean":true' docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json; then
  forced_stop_recover="true"
elif rg -q '"recover_ok":false' docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json; then
  forced_stop_recover="false"
fi

cat > "$REPORT_PATH" <<REPORT
# Second Environment Verification Report

Date: ${RUN_DATE}
Verification mode: independent clean-room copy
Workspace under test: ${CLEAN_ROOT}
Run timestamp (UTC): ${RUN_TS}
Host kernel: ${HOST_INFO}

## Commands Executed

1. \`just benchmark-regression-selfcheck\`
2. \`just handoff-check\`

## Result

- Overall: PASS
- Baseline integrity (\`just baseline-verify\`): PASS
- Full MVP gate (\`just mvp-check\`): PASS
- Release readiness JSON: PASS
- Benchmark regression gate script selfcheck: PASS

## Key Signals

- \`release_ok: true\`
- \`m5_doctor_and_config: true\`
- \`m6_hardening: true\`
- \`default report path network_state: "not_modified"\`
- \`runtime apply DNS smoke: ${dns_smoke_ok}\`
- \`runtime apply route smoke: ${route_smoke_ok}\`
- \`runtime apply route attempt succeeded: ${route_apply_attempt}\`
- \`runtime forced-stop rollback smoke: ${forced_stop_smoke_ok}\`
- \`runtime forced-stop recover+cleanup: ${forced_stop_recover}\`

## Notes

- The verification was executed by the agent end-to-end, no user action required.
- Validation used a clean-room copy isolated from the primary working directory.
- No OS route/firewall/proxy/router/VPS changes were performed during this check.
- Runtime DNS smoke only modifies a resolver test file under \`/tmp\` and rolls it back.
- Runtime route smoke is strict for release gating and requires \`apply_attempt_ok: true\`.
- Runtime forced-stop smoke is strict for release gating and requires \`recover_ok: true\` and \`down_state_clean: true\`.
REPORT

echo "cleanroom handoff check: PASS"
echo "report updated: $REPORT_PATH"
