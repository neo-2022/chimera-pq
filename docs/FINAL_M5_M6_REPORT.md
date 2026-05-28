# FINAL M5/M6 Report

Date: 2026-05-18
Project: CHIMERA-PQ WEAVE MVP
Scope: M5 (Practical WEAVE Usability) and M6 (Hardening Gate)

## Final Status

- M5: PASS
- M6: PASS
- Release readiness: PASS
- Release readiness interpretation: LAB-ONLY PASS (`release_ok_lab_only: true`)
- Network safety contract: PASS for default validation path (`network_state: not_modified`)
- Explicit runtime apply path exists (`--apply-tun|--apply-route|--apply-dns`) and reports `network_state: modified`
- Truth boundary: `lab_scope_only=true`, `real_world_datapath_closed=false`

## What Was Completed

### M5 (Practical WEAVE Usability)

- Client/gateway/lab doctor checks are passing and produce JSON artifacts.
- Route explain artifact is generated and validated.
- Rollback status/recover/status-after-recover artifacts are generated and validated.
- Operational runbook is aligned with the current verification flow.

### M6 (Hardening Gate)

- Parser hardening in `chimera-config`:
  - duplicate keys are rejected;
  - unknown keys are rejected;
  - unknown-key diagnostics include line number;
  - typo suggestion (`did you mean ...`) for close-key mistakes;
  - duplicate-key diagnostics include first declaration line.
- `config-smoke` now includes negative parser smoke checks for:
  - unknown client key;
  - unknown gateway key;
  - duplicate key;
  - malformed `key=value` line.
- Fuzz/perf/net-sim/hardening checks pass in the lab pipeline.

## Verification Commands Executed

Final validation sequence run successfully:

1. `just mvp-check`
2. `just release-readiness-report-json`

Additionally, full refresh verification was run:

1. `just mvp-verify-refresh`

All above commands exited with code `0`.

## Key Output Signals

From release readiness and MVP verification artifacts:

- `release_ok: true`
- `release_ok_lab_only: true`
- `m5_doctor_and_config: true`
- `m6_hardening: true`
- `parser_fuzz_smoke_passes: true`
- `benchmark_report_exists: true`
- default validation artifacts: `network_state: "not_modified"`
- explicit runtime apply artifacts: `network_state: "modified"`
- truth boundary: `lab_scope_only=true`, `real_world_datapath_closed=false`

## Primary Artifacts

- `docs/M5_ARTIFACTS_REPORT.md`
- `docs/M6_ARTIFACTS_REPORT.md`
- `docs/RELEASE_READINESS_REPORT.md`
- `docs/RELEASE_READINESS_REPORT.json`
- `docs/SHIP_READINESS_REPORT.json`
- `docs/MVP_SPEC_COVERAGE.md`
- `docs/MVP_VERIFY.json`
- `docs/MVP_SNAPSHOT.json`
- `docs/REPORT_PACK.md`
- `docs/REPORT_PACK.json`
- `docs/ARTIFACT_AUDIT.json`
- `docs/benchmark_baseline.json`

## Safety Note

During default validation cycle, OS routes/DNS/firewall/proxy/router/VPS were not modified.
Explicit runtime apply checks may modify local OS TUN/route/DNS and must be followed by rollback.
