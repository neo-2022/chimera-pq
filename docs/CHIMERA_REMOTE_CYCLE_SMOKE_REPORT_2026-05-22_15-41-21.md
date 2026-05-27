# CHIMERA Remote Cycle Smoke Report

## Scope
- Scenario: remote install/start/status/stop/uninstall loop
- Objective: verify repeated clean install/uninstall stability and service lifecycle reliability
- Bootstrap source: `neo-2022/chimera` main branch
- Runtime package generation mode: static `musl` binaries (`x86_64-unknown-linux-musl`)

## Environment
- Laptop: `art@192.168.31.31`
- VPS: `root@91.124.19.180`
- Date: 2026-05-22 (Europe/Moscow)

## Commands Executed
- Laptop:
  - `./scripts/chimera_remote_cycle_smoke.sh --host 192.168.31.31 --user art --pass '***' --cycles 5`
- VPS:
  - `./scripts/chimera_remote_cycle_smoke.sh --host 91.124.19.180 --user root --pass '***' --cycles 5`

## Results
- Laptop:
  - `smoke_result=pass cycles=5`
  - Per-cycle outcome: `rc_start=0 rc_status=0 rc_stop=0 rc_uninstall=0` for all 5 cycles
- VPS:
  - `smoke_result=pass cycles=5`
  - Per-cycle outcome: `rc_start=0 rc_status=0 rc_stop=0 rc_uninstall=0` for all 5 cycles

## Status
- Verdict: PASS
- Regression observed: none in this run

## Notes
- Earlier `glibc` compatibility failures were eliminated by shipping static `musl` runtime binaries in bootstrap `0.1.18+`.
- Current bootstrap line includes archive checksum validation and self-refresh logic.
