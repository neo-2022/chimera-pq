# CHIMERA Remote Cycle Smoke Report

## Scope
- Scenario: remote install/start/status/stop/uninstall loop
- Objective: verify repeated clean install/uninstall stability and service lifecycle reliability
- Bootstrap source: `neo-2022/chimera` main branch
- Runtime package generation mode: static `musl` binaries (`x86_64-unknown-linux-musl`)

## Environment
- Side B: `<stand-user>@<stand-host-a>`
- SIDE_A: `<stand-admin>@<stand-host-b>`
- Date: 2026-05-22 (Europe/Moscow)

## Commands Executed
- Side B:
  - `./scripts/chimera_remote_cycle_smoke.sh --host <stand-host-a-ip> --user art --pass '***' --cycles 5`
- SIDE_A:
  - `./scripts/chimera_remote_cycle_smoke.sh --host <stand-host-b-ip> --user root --pass '***' --cycles 5`

## Results
- Side B:
  - `smoke_result=pass cycles=5`
  - Per-cycle outcome: `rc_start=0 rc_status=0 rc_stop=0 rc_uninstall=0` for all 5 cycles
- SIDE_A:
  - `smoke_result=pass cycles=5`
  - Per-cycle outcome: `rc_start=0 rc_status=0 rc_stop=0 rc_uninstall=0` for all 5 cycles

## Status
- Verdict: PASS
- Regression observed: none in this run

## Notes
- Earlier `glibc` compatibility failures were eliminated by shipping static `musl` runtime binaries in bootstrap `0.1.18+`.
- Current bootstrap line includes archive checksum validation and self-refresh logic.
