# Troubleshooting (MVP Handoff/Ship)

## 1) `baseline-verify` fails (checksum mismatch)

Symptom:

- `sha256sum: WARNING: 1 computed checksum did NOT match`

Cause:

- One or more frozen artifacts changed after baseline snapshot.

Fix:

1. Run: `just baseline-freeze`
2. Re-check: `just baseline-verify`

## 2) `cleanroom-handoff-check` fails in the middle

Symptom:

- `just handoff-check` fails inside clean-room copy.

Fix:

1. Re-run full pipeline once: `just ship-readiness`
2. If still failing, inspect latest:
   - `docs/SECOND_MACHINE_REPORT.md`
   - `docs/RELEASE_READINESS_REPORT.json`
   - `docs/MVP_VERIFY.json`
3. Validate clean-room script shape: `just cleanroom-handoff-selfcheck`

## 3) `ship-readiness` fails on final contract check

Symptom:

- `just ship-report-contract-check` fails.

Cause:

- `docs/SHIP_READINESS_REPORT.json` misses required pass fields.

Fix:

1. Re-run: `just ship-readiness`
2. Verify launcher script shape: `just ship-readiness-selfcheck`
3. Verify truth contract: `just truth-contract-check`
4. Validate report manually has:
   - `"status":"ok"`
   - `"release_ok":true`
   - `"release_ok_lab_only":true`
   - `"network_state_not_modified":true`
   - `"runtime_apply_smoke_modified":true`
   - `"runtime_apply_route_smoke_modified":true`
   - `"artifacts_fresh":true`
   - `"benchmark_regression_gate":true` in `steps`
   - `"runtime_apply_dns_smoke":true` in `steps`
   - `"freshness_check":true` in `steps`
   - `"runtime_apply_route_smoke":true` in `steps`
   - `docs/benchmark_baseline.json` exists
   - `docs/benchmark_latest.json` exists
   - `docs/BENCHMARK_REGRESSION_GATE.json` exists
   - `"status":"ok"` in `docs/BENCHMARK_REGRESSION_GATE.json`
   - `"runtime_apply_dns_verified":true` in `docs/RELEASE_READINESS_REPORT.json` (`release_gate`)
   - `"runtime_apply_route_verified":true` in `docs/RELEASE_READINESS_REPORT.json` (`release_gate`)
   - `"runtime_route_policy_validation_verified":true` in `docs/RELEASE_READINESS_REPORT.json` (`release_gate`)
   - `"runtime_tun_name_validation_verified":true` in `docs/RELEASE_READINESS_REPORT.json` (`release_gate`)
   - `"runtime_forced_stop_rollback_verified":true` in `docs/RELEASE_READINESS_REPORT.json` (`release_gate`)
5. Validate runtime apply artifact:
   - `docs/RUNTIME_APPLY_DNS_SMOKE.json`
   - `"status":"ok"`
   - `"network_state":"modified"`
   - `"rollback_ok":true`
6. Validate runtime route artifact:
   - `docs/RUNTIME_APPLY_ROUTE_SMOKE.json`
   - `"status":"ok"`
   - `"network_state":"modified"`
   - `"apply_attempt_ok":true`
   - `"rollback_ok":true`

## 4) Performance values fluctuate

Symptom:

- `benchmark_latest.json` values differ between runs.

Cause:

- Normal machine noise.

Fix:

1. Validate gate script shape:
   - `just benchmark-regression-selfcheck`
2. Re-run once (the gate itself also retries once automatically).
3. Inspect benchmark gate artifact:
   - `docs/BENCHMARK_REGRESSION_GATE.json`
   - `"status":"ok"` expected for green pipeline
4. If gates pass, refresh baseline:
   - `just baseline-freeze`
5. Re-verify:
   - `just baseline-verify`

## Safety Reminder

Default verification flow is local and normally keeps:

- `"network_state":"not_modified"`

Explicit runtime apply checks (`--apply-*`) are also valid in controlled smoke
tests and must then report:

- `"network_state":"modified"`
