# CHIMERA Mesh Session Handoff: Failure Hardening Pass

## Saved At

- Timestamp: 2026-07-05

## Active Objective

- Reduce broad runtime breakage classes before the next remote stand proof:
  - preserve valid operator settings;
  - stop stale or unsafe sourced env files from silently entering runtime;
  - make publication/binding truth stricter when transit/publication is required;
  - harden service lifecycle around port conflicts, stale state and boot recovery intent.

## What Changed In This Pass

- `scripts/chimera-control.sh`
  - validates sourced env files before service start;
  - seeds bootstrap defaults only when keys are absent;
  - clears stale discovery and transit-lane runtime artifacts when strict publication/binding cannot be produced;
  - makes bound-transit publication strict on start/watch/doctor paths;
  - validates datapath prestart env;
  - treats a broken `ss` probe as a sensor failure and falls back to `lsof`/`netstat` instead of silently skipping listener self-heal;
  - publishes stricter start status fields for runtime publication outcomes.

- `scripts/install_desktop_control.sh`
  - preserves existing bootstrap defaults instead of overwriting them from templates;
  - preserves existing peer-env listen settings except when they conflict with auto-listen mode and must be reset to free-port mode;
  - preserves disabled boot-recovery intent on reinstall/update instead of always re-enabling it;
  - only reports `boot_recovery_status=armed` when runtime unit enable state is actually confirmed.

- `deploy/systemd-user/*.service`
  - node/datapath/site-watch units now have tighter lifecycle coupling and restart throttling;
  - datapath has prestart validation;
  - node has explicit prestart/poststart hooks.

- contract tests
  - start/installer guards now cover strict publication failure, stale discovery cleanup, auto-listen legacy migration and preserved disabled boot recovery.

## Local Evidence

- Syntax:
  - `bash -n scripts/chimera_start_contract_smoke.sh`
  - `bash -n scripts/chimera_installer_gate.sh`
  - `bash -n scripts/install_desktop_control.sh`

- Proof bundle:
  - `bash scripts/chimera_start_contract_smoke.sh`
  - `bash scripts/chimera_stop_contract_smoke.sh`
  - `bash scripts/chimera_update_contract_smoke.sh`
  - `bash scripts/chimera_installer_gate.sh`

- Process / guard bundle:
  - `just session-process-guard`

## Truth Boundary

- The local contract bundle for start/stop/update/install is green in this session.
- This pass does **not** prove real reboot persistence on the remote stand yet.
- This pass does **not** close multi-node live self-recovery after real network partition yet.

## Remaining High-Value Gaps

- Runtime/publication truth can still look healthier than it is in some background-reconcile paths; this needs remote proof and likely another tightening pass.
- Freshness is still weaker than it should be for some persisted selection/publication artifacts; stale-but-present files are not yet universally treated as degraded.
- Real reboot-cycle proof is still missing on laptop/VPS after these hardening changes.

## Next Step

1. Publish these hardening changes through the normal release path.
2. Re-run proof on remote stand:
   - install from release;
   - start/stop/restart;
   - reboot;
   - port-conflict recovery;
   - stale publication recovery;
   - preserved disabled-autostart intent on update.
3. After remote proof, decide whether start should continue to allow `auto_reconcile=armed` success semantics or switch that path to a stricter degraded/non-zero contract.
