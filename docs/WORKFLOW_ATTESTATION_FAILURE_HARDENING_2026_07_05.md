# Workflow Attestation: Failure Hardening 2026-07-05

Status: partial

## Objective

Reduce broad runtime and delivery breakage classes before the next remote
stand proof:

- restore external operator state transactionally on failed install/update;
- preserve valid operator intent instead of silently overwriting it;
- keep port-conflict self-heal working even when the primary port sensor is
  missing or broken;
- keep install/update launch location stable across upgrades;
- add contract evidence for rollback of mixed external state;
- keep the remaining truth gaps explicit instead of overclaiming closure.

## Why This Workline Exists

- The July 5 handoff moved the active objective to failure hardening before the
  next SSH stand proof.
- The live current-workline artifact still pointed at the July 4 discovery
  failover line, so `just session-process-guard` no longer matched the newest
  handoff.
- The user explicitly asked for broad failure analysis with a real council and
  for hardening that prevents CHIMERA from breaking while preserving valid
  user settings.

## Architectural Decision

- Do not do a wide speculative refactor.
- Fix the narrowest shared failure boundaries that create real mixed state:
  `install_release.sh` transactionality, installer setting preservation,
  listener-port sensor fallback, and contract coverage.
- Preserve historical July 4 receipts as historical evidence; create a new
  July 5 workline bundle instead of mutating the old one.
- Keep status truthful: this line is still `partial` because publication truth,
  remote reboot persistence, stale artifact precedence, and live rebind
  continuity are not yet fully proved.

## Implementation Slice

- `scripts/install_release.sh`
  - now snapshots external operator state before installer mutation;
  - restores external state on installer failure or launcher-link failure;
  - reuses the recorded local-bin target when the operator did not override it.
- `scripts/install_desktop_control.sh`
  - now preserves existing transparent runtime UID/GID/exempt UID when valid,
    instead of silently overwriting them with the current shell user.
- `scripts/chimera-control.sh`
  - now treats a broken `ss` listener probe as sensor failure and falls back
    to the next available probe, so fixed-port self-heal still arms runtime
    auto-listen overrides instead of quietly doing nothing.
- contract coverage
  - installer gate now proves preserved transparent UID/GID/exempt UID;
  - start contract smoke now proves blocked fixed-listener recovery still
    happens when `ss` is unavailable but fallback probes are present;
  - update contract smoke now proves external-state rollback on failed install;
  - update contract smoke now proves external-state rollback on failed launcher
    link after installer mutation;
  - update contract smoke now proves recorded local-bin reuse;
  - release bundle install contract still passes on the packaged path.

## Evidence

- `docs/MESH_SESSION_HANDOFF_2026-07-05_FAILURE_HARDENING_PASS.md`
- `scripts/install_release.sh`
- `scripts/install_desktop_control.sh`
- `scripts/chimera-control.sh`
- `scripts/chimera_start_contract_smoke.sh`
- `scripts/chimera_installer_gate.sh`
- `scripts/chimera_update_contract_smoke.sh`
- `scripts/release_bundle_install_contract_smoke.sh`
- `bash scripts/chimera_start_contract_smoke.sh`
- `bash scripts/chimera_stop_contract_smoke.sh`
- `bash scripts/chimera_installer_gate.sh`
- `bash scripts/chimera_update_contract_smoke.sh`
- `bash scripts/release_bundle_install_contract_smoke.sh`
- `just session-process-guard`

## Truth Boundary

- Local contract evidence for start/stop/update/install rollback, operator-
  setting preservation, and broken-sensor listener self-heal is green in this
  session.
- This workline does not prove remote reboot persistence yet.
- This workline does not prove live multi-node rebind continuity yet.
- This workline does not close the remaining false-green publication semantics
  around `auto_reconcile=armed`.

## Open Blockers

- Start/publication truth is still too optimistic in some `auto_reconcile`
  paths and can look healthier than it is.
- Consumer-side stale artifact precedence is still weaker than the verified CLI
  discovery path in `mesh_launch_preflight_auto_bind.sh`.
- Remote packaged proof is still missing for reboot persistence, post-rebind
  peer continuity, and live publication recovery on the SSH stand.
