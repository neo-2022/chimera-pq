# CHIMERA-PQ VPN

Linux-first MVP workspace for CHIMERA-PQ VPN.

Start with the parent workspace documents:

1. `../AGENTS.md`
2. `../CHIMERA-PQ_MVP_SPEC.md`
3. `../Agent.md`

This repository intentionally starts small: client, gateway, secure session
skeleton, carrier abstraction, policy routing, DNS binding, diagnostics and
tests. CHIMERA-NOVA CEF features such as DHT, DPS, ZK complaints, relay economy
and emergency carriers are post-MVP.

Current status:

- M0 workspace and tooling are in place.
- Minimal CI is configured in `.github/workflows/ci.yml`.
- M1 fake client/gateway handshake works over an in-memory carrier.
- M2 key schedule skeleton is wired into `chimera-session` via `chimera-crypto`
  using HKDF-SHA256 with zeroized directional traffic secrets.
- Carrier and capture crates now include explicit MVP skeletons for TLS, QUIC,
  and TUN/local-proxy capture planning.
- Default command paths do not change OS routes/DNS/firewall/proxy.
- Explicit `chimera up --apply-*` flags can apply local OS-level TUN/route/DNS
  changes with rollback state.

## Commands

```bash
cargo check --workspace
cargo test --workspace
cargo deny check
cargo run -p chimera-cli -- status
cargo run -p chimera-cli -- status --config configs/client.example.conf
cargo run -p chimera-cli -- health --config configs/client.example.conf
cargo run -p chimera-cli -- doctor --config configs/client.example.conf
cargo run -p chimera-cli -- --lang ru status
cargo run -p chimera-cli -- --lang ru health --config configs/client.example.conf
cargo run -p chimera-cli -- --lang ru doctor --config configs/client.example.conf --json --out docs/doctor_latest.json
cargo run -p chimera-cli -- route explain example.org --json --out docs/route_explain_latest.json
cargo run -p chimera-lab -- datapath-report --json --out docs/datapath_latest.json
cargo run -p chimera-cli -- up --state-file docs/runtime_state_latest.json
cargo run -p chimera-cli -- up --state-file docs/runtime_state_latest.json --apply-dns true --dns-server 9.9.9.9 --resolv-conf /tmp/chimera_resolv_test.conf
cargo run -p chimera-cli -- down --state-file docs/runtime_state_latest.json
cargo run -p chimera-cli -- rollback status --state-file docs/runtime_state_latest.json
cargo run -p chimera-cli -- rollback clean --state-file docs/runtime_state_latest.json
cargo run -p chimera-cli -- rollback recover --state-file docs/runtime_state_latest.json
cargo run -p chimera-cli -- rollback status --state-file docs/runtime_state_latest.json --json --out docs/rollback_status_latest.json
cargo run -p chimera-cli -- rollback recover --state-file docs/runtime_state_latest.json --json --out docs/rollback_recover_latest.json
cargo run -p chimera-cli -- diag export --config configs/client.example.conf --age 120 --packets 1 --out docs/diag_export_latest.json
cargo run -p chimera-cli -- lab smoke
cargo run -p chimera-cli -- lab doctor --json --out docs/lab_doctor_latest.json
cargo run -p chimera-cli -- lab hardening-smoke
cargo run -p chimera-cli -- lab mvp-spec-check --json --out docs/mvp_spec_check_latest.json
cargo run -p chimera-cli -- lab mvp-spec-report --out docs/MVP_SPEC_COVERAGE.md
cargo run -p chimera-cli -- lab m5-artifacts-report --out docs/M5_ARTIFACTS_REPORT.md
cargo run -p chimera-cli -- lab m6-artifacts-report --out docs/M6_ARTIFACTS_REPORT.md
cargo run -p chimera-cli -- lab release-readiness-report --json --out docs/RELEASE_READINESS_REPORT.json
cargo run -p chimera-cli -- lab report-pack --json --out docs/REPORT_PACK.json
cargo run -p chimera-cli -- lab artifact-audit --json --out docs/ARTIFACT_AUDIT.json
cargo run -p chimera-cli -- lab mvp-snapshot --json --out docs/MVP_SNAPSHOT.json
cargo run -p chimera-cli -- lab mvp-verify --refresh --json --out docs/MVP_VERIFY.json
cargo run -p chimera-cli -- lab mvp-check
cargo run -p chimera-cli -- mvp-check
cargo run -p chimera-cli -- mvp-verify --refresh --json --out docs/MVP_VERIFY.json
cargo run -p chimera-cli -- mvp-snapshot --json --out docs/MVP_SNAPSHOT.json
cargo run -p chimera-cli -- mvp-spec-check --json --out docs/mvp_spec_check_latest.json
cargo run -p chimera-cli -- mvp-spec-report --out docs/MVP_SPEC_COVERAGE.md
cargo run -p chimera-cli -- m5-artifacts-report --out docs/M5_ARTIFACTS_REPORT.md
cargo run -p chimera-cli -- m6-artifacts-report --out docs/M6_ARTIFACTS_REPORT.md
cargo run -p chimera-cli -- release-readiness-report --json --out docs/RELEASE_READINESS_REPORT.json
cargo run -p chimera-cli -- artifact-audit --json --out docs/ARTIFACT_AUDIT.json
cargo run -p chimera-cli -- report-pack --json --out docs/REPORT_PACK.json
cargo run -p chimera-cli -- lab-smoke
cargo run -p chimera-cli -- lab-doctor --json --out docs/lab_doctor_latest.json
cargo run -p chimera-cli -- lab-hardening-smoke
cargo run -p chimera-cli -- hardening-smoke
cargo run -p chimera-cli -- benchmark-report --out docs/benchmark_latest.json
cargo run -p chimera-cli -- benchmark-regression-check
cargo run -p chimera-cli -- net-sim
cargo run -p chimera-cli -- perf-smoke
cargo run -p chimera-cli -- fuzz-smoke
cargo run -p chimera-cli -- config-smoke
cargo run -p chimera-cli -- lab config-smoke
cargo run -p chimera-cli -- lab fuzz-smoke
cargo run -p chimera-cli -- lab perf-smoke
cargo run -p chimera-cli -- lab net-sim
cargo run -p chimera-cli -- lab benchmark-report --out docs/benchmark_latest.json
cargo run -p chimera-cli -- lab benchmark-regression-check
cargo run -p chimera-gateway -- run --config configs/gateway.example.conf
cargo run -p chimera-gateway -- health --config configs/gateway.example.conf
cargo run -p chimera-gateway -- doctor --config configs/gateway.example.conf
cargo run -p chimera-gateway -- --lang ru run --config configs/gateway.example.conf
cargo run -p chimera-gateway -- --lang ru health --config configs/gateway.example.conf
cargo run -p chimera-gateway -- --lang ru doctor --config configs/gateway.example.conf --json --out docs/gateway_doctor_latest.json
cargo run -p chimera-lab -- smoke
cargo run -p chimera-lab -- doctor
cargo run -p chimera-lab -- mvp-spec-check --json --out docs/mvp_spec_check_latest.json
cargo run -p chimera-lab -- mvp-spec-report --out docs/MVP_SPEC_COVERAGE.md
cargo run -p chimera-lab -- m5-artifacts-report --out docs/M5_ARTIFACTS_REPORT.md
cargo run -p chimera-lab -- m6-artifacts-report --out docs/M6_ARTIFACTS_REPORT.md
cargo run -p chimera-lab -- release-readiness-report --out docs/RELEASE_READINESS_REPORT.md
cargo run -p chimera-lab -- --lang ru release-readiness-report --out docs/RELEASE_READINESS_REPORT_RU.md
cargo run -p chimera-lab -- release-readiness-report --json --out docs/RELEASE_READINESS_REPORT.json
cargo run -p chimera-lab -- report-pack --out docs/REPORT_PACK.md
cargo run -p chimera-lab -- report-pack --json --out docs/REPORT_PACK.json
cargo run -p chimera-lab -- artifact-audit --json --out docs/ARTIFACT_AUDIT.json
cargo run -p chimera-lab -- artifact-audit --text --out docs/ARTIFACT_AUDIT.txt
cargo run -p chimera-lab -- mvp-snapshot --json --out docs/MVP_SNAPSHOT.json
cargo run -p chimera-lab -- mvp-snapshot --text --out docs/MVP_SNAPSHOT.txt
cargo run -p chimera-lab -- mvp-verify --json --out docs/MVP_VERIFY.json
cargo run -p chimera-lab -- mvp-verify --refresh --json --out docs/MVP_VERIFY.json
cargo run -p chimera-lab -- config-smoke
cargo run -p chimera-lab -- --lang ru hardening-smoke
cargo run -p chimera-lab -- --lang ru doctor --json --out docs/lab_doctor_latest.json
cargo run -p chimera-lab -- fuzz-smoke
cargo check --manifest-path fuzz/Cargo.toml
cargo run -p chimera-lab -- perf-smoke
cargo run -p chimera-lab -- net-sim
cargo run -p chimera-lab -- hardening-smoke
cargo run -p chimera-lab -- benchmark-report --out docs/benchmark_latest.json
cargo run -p chimera-lab -- benchmark-report --baseline docs/benchmark_baseline.json --max-regression-pct 20 --out docs/benchmark_latest.json
```

If `just` is installed, the equivalent shortcuts are:

```bash
just check
just test
just deny
just lab-smoke
just lab-doctor
just mvp-spec-check
just mvp-spec-report
just datapath-report-json
just m5-artifacts-report
just m6-artifacts-report
just release-readiness-report
just release-readiness-report-ru
just release-readiness-report-json
just report-pack
just report-pack-json
just artifact-audit
just mvp-snapshot
just mvp-snapshot-text
just mvp-verify
just mvp-verify-refresh
just rollback-smoke
just rollback-json-smoke
just config-smoke
just fuzz-targets-check
just diag-export
just fuzz-smoke
just perf-smoke
just net-sim
just hardening-smoke
just benchmark-report
just benchmark-regression-selfcheck
just benchmark-regression-check
just baseline-verify
just baseline-freeze
just cleanroom-handoff-check
just json-message-contract-check
just rollback-json-contract-check
just truth-contract-check
just ship-report-contract-check
just ship-readiness-selfcheck
just cleanroom-handoff-selfcheck
just runtime-apply-dns-smoke
just runtime-apply-route-smoke
just runtime-apply-route-smoke-selfcheck
just mvp-check
just handoff-check
just ship-readiness
```

## Desktop Control (Linux User Session)

For app-menu launcher + tray control + user services:

```bash
cd ~/chimera-pq
./scripts/install_desktop_control.sh
```

Installed files:
- `~/.config/systemd/user/chimera-gateway.service`
- `~/.config/systemd/user/chimera-client.service`
- `~/.local/share/applications/chimera-control.desktop`

Control commands:

```bash
./scripts/chimera-control.sh start
./scripts/chimera-control.sh status
./scripts/chimera-control.sh stop
./scripts/chimera-control.sh proxy-status
./scripts/chimera-control.sh route-status
./scripts/chimera-control.sh upstream-probe
./scripts/chimera-control.sh upstream-audit 30
just upstream-resilience-smoke
```

If `yad` is available, launcher `CHIMERA Control` shows tray menu with:
`Start CHIMERA`, `Stop CHIMERA`, `Restart CHIMERA`, `Status`, `Doctor`, `Logs`.

UI mode selection is automatic by default.
Optional manual override:

```bash
./scripts/chimera-control.sh ui-mode show
./scripts/chimera-control.sh ui-mode auto
./scripts/chimera-control.sh ui-mode tray
./scripts/chimera-control.sh ui-mode dialog
./scripts/chimera-control.sh ui-mode cli
```

Emergency/debug overrides:

```bash
CHIMERA_FORCE_CLI=1 ./scripts/chimera-control-launcher.sh
CHIMERA_UI_MODE=dialog ./scripts/chimera-control-launcher.sh
```

`benchmark-report` creates a machine-readable JSON report.  
`benchmark-regression-check` compares current performance and net-sim drop rate
with the saved baseline and fails if regression is greater than 20%.
For noise-resilience on shared/dev machines it automatically retries once
before returning final FAIL.
`benchmark-regression-selfcheck` validates benchmark gate script shape
(`scripts/benchmark_regression_check.sh`) before full pipeline runs.
`fuzz-targets-check` verifies dedicated `cargo-fuzz` targets compile
(`fuzz/fuzz_targets/config_parser.rs`, `frame_decoder.rs`, `handshake_decoder.rs`).

`rollback status` shows whether recovery state exists and reports applied runtime flags in JSON mode.  
`rollback clean` performs rollback from state (DNS/route/TUN best-effort) and then removes saved recovery state.  
`rollback recover` performs the same crash-safe rollback flow after forced stop.

`release-readiness-report --json` writes a strict machine-readable release gate report.  
`report-pack --json` writes one JSON summary for CI/CD checks.
`artifact-audit --json` writes one strict JSON with pass/fail for required artifacts.
`artifact-audit --text` writes the same audit in human-readable text.
`mvp-snapshot --json` writes one combined JSON snapshot (milestones + release gate + artifact audit).
`mvp-snapshot --text` writes the same snapshot in human-readable text.
`mvp-verify --json` runs the end-to-end MVP verification pipeline and writes one combined JSON result.
`mvp-verify --refresh` first refreshes key artifacts and then verifies everything in one pass.
Core JSON outputs from `chimera-cli` include simple bilingual fields `message_en` and `message_ru`
for quick non-expert readability.
Doctor JSON outputs are aligned too:
`docs/doctor_latest.json`, `docs/gateway_doctor_latest.json`, `docs/lab_doctor_latest.json`, `docs/datapath_latest.json`
also include `message_en` and `message_ru`.
`just json-message-contract-check` validates that required JSON artifacts contain
`message_en`/`message_ru` and a valid `network_state` field.
`just rollback-json-contract-check` validates rollback JSON fields
(`network_state`, `tun_applied`, `route_applied`, `dns_applied`) in rollback artifacts.
`just baseline-verify` validates SHA-256 integrity of frozen baseline artifacts
listed in `docs/V1_MVP_BASELINE.sha256`.
`just baseline-freeze` regenerates baseline checksums and baseline manifest from current artifacts.
`just cleanroom-handoff-check` runs full handoff gate inside an isolated clean-room copy and refreshes `docs/SECOND_MACHINE_REPORT.md`.
`just truth-contract-check` validates truth-first consistency signals in key docs/reports
(no stale contradictory claims, plus required clean-room benchmark selfcheck signals).
`just ship-report-contract-check` validates required pass/fail fields in `docs/SHIP_READINESS_REPORT.json`
including:
- `"release_ok_lab_only": true`
- `"truth_boundary":{"lab_scope_only":true,"real_world_datapath_closed":false}`
and baseline benchmark artifact presence:
- `docs/benchmark_baseline.json`
- `docs/benchmark_latest.json`
and explicit runtime smoke artifact fields in:
- `docs/RUNTIME_APPLY_DNS_SMOKE.json`
- `docs/RUNTIME_APPLY_ROUTE_SMOKE.json`
and strict runtime release-gate fields in:
- `docs/RELEASE_READINESS_REPORT.json` (`release_gate.runtime_apply_dns_verified=true`)
- `docs/RELEASE_READINESS_REPORT.json` (`release_gate.runtime_apply_route_verified=true`)
`just handoff-check` runs baseline verification + full MVP gate + release readiness JSON.
`just ship-readiness` runs baseline freeze + clean-room handoff verification + final release readiness JSON and writes `docs/SHIP_READINESS_REPORT.json` and `docs/SHIP_READINESS_REPORT.md`.
All three commands are strict by default: if checks fail, exit code is non-zero.
For non-strict report generation, use `chimera-lab` commands that support `--no-strict`,
for example:
- `cargo run -p chimera-lab -- mvp-verify --json --no-strict --out docs/MVP_VERIFY.json`
- `cargo run -p chimera-lab -- mvp-snapshot --json --no-strict --out docs/MVP_SNAPSHOT.json`
- `cargo run -p chimera-lab -- artifact-audit --json --no-strict --out docs/ARTIFACT_AUDIT.json`

## Handoff Bundle

For MVP handoff and wider lab readiness, use:

- `docs/MVP_HANDOFF_CHECKLIST.md`
- `docs/TROUBLESHOOTING.md`
- `docs/FINAL_M5_M6_REPORT.md`
- `docs/GO_NO_GO.md`
- `docs/CEF_GAP_MAP_2026-05-18.md`
- `docs/CEF_PHASE1_GATES.md`
- `docs/CEF_TRACK_REPORT.json`
- `docs/CEF_TRACK_REPORT.md`
- `docs/V1_MVP_BASELINE_MANIFEST.json`
- `docs/V1_MVP_BASELINE.sha256`
- `docs/benchmark_baseline.json`
- `docs/benchmark_latest.json`
- `docs/RELEASE_READINESS_REPORT_RU.md`
- `docs/SHIP_READINESS_REPORT.json`
- `docs/SHIP_READINESS_REPORT.md`
- `docs/RUNTIME_APPLY_DNS_SMOKE.json`
- `docs/RUNTIME_APPLY_ROUTE_SMOKE.json`

The development machine currently has `just` and `cargo-deny` installed via
`cargo install`.

## Quick Start (Simple, RU/EN)

English:

1. Check build: `cargo check --workspace`
2. Run tests: `cargo test --workspace`
3. Quick local VPN smoke: `cargo run -p chimera-cli -- lab-smoke`
4. Full MVP verification: `cargo run -p chimera-cli -- mvp-check`
5. Full hardening check: `cargo run -p chimera-cli -- hardening-smoke`
6. Russian output example: `cargo run -p chimera-cli -- --lang ru mvp-verify --text --out docs/MVP_VERIFY.txt`

Русский:

1. Проверка сборки: `cargo check --workspace`
2. Запуск тестов: `cargo test --workspace`
3. Быстрая локальная проверка VPN: `cargo run -p chimera-cli -- lab-smoke`
4. Полная проверка MVP: `cargo run -p chimera-cli -- mvp-check`
5. Полная проверка надежности: `cargo run -p chimera-cli -- hardening-smoke`
6. Пример вывода на русском: `cargo run -p chimera-cli -- --lang ru mvp-verify --text --out docs/MVP_VERIFY.txt`
