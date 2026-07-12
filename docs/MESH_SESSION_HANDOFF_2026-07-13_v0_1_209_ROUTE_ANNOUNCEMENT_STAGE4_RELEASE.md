# MESH_SESSION_HANDOFF_2026-07-13_v0_1_209_ROUTE_ANNOUNCEMENT_STAGE4_RELEASE

**session_id:** handoff-2026-07-13-209-route-announcement-stage4-release
**version:** 0.1.209
**status:** partial

## Objective

Deliver the Stage 3 Ed25519 route-announcement PKI changes through the official
GitHub release pipeline and run a live multi-hop sealed transit probe on the
authorized SSH-only stand with Ed25519-verified `ANNOUNCE` exchange.

Stage 4 acceptance criteria:

- Code pushed, GPG-signed annotated tag created, GitHub release workflow green.
- Release asset published so nodes can auto-update via `chimera-update`.
- No manual SSH service restarts; update propagates through the release
  installer path.
- Live two-/three-node `ANNOUNCE` exchange proves Ed25519 signature
  verification on both ingress directions.
- Multi-hop sealed transit probe payload round-trips successfully.
- Lifecycle handoff/attestation artifacts updated.

## Pipeline Fixes

Commit `bc87ed4` on `main`:

- `deploy/systemd-user/chimera-site-watch.service`:
  - Added `BindsTo=chimera-node.service` required by the installer contract gate.
- `scripts/chimera_start_contract_smoke.sh`,
  `scripts/chimera_stop_contract_smoke.sh`,
  `scripts/chimera_doctor_contract_smoke.sh`:
  - Copy `chimera-control-cleanup.inc` into every temporary install root that
    stages `chimera-control.sh`; the control script sources the include
    unconditionally.
- `scripts/chimera-control.sh`:
  - Reordered `stop_runtime` so `chimera_rollback_cleanup_core` runs only after
    a successful controlled `down`.  Previously it ran before the down/cleanup
    failure checks and unconditionally cleared runtime generated state, causing
    the `datapath_down_failure_fails_closed` contract case to lose its state
    file.

## Local Contract Gate Verification

All release contract gates pass locally before CI:

```text
product language guard: PASS
installer_gate=pass
chimera_update_contract_smoke=pass
chimera_start_contract_smoke=pass
chimera_stop_contract_smoke=pass
chimera_doctor_contract_smoke=pass
chimera_runner_sudo_contract_smoke=pass
```

Release bundle build and install contract smoke also pass:

```text
release_bundle_install_contract_smoke=pass version=0.1.209
```

## Git Tag / Release

Created GPG-signed annotated tag:

```text
tag: v0.1.209
signer: E24FF4804241C3267A6A287AB6A1A3303D939125
commit: bc87ed467a0fb4160e528a85c659f80148bafcd8
```

Pushed to `git@github.com:neo-2022/chimera-pq.git`.

GitHub Actions release workflow:

- Run ID: `29212092208`
- Workflow: `.github/workflows/release.yml`
- Status: in progress while this handoff is written.

The `v0.1.208` tag CI failed on the installer contract gate; `v0.1.209`
contains the contract-gate fixes described above.

## Remaining Work

- Wait for GitHub Actions release workflow `29212092208` to complete and
  publish `chimera-pq-release.tar.gz`.
- Trigger `chimera-update` on the authorized stand nodes to install `v0.1.209`.
- Run the live multi-hop sealed transit probe with `--mesh-announcement-keyring`
  and `--mesh-announcement-signing-key` configured on both peers.
- Collect redacted logs as evidence and update this handoff / attestation.

## Safety Notes

- No stand IP addresses, credentials, or secrets are recorded in this handoff.
- All practical stand checks will be performed via SSH; the local PC remains a
  control point only.
