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

1. Uses the normal process network path without `--proxy` or app proxy flags.
2. Captures a direct baseline probe and transparent datapath target probes.
3. Emits compact per-target result rows.
4. Produces explicit pass/fail reason fields (`status`, `reason`, per-row reasons).
5. Writes JSON artifact to `docs/CHIMERA_PATH_PROOF.json` (or custom output path).

Key env overrides:

- `CHIMERA_PATH_PROOF_IP_CHECK_URL`
- `CHIMERA_PATH_PROOF_TARGETS_CSV`
- `CHIMERA_PATH_PROOF_DIRECT_URL`
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

- CHIMERA transparent runtime status;
- transparent datapath proof status;
- selective routing inventory (`app_routes_count`, `service_routes_count`);
- system default-path class (`regular_interface` or `tunnel_path`).

Parallel WEAVE isolation:

- CHIMERA must not hijack already-used local WEAVE ports.
- CHIMERA runtime uses the transparent TUN path and does not require a
  user-facing proxy-port selection.
- Selected upstream settings are persisted to the configured bootstrap state
  for transport setup and reused by runtime.

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
third-party installation from required flow.

## Stand Install/Update Contract

Laptop/VPS stand verification must use the published GitHub one-command path.
Source-tree installs are development-only and do not prove that the shipped
CHIMERA release works.

Required stand flow:

1. Publish the fixed build as a GitHub Release and verify that
   `releases/latest` points to that version.
2. Run the GitHub bootstrap command on the stand over SSH.
3. The bootstrap downloads the latest release bundle, verifies version/checksum,
   installs ready binaries, and runs the normal installer.
4. Before `start`, `restart`, or peer connect, CHIMERA checks for a newer
   version first.
5. If a newer version is found, the order is always:
   `update -> verify installed version/checksum -> start/connect`.

Canonical stand command:

```bash
curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60 https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh | bash -s -- -install
```

The outer GitHub bootstrap download must keep these timeout/retry bounds.
Without them a network stall can hang before the published installer gets a
chance to run its own bounded downloads and checksum checks.

Required latest release assets:

- `chimera.sh` - public bootstrap script with `VERSION`,
  `ARCHIVE_URL_DEFAULT`, and `CHECKSUM_URL_DEFAULT` metadata;
- `chimera-pq-release.tar.gz` - release bundle with ready binaries under `bin/`;
- `chimera-pq-release.tar.gz.sha256` - checksum file for the bundle.

The published `chimera.sh` bootstrap must download the bundle from
`GitHub Release/Latest`, verify the checksum before extraction, install ready
binaries, write installed version/checksum metadata, and then run the normal
installer. Installed `chimera-sh -start`, `chimera-sh -restart`,
`chimera-sh -mesh ...`, and `chimera-sh -connect ...` must perform the
update-first check before starting or connecting.

Peer update fallback:

- GitHub Release/Latest remains the primary source for first install and stand
  proof.
- If GitHub is unavailable during `start`, `restart`, `mesh`, or `connect`,
  an already-installed CHIMERA can try trusted peer bootstrap URLs from:
  - `CHIMERA_UPDATE_PEER_BOOTSTRAP_URLS`
  - `${XDG_CONFIG_HOME:-$HOME/.config}/chimera/update_peer_bootstrap_urls.list`
  For `chimera-sh -connect <peer>`, peer fallback is narrowed to the
  `update_bootstrap_url` of the selected peer. The general peer list is not a
  silent substitute for a different selected peer.
- Each peer entry may be the peer base URL, `/metadata.json`, `/chimera.sh`,
  `/chimera-pq-release.tar.gz`, or `/chimera-pq-release.tar.gz.sha256`;
  the launcher normalizes it to the peer metadata endpoint. The peer bootstrap
  script is not executed or parsed as a trust source during peer fallback.
- A peer source is used only if `/metadata.json` reports
  `kind=chimera_peer_update_metadata`, `status=ok`, a newer semver version, the
  canonical same-origin archive/checksum URLs, and a 64-hex `sha256` value.
  The update still downloads the archive and checksum, verifies that metadata
  sha equals the checksum file before extraction, writes installed
  version/checksum metadata, and re-runs the original command after install.
- Peer fallback is allowed only when GitHub Latest is unreachable. If GitHub
  responds with invalid metadata, bad version, invalid checksum, or an
  inconsistent source, CHIMERA fails closed and does not try a peer substitute.
- If GitHub and all trusted peers are unreachable, CHIMERA keeps the installed
  version and emits `chimera_update=unavailable`; network outage alone is not
  a release block.
- Peer update is update-only fallback. It is not acceptable evidence for the
  GitHub one-command first-install stand proof.
- Checksum verification gives bundle integrity. Peer-source provenance must be
  bounded operationally by a trusted peer list until signed release manifests
  are added.

Start contract:

- `chimera-sh -start` prepares user-cache log targets before the systemd user
  start path, so `StandardOutput=append:%h/.cache/chimera/...` does not fail on
  a missing log file or missing cache directory.
- If either `chimera-gateway.service` or the transparent runtime service fails
  its active check, `chimera-sh -start` must return non-zero and report
  `start_status=fail` with the matching failure reason.

Serving a release from an already-installed CHIMERA:

```bash
chimera-bootstrap serve-release \
  --root "${CHIMERA_HOME:-$HOME/.local/share/chimera}" \
  --listen 0.0.0.0:18179 \
  --base-url http://node.example:18179
```

The peer server exposes:

- `/metadata.json`
- `/chimera.sh`
- `/chimera-pq-release.tar.gz`
- `/chimera-pq-release.tar.gz.sha256`

The shipped peer-egress role is `node`. `client`, `gateway`, `server`, `vps`,
and `laptop` remain compatibility labels only.

Forbidden for laptop/VPS stand proof:

- `cargo build`, `cargo run`, or requiring `cargo` on the target;
- `git clone` as the install source on the target;
- `rsync`, `scp`, local tarballs, `target/`, or a local working tree as the
  installed artifact;
- manual replacement of runtime files.

Scripts that accept a local directory or local tarball are allowed only for
development/debug packaging checks. They are not acceptable evidence for
real-world stand verification. Local release sources require explicit
`CHIMERA_ALLOW_LOCAL_RELEASE_SOURCE=1`, and local tarballs still require a
matching checksum file before extraction.

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

If path proof fails:

1. Start CHIMERA control path:
   `bash scripts/chimera-control.sh start`
2. Check transparent datapath status:
   `bash scripts/chimera-control.sh datapath-status`
3. Re-run:
   `just chimera-runtime-verify`

## Selective App/Service Routing

CHIMERA control supports selective routing for apps/services through the transparent CHIMERA datapath without forcing whole-device traffic.

1. Create config from example:
   `cp configs/chimera-app-routes.example.conf configs/chimera-app-routes.conf`
2. Inspect parsed config:
   `bash scripts/chimera-control.sh app-routes-status`
3. Run a selected app normally through the configured route:
   `bash scripts/chimera-control.sh run-app telegram`
4. Enable route override for selected user services:
   `bash scripts/chimera-control.sh service-route-enable`
5. Disable route override for selected user services:
   `bash scripts/chimera-control.sh service-route-disable`

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

- `SITE_FAILOVER_DATAPATH_THRESHOLD` (`1` default): consecutive datapath successes
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

- transparent datapath mode;
- direct baseline result;
- per-target datapath result with explicit pass/fail fields.

Selfcheck:

- `just chimera-path-proof-selfcheck`

## App/Service Selective Routing

`scripts/chimera-control.sh` supports selective routing through the transparent
datapath without changing per-app proxy settings.

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
3. Run one app through the configured route:
   `scripts/chimera-control.sh run-app <app_id> [args...]`
4. Show live datapath + upstream state:
   `scripts/chimera-control.sh datapath-status`
5. Show routing + upstream sticky/degrade state:
   `scripts/chimera-control.sh route-status`
6. Probe upstream endpoint pool and best candidate:
   `scripts/chimera-control.sh upstream-probe`
7. Show compact upstream health+history audit:
   `scripts/chimera-control.sh upstream-audit 30`
8. Run upstream resilience smoke and write JSON artifact:
   `just upstream-resilience-smoke`
9. Enable route override for configured user services:
   `scripts/chimera-control.sh service-route-enable`
10. Enable route override for one service:
   `scripts/chimera-control.sh service-route-enable <service_name>`
11. Disable route override for configured user services:
   `scripts/chimera-control.sh service-route-disable`

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
- installer bootstraps upstream settings from the configured CHIMERA bootstrap state;
- control/runtime consume the same upstream bootstrap state from that file.
