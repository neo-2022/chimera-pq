# Ship Readiness Report

Status: **PASS**
Generated at (UTC): `2026-05-21T14:43:08Z`

Checks:
- Baseline freeze: `true`
- Clean-room handoff check: `true`
- CEF track report: `true`
- CEF track guard: `true`
- CEF track sync guard: `true`
- Benchmark regression gate: `true`
- CEF gap map guard: `true`
- CEF consistency guard: `true`
- Mesh auto smoke: `true`
- Mesh auto adaptive trace guard: `true`
- Mesh CLI recovery schema guard selfcheck: `true`
- Mesh CLI recovery schema guard: `true`
- Release readiness JSON: `true`
- Release readiness RU markdown: `true`
- Report pack JSON: `true`
- Report pack markdown: `true`
- Release gate (`release_ok`): `true`
- Release gate is lab-only (`release_ok_lab_only`): `true`
- CEF phase1 smoke: `true`
- CEF phase1 closed: `true`
- Mesh route explain: `true`
- Mesh auto adaptive trace: `true`
- Network state unchanged: `true`
- Runtime apply smoke (`modified + rollback`): `true`
- Runtime route smoke (`modified + rollback`): `true`
- Runtime route existing-TUN smoke (`modified + rollback`): `true`
- Runtime route multi-CIDR smoke (`modified + rollback`): `true`
- Runtime route policy validation smoke (`reject invalid + no state`): `true`
- Runtime route duplicate-CIDR validation smoke (`reject invalid + no state`): `true`
- Runtime TUN-name validation smoke (`reject invalid + no state`): `true`
- Runtime resolv.conf validation smoke (`reject invalid + no state`): `true`
- Runtime datapath multiflow smoke (`gateway/direct/block explain`): `true`
- Runtime policy precedence smoke (`exact>suffix + dns-binding`): `true`
- Runtime forced-stop rollback smoke (`recover without graceful down`): `true`
- Runtime probe-access smoke (`batch targets + totals/threshold report`): `true`
- Runtime real-world probe smoke (`direct/proxy snapshot only`): `true`
- Runtime real-world direct probe ok: `true`
- Runtime real-world proxy listener detected: `false`
- Runtime real-world proxy probe attempted: `false`
- Runtime real-world proxy probe ok: `false`
- Runtime real-world proxy selected from candidates: `false`
- Runtime real-world proxy candidates: `socks5h://127.0.0.1:11080,http://127.0.0.1:1080`
- Runtime real-world proxy probe error: `proxy_listener_not_found`
- Runtime real-world proxy blocked targets total: `0`
- Runtime real-world proxy blocked targets ok: `0`
- Runtime real-world proxy blocked targets failed: `0`
- Runtime real-world skipped no curl: `false`
- Runtime real-world skipped no proxy listener: `true`
- Artifacts refreshed in this run: `true`

Truth boundary:
- Lab/proof/report contour only: `true`
- Real OS-level datapath closure (strict M4/M5): `false`

Artifacts:
- `docs/SHIP_READINESS_REPORT.json`
- `docs/RELEASE_READINESS_REPORT.json`
- `docs/RELEASE_READINESS_REPORT_RU.md`
- `docs/REPORT_PACK.json`
- `docs/REPORT_PACK.md`
- `docs/CEF_GAP_MAP_2026-05-18.md`
- `docs/CEF_PHASE1_SMOKE.json`
- `docs/BENCHMARK_REGRESSION_GATE.json`
- `docs/benchmark_baseline.json`
- `docs/benchmark_latest.json`
- `docs/RUNTIME_APPLY_DNS_SMOKE.json`
- `docs/RUNTIME_APPLY_ROUTE_SMOKE.json`
- `docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json`
- `docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json`
- `docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json`
- `docs/RUNTIME_ROUTE_DUPLICATE_CIDR_VALIDATION_SMOKE.json`
- `docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json`
- `docs/RUNTIME_RESOLV_CONF_VALIDATION_SMOKE.json`
- `docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json`
- `docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json`
- `docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json`
- `docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json`
- `docs/probe_access_latest.json`
- `docs/REALITY_AUDIT_LATEST.json`
- `docs/SECOND_MACHINE_REPORT.md`
- `docs/V1_MVP_BASELINE_MANIFEST.json`
- `docs/V1_MVP_BASELINE.sha256`
