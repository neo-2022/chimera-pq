# Operations

Default M0/M1 command paths are safe local commands only and do not change OS networking.
Runtime paths with explicit `chimera up --apply-*` flags can modify local
routes/DNS/TUN and must be used only with rollback plan and explicit user approval.

## MVP Validation Runbook

Use this quick sequence before sharing build artifacts:

1. `just mvp-check`

Manual expanded sequence (same checks, step-by-step):

1. `just check`
2. `just test`
3. `just lint`
4. `just deny`
5. `just client-doctor`
6. `just gateway-doctor`
7. `just lab-doctor`
8. `just mvp-spec-check`
9. `just hardening-smoke`
10. `just benchmark-regression-check`
11. `just json-message-contract-check`
12. `just rollback-json-contract-check`
13. `just runtime-apply-dns-smoke`
14. `just runtime-apply-route-smoke-selfcheck`
15. `just runtime-apply-route-smoke`
16. `just ship-readiness-selfcheck`
17. `just cleanroom-handoff-selfcheck`
18. `just truth-contract-check`

Gateway config validation check (no network changes):

1. prepare a local gateway config file;
2. run `cargo run -p chimera-gateway -- run --config <file>`;
3. ensure output contains `config accepted` and safety line.

Expected result:

- all commands exit with code `0`;
- doctor reports are refreshed in `docs/doctor_latest.json`,
  `docs/gateway_doctor_latest.json`, `docs/lab_doctor_latest.json`;
- release-readiness reports are refreshed in:
  `docs/RELEASE_READINESS_REPORT.md`, `docs/RELEASE_READINESS_REPORT_RU.md`, `docs/RELEASE_READINESS_REPORT.json`;
- `config-smoke` covers both positive config parsing and negative parser smoke
  for unknown keys, duplicate keys and malformed `key=value` lines;
- doctor JSON reports include simple bilingual fields `message_en` and `message_ru`;
- `docs/benchmark_latest.json` is updated;
- default validation commands do not modify OS routes/DNS/firewall/proxy settings.
- explicit runtime apply flags (`--apply-tun`, `--apply-route`, `--apply-dns`)
  are outside this default validation path and must be rollback-tested.

If `benchmark-regression-check` fails:

1. ensure script selfcheck is green: `just benchmark-regression-selfcheck`;
2. re-run once to exclude short-term machine noise (the gate also retries once automatically);
3. if it fails again, keep the previous baseline file and investigate recent
   performance-sensitive changes;
4. only update baseline after confirming the regression is acceptable.

## Quick Ops (RU/EN)

English (simple):

1. Build and test: `just mvp-check`
2. One-command MVP check via CLI: `cargo run -p chimera-cli -- mvp-check`
3. Fast smoke only: `cargo run -p chimera-cli -- lab-smoke`
4. Refresh and verify all MVP artifacts:
   `cargo run -p chimera-cli -- mvp-verify --refresh --json --out docs/MVP_VERIFY.json`
5. Hardening check: `cargo run -p chimera-cli -- hardening-smoke`
6. Russian text output example:
   `cargo run -p chimera-cli -- --lang ru mvp-verify --text --out docs/MVP_VERIFY.txt`

Русский (просто):

1. Сборка и тесты: `just mvp-check`
2. MVP одной командой через CLI: `cargo run -p chimera-cli -- mvp-check`
3. Только быстрая проверка: `cargo run -p chimera-cli -- lab-smoke`
4. Обновить и проверить все артефакты MVP:
   `cargo run -p chimera-cli -- mvp-verify --refresh --json --out docs/MVP_VERIFY.json`
5. Проверка надежности: `cargo run -p chimera-cli -- hardening-smoke`
6. Пример текстового вывода на русском:
   `cargo run -p chimera-cli -- --lang ru mvp-verify --text --out docs/MVP_VERIFY.txt`

## CHIMERA Path Proof

Use `just chimera-path-proof` to generate direct-vs-CHIMERA path evidence.

Behavior:

1. Checks local listener availability for CHIMERA proxy candidates.
2. Captures observed public IP for direct path and CHIMERA path.
3. Probes each target through both paths and emits compact per-target result rows.
4. Produces explicit pass/fail reason fields (`status`, `reason`, per-row reasons).
5. Writes JSON artifact to `docs/CHIMERA_PATH_PROOF.json` (or custom output path).

Key env overrides:

- `CHIMERA_PATH_PROOF_IP_CHECK_URL`
- `CHIMERA_PATH_PROOF_TARGETS_CSV`
- `CHIMERA_PATH_PROOF_PROXY_CANDIDATES`
- `CHIMERA_PATH_PROOF_TIMEOUT_SEC`
- `CHIMERA_PATH_PROOF_JSON_OUT`

Selfcheck:

- `just chimera-path-proof-selfcheck`

## CHIMERA Channel Audit

Use channel audit when you must prove traffic separation with selected
apps/services (and possible parallel WEAVEs on the same host).

1. Run audit:
   `just chimera-channel-audit`
2. Read artifact:
   `docs/CHIMERA_CHANNEL_AUDIT.json`

Report includes:

- CHIMERA proxy listener status;
- direct vs CHIMERA path proof status and observed public IP split;
- selective routing inventory (`app_routes_count`, `service_routes_count`);
- system default-path class (`regular_interface` or `tunnel_path`).

Parallel WEAVE isolation:

- CHIMERA must not hijack already-used local WEAVE ports.
- CHIMERA runtime uses the transparent TUN path and does not require a
  user-facing proxy-port selection.
- Selected upstream settings are persisted to `~/.config/chimera/upstream_proxy.env`
  for transport bootstrap and reused by runtime.

Selfcheck:

- `just chimera-channel-audit-selfcheck`

## One-Command Runtime Verification

For practical runtime verification (start + routing status + path proof):

1. `just chimera-runtime-verify`

This flow:

- starts CHIMERA control path;
- prints selective routing status (`route-status`);
- runs path proof and writes `docs/CHIMERA_PATH_PROOF.json`;
- runs channel audit and writes `docs/CHIMERA_CHANNEL_AUDIT.json`;
- prints compact summary fields from the JSON artifact.

## Self-Contained Runtime Bootstrap

CHIMERA runtime now includes automatic dependency bootstrap for split transparent
mode:

1. `chimera.sh -install` executes runtime bootstrap (`chimera_runtime_bootstrap.sh`)
   with no manual user action.
2. `chimera-control.sh start` auto-checks split runtime and bootstraps missing
   component if needed.
3. Runtime binary is placed under:
   `${XDG_DATA_HOME:-$HOME/.local/share}/chimera-pq/runtime/singbox/sing-box`
4. Operator can pin version/checksum via env:
   - `CHIMERA_SINGBOX_VERSION`
   - `CHIMERA_SINGBOX_URL`
   - `CHIMERA_SINGBOX_SHA256`

This keeps install/start one-command for end users and removes manual

## Mesh Node Selection Flow

Mesh node discovery is not baked into installation. On a fresh install the
available node list is loaded from the upstream/bootstrap source at runtime.
If no endpoint is configured yet, the first start/install path opens node
selection and then resolves the chosen node endpoint automatically.

Operator flow:

1. Start CHIMERA normally or let install open the selection prompt.
2. Open `chimera mesh nodes select`.
3. Choose one node from the loaded list manually on the first run.
4. After that CHIMERA persists `current`, `pinned`, and `autoconnect`.
5. Per-resource route selection and automatic fallback are handled by
   `site_auto_watch` and the adaptive split-routing path, not by changing the
   user-selected mesh node.

Important:

- install does not generate a baked node inventory;
- first selection is user-visible and manual;
- subsequent route changes are automatic and hidden from the user when
  runtime conditions change;
- the selected mesh node stays pinned until the user changes it manually.
third-party installation from required flow.

## End-to-End Channel Gate

For one-command operator evidence (runtime + channel + selected app/service routing):

1. `just chimera-e2e-channel-gate`

Artifact:

- `docs/CHIMERA_E2E_CHANNEL_GATE.json`

Gate requires:

- `path_proof.status=pass`;
- `channel_audit.status=pass`;
- `run-app curl_example` succeeds;
- service override check for `example.service` is confirmed as `enabled`.

Selfcheck:

- `just chimera-e2e-channel-gate-selfcheck`

Team gate (single command):

1. `just chimera-ops-gate`
2. `just chimera-ops-gate-quiet` (same checks with reduced console noise)
3. `just chimera-ops-gate-fresh` (same checks + forced fresh laptop load run + unified fresh report)
4. `just chimera-laptop-fresh-gate-sync` (run fresh gate on laptop and sync artifacts locally)

This runs:

- path-proof selfcheck;
- channel-audit selfcheck;
- e2e gate selfcheck;
- laptop load-gate selfcheck;
- runtime verify;
- end-to-end channel gate.
- e2e gate artifact guard (`status`, contract fields, freshness).
- laptop load gate (`status`, min success-rate, min request volume).
- unified fresh gate report (`docs/CHIMERA_FRESH_GATE_REPORT.json` + `.md`).

Direct guard run:

1. `just chimera-e2e-channel-gate-guard`

Laptop real-world load run (parallel, default 300s):

1. `just chimera-load-laptop`
2. selfcheck only: `just chimera-load-laptop-selfcheck`
3. strict gate from latest load artifact: `just chimera-load-gate-laptop`
4. gate selfcheck only: `just chimera-load-gate-laptop-selfcheck`

Optional env overrides:

- `CHIMERA_LAPTOP_HOST`
- `CHIMERA_LAPTOP_USER`
- `CHIMERA_LAPTOP_PASS`
- `CHIMERA_LAPTOP_REPO`
- `CHIMERA_LOAD_DURATION_SEC`
- `CHIMERA_LOAD_TIMEOUT_SEC`
- `CHIMERA_LOAD_CONNECT_TIMEOUT_SEC`
- `CHIMERA_LOAD_GATE_MIN_SUCCESS_RATE` (default `0.95`)
- `CHIMERA_LOAD_GATE_MIN_TOTAL_REQUESTS` (default `100`)
- `CHIMERA_LOAD_GATE_MAX_AGE_SEC` (default `3600`)
- `CHIMERA_LOAD_GATE_FORCE_FRESH` (`1` = always run a new laptop load before gate)

Additional selfcheck for app/service routing config:

- `just chimera-app-routes-selfcheck`

If path proof reports `proxy_listener_not_found`:

1. Configure upstream channel file from template:
   `cp configs/upstream_proxy.env.example ~/.config/chimera/upstream_proxy.env`
2. Fill real upstream credentials.
3. Start CHIMERA control path:
   `bash scripts/chimera-control.sh start`
4. Re-run:
   `just chimera-runtime-verify`

## Selective App/Service Routing

CHIMERA control supports selective routing for apps/services through the CHIMERA proxy channel without forcing whole-device traffic.

1. Create config from example:
   `cp configs/chimera-app-routes.example.conf configs/chimera-app-routes.conf`
2. Inspect parsed config:
   `bash scripts/chimera-control.sh app-routes-status`
3. Run a selected app via CHIMERA proxy env:
   `bash scripts/chimera-control.sh run-app telegram`
4. Enable proxy env for selected user services:
   `bash scripts/chimera-control.sh service-proxy-enable`
5. Disable proxy env for selected user services:
   `bash scripts/chimera-control.sh service-proxy-disable`

Notes:

## Split Auto-Failover (Default)

Target operating model:

1. Keep CHIMERA in `split` mode by default.
2. Keep direct path for reachable resources.
3. Auto-switch only unreachable resources to CHIMERA path.
4. Keep other traffic direct.

Operational commands:

1. Bootstrap adaptive resource list:
   `bash scripts/chimera-control.sh site-auto-bootstrap`
2. Start adaptive background recheck:
   `bash scripts/chimera-control.sh site-auto-watch start`
3. Check adaptive DB:
   `bash scripts/chimera-control.sh site-auto-status`
4. Force one-shot DNS-driven discovery + adaptive resolve:
   `bash scripts/chimera-control.sh site-auto-discover run`
5. Check discovered domains:
   `bash scripts/chimera-control.sh site-auto-discover status`

Seed sources for bootstrap:

- `configs/auto_failover_seeds.txt`
- `configs/manual_gateway_domains.txt`
- adaptive DB (`~/.config/chimera/site_adaptive_routes.db`)
- URL domains discovered in app-routes config (`configs/chimera-app-routes.conf`)

System-wide discovery source:

- recent DNS domains from system resolver logs (`systemd-resolved` journal),
  controlled by:
  - `SITE_AUTO_DISCOVERY_ENABLED` (`1` default),
  - `SITE_AUTO_DISCOVERY_LOOKBACK_SEC` (`120` default).

Adaptive switching hysteresis:

- `SITE_FAILOVER_PROXY_THRESHOLD` (`1` default): consecutive proxy successes
  needed before switching a domain to CHIMERA path.
- `SITE_FAILBACK_DIRECT_THRESHOLD` (`3` default): consecutive direct successes
  needed before switching a domain back to direct path.
- `SITE_ADAPTIVE_ENTRY_TTL_SEC` (`86400` default): adaptive entry retention TTL.

- Existing domain-based PAC split-routing behavior remains unchanged.
- App/service mode only affects selected targets and does not force unrelated traffic.

## Traffic Path Proof

Use path proof to verify actual path evidence (not just "site opened").

1. Run proof:
   `just chimera-path-proof`
2. Read JSON artifact:
   `docs/CHIMERA_PATH_PROOF.json`

The report includes:

- proxy listener presence (`proxy_listener`);
- observed public IP direct vs via CHIMERA (`path_ip_direct`, `path_ip_via_chimera`);
- per-target direct result and via-CHIMERA result with explicit pass/fail fields.

Selfcheck:

- `just chimera-path-proof-selfcheck`

## App/Service Selective Routing

`scripts/chimera-control.sh` supports selective routing via proxy environment
variables without changing PAC domain behavior.

Config file:

- copy `configs/chimera-app-routes.example.conf` to
  `configs/chimera-app-routes.conf`;
- define:
  - `app:<id>=<command>`;
  - `app-env:<id>=KEY=VALUE;KEY2=VALUE2` (optional);
  - `service:<id>=<systemd-user-service-name>`;
  - `service-env:<id>=KEY=VALUE;KEY2=VALUE2` (optional).

Commands:

1. Introspection:
   `scripts/chimera-control.sh route-status`
2. Show parsed map only:
   `scripts/chimera-control.sh app-routes-status`
3. Run one app via CHIMERA proxy env:
   `scripts/chimera-control.sh run-app <app_id> [args...]`
4. Show live proxy + upstream state:
   `scripts/chimera-control.sh proxy-status`
5. Show routing + upstream sticky/degrade state:
   `scripts/chimera-control.sh route-status`
6. Probe upstream endpoint pool and best candidate:
   `scripts/chimera-control.sh upstream-probe`
7. Show compact upstream health+history audit:
   `scripts/chimera-control.sh upstream-audit 30`
8. Run upstream resilience smoke and write JSON artifact:
   `just upstream-resilience-smoke`
9. Enable proxy env override for configured user services:
   `scripts/chimera-control.sh service-proxy-enable`
10. Enable proxy env override for one service:
   `scripts/chimera-control.sh service-proxy-enable <service_name>`
11. Disable proxy env override for configured user services:
   `scripts/chimera-control.sh service-proxy-disable`

Self-check commands:

1. `bash -n scripts/chimera-control.sh`
2. `scripts/chimera-control.sh app-routes-status`
3. `scripts/chimera-control.sh route-status`
4. `scripts/chimera-control.sh run-app curl_example`

## Installer Gate (Parallel WEAVE Safety)

Run installer gate before release/install validation:

- `bash scripts/chimera_installer_gate.sh`

Gate guarantees:

- installer keeps CHIMERA in the transparent runtime contour;
- installer bootstraps upstream settings from `~/.config/chimera/upstream_proxy.env`;
- control/runtime consume the same upstream bootstrap state from that file.
