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
5. `./scripts/chimera-control.sh status`
6. `./scripts/chimera-control.sh datapath-status`
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

WEAVE node config validation check (no network changes):

1. prepare a local mesh-node config file;
2. run the safe config/doctor checks for the node/datapath workflow;
3. ensure output contains `config accepted` and safety line.

Expected result:

- all commands exit with code `0`;
- doctor reports are refreshed in `docs/doctor_latest.json` and
  `docs/lab_doctor_latest.json`;
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

Performance work that changes hot metadata, lane selection, or rebuild policy
must follow `docs/PERFORMANCE.md`. Keep the optimization scope on control
metadata only; sealed payload stays opaque.

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

`just chimera-path-proof` is an external reachability snapshot. By itself it is
not release evidence and not proof that traffic crossed the CHIMERA/WEAVE
datapath.

Behavior:

1. Uses the normal process network path without `--proxy` or app proxy flags.
2. Captures a direct baseline probe and external target probes.
3. Emits compact per-target result rows.
4. Produces explicit pass/fail reason fields (`status`, `reason`, per-row reasons).
5. Writes JSON artifact to `docs/CHIMERA_PATH_PROOF.json` (or custom output path).

Gate rule:

- release/runtime aggregators must not accept `status=pass` alone;
- CHIMERA datapath proof requires `mode=chimera_transparent_datapath`,
  `chimera_datapath_evidence=true`, `datapath.attempted=true`,
  `datapath.ok=true`, at least one datapath target, and zero failed datapath
  targets;
- if the artifact says `external_reachability_without_system_proxy`, it remains
  ordinary external reachability only.

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
  user-facing per-app port selection.
- Selected mesh endpoint settings are persisted to private bootstrap state for
  transport setup and reused by runtime.

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

## Self-Contained Transparent Runtime

CHIMERA release must use the shipped CHIMERA binaries for the normal
transparent mesh path:

1. `chimera.sh -install` installs the CHIMERA release and prepares runtime
   config without downloading third-party network runtime binaries.
2. `chimera-control.sh start` always starts the CHIMERA node service from the
   shipped release tree, but transparent datapath starts only when a real peer
   endpoint is already configured and the node is not in bootstrap-only
   `listener_only` mode.
3. A missing first-party transparent datapath is a fail-closed condition. It
   must not be silently replaced with a third-party runtime bootstrap.

Release and install gates must reject `chimera_runtime_bootstrap.sh` in the
normal bundle. Historical third-party bootstrap scripts are not release
evidence and are not allowed to prove invisible app UX.

## External Proof Install/Update Contract

External proof-node verification must use the published GitHub one-command
path. Source-tree installs are development-only and do not prove that the
shipped CHIMERA release works.

Required external proof flow:

1. Publish the fixed build as a GitHub Release and verify that
   `releases/latest` points to that version.
2. Run the GitHub bootstrap command on each authorized proof node over SSH.
3. The bootstrap downloads the latest release bundle, verifies version/checksum,
   installs ready binaries, and runs the normal installer.
4. Before `start`, `restart`, or peer connect, CHIMERA checks for a newer
   version first.
5. If a newer version is found, the order is always:
   `update -> verify installed version/checksum -> start/connect`.

Canonical remote proof command:

```bash
bash -o pipefail -c 'curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60 https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh | bash -s -- -install'
```

The outer GitHub bootstrap download must keep these timeout/retry bounds.
Without them a network stall can hang before the published installer gets a
chance to run its own bounded downloads and checksum checks. The outer shell
must also enable `pipefail`; otherwise a failed bootstrap download can be
masked by an empty `bash` process on the right side of the pipe.

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

- GitHub Release/Latest remains the primary source for first install and remote
  proof.
- Gitvers is the secondary public bootstrap source for installed CHIMERA and
  operator-driven reinstall/update when GitHub is unreachable.
- Gitvers does not have its own product version stream. It serves the same
  CHIMERA release version (`VERSION` plus matching archive/checksum) as the
  bundle published for that release.
- Gitvers bootstrap mirrors are configured through:
  - `CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URL`
  - `CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URLS`
  - `${XDG_CONFIG_HOME:-$HOME/.config}/chimera/update_gitvers_bootstrap_urls.list`
- A Gitvers entry is treated as a public bootstrap source. CHIMERA normalizes
  the URL to the bootstrap script path, including GitVerse repo-root and
  GitVerse viewer URLs such as
  `https://gitverse.ru/<owner>/<repo>` and
  `https://gitverse.ru/<owner>/<repo>/content/main/chimera.sh`, which are
  converted to
  `https://gitverse.ru/api/repos/<owner>/<repo>/raw/branch/main/chimera.sh`.
  CHIMERA then reads release version/archive/checksum metadata from that
  bootstrap script and verifies the downloaded archive checksum before
  extraction.
- After a successful GitHub publish, refresh trusted peer mirror nodes to
  the same release tree and serve them with `chimera-bootstrap serve-release`
  so peers can update from an already-installed Chimera when GitHub is
  unreachable.
- See [UPDATE_SOURCE_DECISION_MATRIX.md](UPDATE_SOURCE_DECISION_MATRIX.md) for
  the normative runtime matrix.
- During `start`, `restart`, `mesh`, or `connect`, CHIMERA walks the trust
  ladder `GitHub -> Gitvers -> peer`.
- A source becomes authoritative only after CHIMERA verifies a full release
  tuple: `version + archive checksum`.
- If GitHub is unreachable before a verified tuple exists, CHIMERA may try
  Gitvers. If GitHub is invalid or inconsistent, CHIMERA fails closed and does
  not try a lower-trust substitute.
- If GitHub yields a verified current or stale tuple, CHIMERA emits
  `chimera_update=no_newer_release`, keeps the installed release, and does not
  let Gitvers or peer outrun GitHub in the same round.
- If GitHub yields a verified newer tuple, CHIMERA installs that release from
  GitHub.
- If GitHub yielded a verified newer tuple but the archive delivery/install
  path was unavailable, CHIMERA may try a lower-trust mirror only for the exact
  same `{version, sha256}` tuple. A different version blocks with
  `trusted_version_divergence`; the same version with a different checksum
  blocks with `trusted_checksum_divergence`.
- If GitHub is unreachable and Gitvers yields a verified current or stale
  tuple, CHIMERA emits `chimera_update=no_newer_release` and does not let peer
  outrun Gitvers in the same round.
- If GitHub is unreachable and Gitvers yields a verified newer tuple, CHIMERA
  installs that release from Gitvers.
- If GitHub and Gitvers are unreachable before a verified tuple exists during
  `start`, `restart`, `mesh`, or `connect`, an already-installed CHIMERA can
  try trusted peer bootstrap URLs from:
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
  `kind=chimera_peer_update_metadata`, `status=ok`, a semver version that is
  either newer than the installed release or matches an already verified
  higher-trust tuple, the canonical same-origin archive/checksum URLs, and a
  64-hex `sha256` value. The update still downloads the archive and checksum,
  verifies that metadata sha equals the checksum file before extraction, writes
  installed version/checksum metadata, and re-runs the original command after
  install.
- Peer fallback is allowed only when higher-trust sources have not yet produced
  a verified tuple, or when a higher-trust verified newer tuple needs mirror
  delivery of the exact same `{version, sha256}` because the original archive
  delivery/install path was unavailable.
- If GitHub, Gitvers, and all trusted peers are unreachable, CHIMERA keeps the
  installed version and emits `chimera_update=unavailable`; network outage
  alone is not a release block.
- If one or more configured sources are reachable and valid but none of them is
  newer than the installed version, CHIMERA emits
  `chimera_update=no_newer_release` and continues without downgrade.
- Peer update is update-only fallback. It is not acceptable evidence for the
  GitHub one-command first-install external proof.
- Checksum verification gives bundle integrity. Peer-source provenance must be
  bounded operationally by a trusted peer list until signed release manifests
  are added.

Start contract:

- `chimera-sh -start` prepares user-cache log targets before the systemd user
  start path, so `StandardOutput=append:%h/.cache/chimera/...` does not fail on
  a missing log file or missing cache directory.
- If either `chimera-node.service` or `chimera-datapath.service` fails
  its active check, `chimera-sh -start` must return non-zero and report
  `start_status=fail` with the matching failure reason.

Serving a release from an already-installed CHIMERA:

```bash
chimera-bootstrap serve-release \
  --root "${CHIMERA_HOME:-$HOME/.local/share/chimera}" \
  --listen 0.0.0.0:0 \
  --base-url http://node.example \
  --state-file "${XDG_CACHE_HOME:-$HOME/.cache}/chimera/peer-update.state.json"
```

The peer server exposes:

- `/metadata.json`
- `/chimera.sh`
- `/chimera-pq-release.tar.gz`
- `/chimera-pq-release.tar.gz.sha256`
- `/mesh_nodes.discovery.json` when the current node has already published a signed discovery snapshot in its cache directory
- `/mesh_nodes.discovery.pubkey` when the current node has already published the paired discovery public key in its cache directory

When `--listen` uses port `0`, CHIMERA asks the OS for a free port, records the
selected `listen` and `update_bootstrap_url` in the private state file, and can
publish that URL through `chimera mesh nodes advertise --update-state-file`.
If `--base-url` has no port or uses `:0`, the selected port is inserted into
peer metadata and the generated peer bootstrap script. A fixed `--listen` port
is an explicit operator override only; it is not the normal workflow.

The state file is private operator/runtime state. Public proof reports must use
redacted markers such as `peer_update_state=present`, `version_ok=true`, and
`checksum_ok=true`, not raw host names, ports, paths or stand addresses.
When a peer update is fronted by a relay, the private `listen` bind and the
public `update_bootstrap_url` may differ; advertising must use the published
public origin, not assume both ports are the same.

The shipped peer-egress role is `node`. `client`, `gateway`, `server`, `side_a`,
and `side_b` remain compatibility labels only.

Forbidden for external remote proof:

- `cargo build`, `cargo run`, or requiring `cargo` on the target;
- `git clone` as the install source on the target;
- `rsync`, `scp`, local tarballs, `target/`, or a local working tree as the
  installed artifact;
- manual replacement of runtime files.

Scripts that accept a local directory or local tarball are allowed only for
development/debug packaging checks. They are not acceptable evidence for
real-world external proof verification. Local release sources require explicit
`CHIMERA_ALLOW_LOCAL_RELEASE_SOURCE=1`, and local tarballs still require a
matching checksum file before extraction.

## Mesh Node Selection Flow

Mesh node discovery is not baked into installation. On a fresh install the
available node list is loaded from the upstream/bootstrap source at runtime.
If no endpoint is configured yet, the first start/install path opens node
selection and then resolves the chosen node endpoint automatically.
If no trusted bootstrap source is present yet, the first start may still bring
the node up in listener-only mode so it can bind and publish its own ingress
endpoint, but transparent datapath and doctor stay fail-closed until a real
peer endpoint is selected or materialized from trusted bootstrap data.

Node-role installs now default `CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true`
for symmetric WEAVE behavior, while `CHIMERA_PEER_EGRESS_ALLOW_POOL_TRANSIT`
remains `false`. This does not make the node mesh-ready by itself: live bound
transit still requires authoritative mesh context, policy, peers and active
lane bindings. Operators that need to disable bound transit must set
`CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=false` explicitly before install/update
or change `peer-egress.env` after install.

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
3. `just chimera-ops-gate-fresh` (same checks + forced fresh external
   proof-node load run + unified fresh report)
4. `just chimera-side-b-fresh-gate-sync` (legacy lab task; run fresh gate on the
   configured external proof node and sync artifacts locally)

This runs:

- path-proof selfcheck;
- channel-audit selfcheck;
- e2e gate selfcheck;
- external proof-node load-gate selfcheck;
- runtime verify;
- end-to-end channel gate.
- e2e gate artifact guard (`status`, contract fields, freshness).
- external proof-node load gate (`status`, min success-rate, min request volume).
- unified fresh gate report (`docs/CHIMERA_FRESH_GATE_REPORT.json` + `.md`).

Direct guard run:

1. `just chimera-e2e-channel-gate-guard`

External proof-node real-world load run (parallel, default 300s):

1. `just chimera-load-side-b`
2. selfcheck only: `just chimera-load-side-b-selfcheck`
3. strict gate from latest load artifact: `just chimera-load-gate-side-b`
4. gate selfcheck only: `just chimera-load-gate-side-b-selfcheck`

Proof-node host/user/path values must live in private operator notes or an
external shell environment. Do not copy concrete external proof-machine details
into CHIMERA product docs, release artifacts, defaults, or source-controlled
config.

Optional load/gate env overrides:
- `CHIMERA_LOAD_DURATION_SEC`
- `CHIMERA_LOAD_TIMEOUT_SEC`
- `CHIMERA_LOAD_CONNECT_TIMEOUT_SEC`
- `CHIMERA_LOAD_GATE_MIN_SUCCESS_RATE` (default `0.95`)
- `CHIMERA_LOAD_GATE_MIN_TOTAL_REQUESTS` (default `100`)
- `CHIMERA_LOAD_GATE_MAX_AGE_SEC` (default `3600`)
- `CHIMERA_LOAD_GATE_FORCE_FRESH` (`1` = always run a new external proof-node
  load before gate)

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
- legacy compatibility seed file (`configs/manual_gateway_domains.txt`)
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
