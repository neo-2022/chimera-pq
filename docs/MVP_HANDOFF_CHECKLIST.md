# MVP Handoff Checklist (Linux)

This checklist is for a clean Linux machine validation of CHIMERA-PQ MVP.
Default validation commands keep OS routes/DNS/firewall/proxy unchanged.
The explicit runtime smoke step (`runtime-apply-dns-smoke`) intentionally applies
DNS only to a test file in `/tmp` and then rolls it back.

## 1) Environment

1. Install Rust toolchain.
2. Install `just` (optional, but recommended).
3. Open repository root: `chimera-pq/`.

## 2) Fast Gate (Single Command)

1. Run: `just handoff-check`
2. Expected: exit code `0` (runs baseline integrity + full MVP check + release readiness JSON).
3. For isolated reproducibility check (no manual setup), run: `just cleanroom-handoff-check`.
4. For full final packaging in one command, run: `just ship-readiness`.
5. Performance gate script integrity is verified inside `just mvp-check` via
   `just benchmark-regression-selfcheck` before benchmark regression check.
6. Truth-first report consistency can be checked explicitly:
   `just truth-contract-check`.

## 3) Full M5/M6 Verification Refresh

1. Run: `just mvp-verify-refresh`
2. Expected: exit code `0` and `docs/MVP_VERIFY.json` with:
   - `"status":"ok"`
   - `"m5_artifacts_report":true`
   - `"m6_artifacts_report":true`
   - `"release_readiness_report":true`
   - `"truth_boundary":{"lab_scope_only":true,"real_world_datapath_closed":false}`

## 4) Required Artifacts

Verify files exist and are updated:

- `docs/M5_ARTIFACTS_REPORT.md`
- `docs/M6_ARTIFACTS_REPORT.md`
- `docs/RELEASE_READINESS_REPORT.md`
- `docs/RELEASE_READINESS_REPORT_RU.md`
- `docs/RELEASE_READINESS_REPORT.json`
- `docs/MVP_SPEC_COVERAGE.md`
- `docs/MVP_VERIFY.json`
- `docs/MVP_SNAPSHOT.json`
- `docs/REPORT_PACK.md`
- `docs/REPORT_PACK.json`
- `docs/CEF_GAP_MAP_2026-05-18.md`
- `docs/CEF_TRACK_REPORT.json`
- `docs/ARTIFACT_AUDIT.json`
- `docs/BENCHMARK_REGRESSION_GATE.json`
- `docs/RUNTIME_APPLY_DNS_SMOKE.json`
- `docs/RUNTIME_APPLY_ROUTE_SMOKE.json`
- `docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json`
- `docs/benchmark_baseline.json`

## 5) Baseline Integrity Check

1. Run: `just baseline-verify`
2. Expected: all files return `OK` from `sha256sum -c`.
3. If artifacts were intentionally refreshed, run: `just baseline-freeze`, then re-run `just baseline-verify`.

## 6) Config Parser Hardening Expectations (M6)

`config-smoke` includes negative parser smoke and must reject:

- unknown client config keys;
- unknown gateway config keys;
- duplicate keys;
- malformed `key=value` lines (missing `=`).

## 7) Safety Contract

Default generated validation reports must keep:

- `"network_state":"not_modified"`

Runtime apply smoke must additionally prove:

- `"network_state":"modified"` in `docs/RUNTIME_APPLY_DNS_SMOKE.json`;
- `"rollback_ok":true` in `docs/RUNTIME_APPLY_DNS_SMOKE.json`.
- `"network_state":"modified"` in `docs/RUNTIME_APPLY_ROUTE_SMOKE.json`;
- `"apply_attempt_ok":true` in `docs/RUNTIME_APPLY_ROUTE_SMOKE.json`;
- `"rollback_ok":true` in `docs/RUNTIME_APPLY_ROUTE_SMOKE.json`.
- `"runtime_apply_dns_verified":true` in `docs/RELEASE_READINESS_REPORT.json` (`release_gate`);
- `"runtime_apply_route_verified":true` in `docs/RELEASE_READINESS_REPORT.json` (`release_gate`).
- `"runtime_route_policy_validation_verified":true` in `docs/RELEASE_READINESS_REPORT.json` (`release_gate`);
- `"runtime_tun_name_validation_verified":true` in `docs/RELEASE_READINESS_REPORT.json` (`release_gate`).
- `"runtime_forced_stop_rollback_verified":true` in `docs/RELEASE_READINESS_REPORT.json` (`release_gate`).
- `"truth_boundary":{"lab_scope_only":true,"real_world_datapath_closed":false}` in:
  `docs/RELEASE_READINESS_REPORT.json`, `docs/MVP_SNAPSHOT.json`, `docs/MVP_VERIFY.json`.
- `"artifacts_fresh":true` in `docs/SHIP_READINESS_REPORT.json`.
- `"freshness_check":true` in `docs/SHIP_READINESS_REPORT.json` (`steps`).
- `"release_ok_lab_only":true` in `docs/SHIP_READINESS_REPORT.json`.
- `"cef_phase1_smoke_ok":true` in `docs/SHIP_READINESS_REPORT.json`.
- `"cef_gap_map_guard":true` in `docs/SHIP_READINESS_REPORT.json` (`steps`).
- `"benchmark_regression_gate":true` in `docs/SHIP_READINESS_REPORT.json` (`steps`).
- `"report_pack_json":true` in `docs/SHIP_READINESS_REPORT.json` (`steps`).
- `"cef_phase1_smoke":true` in `docs/SHIP_READINESS_REPORT.json` (`steps`).
- `"cef_phase1_smoke":true` in `docs/RELEASE_READINESS_REPORT.json` (`artifacts`).
- `"cef_phase1_smoke":true` in `docs/REPORT_PACK.json`.
- `"status":"ok"` in `docs/BENCHMARK_REGRESSION_GATE.json`.

No command in this checklist should change:

- OS routes;
- OS DNS;
- firewall (`iptables`/`nftables`);
- system proxy settings;
- router/VPS configuration.
