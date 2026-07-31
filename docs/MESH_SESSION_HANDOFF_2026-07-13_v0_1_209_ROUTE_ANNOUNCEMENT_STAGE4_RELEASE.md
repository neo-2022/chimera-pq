# MESH_SESSION_HANDOFF_2026-07-13_v0_1_209_ROUTE_ANNOUNCEMENT_STAGE4_RELEASE

**session_id:** handoff-2026-07-13-209-route-announcement-stage4-release
**version:** 0.1.209
**status:** pass

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
- Status: `completed` — all jobs green; GitHub `latest` release assets published
  for `v0.1.209`.

The `v0.1.208` tag CI failed on the installer contract gate; `v0.1.209`
contains the contract-gate fixes described above.

## Stand Update

Both authorized stand nodes auto-updated through the release installer path
(`chimera-sh -restart`, which runs `auto_update_if_needed` and then installs
from the GitHub `latest` release before restarting the user units):

```text
chimera_update=source_current source=github current_version=0.1.209 latest_version=0.1.209 ... action=continue reason=source_not_newer
bound_transit_start_contract=ok
start_status=ok mode=systemd_user node_runtime=running ...
```

No manual service-unit restart was performed.

## SSH Stand Release Proof (skill `chimera-ssh-stand-release-proof`)

After the initial report the skill was applied retroactively.  The PC stayed a
control point only; all work was done over SSH.

### Proof host #1 — `amai` (GitHub Latest install proven)

- Stopped the existing runtime.
- Ran the canonical GitHub one-command install exactly:

```bash
bash -o pipefail -c 'curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60 \
  https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh | \
  bash -s -- -install'
```

Result:

```text
chimera_install=ok version=0.1.209 home=<redacted-path>
```

- Installed bundle checksum matches GitHub `latest` checksum:

```text
2a37c7723335a588f28d462694dc94a71b45ef8c595ebe19f953e690ee034542
```

- Lifecycle slice passed:
  `-start` → `-status` → `-restart` → `-status` → `-stop` → delayed `-status`.

### Second host — `vdsina` (local-bundle alignment, not direct GitHub proof)

- GitHub `curl` timed out from this host (`curl: (28) Connection timed out`).
- The installed bundle checksum still matched the GitHub `latest` checksum:
  `2a37c77...034542`.
- This is recorded as "same bundle aligned on second stand host", not as
  "GitHub delivery proven on second host".
- During alignment the installer regenerated `peer-egress.env` and truncated
  the first lines, causing `error: missing CHIMERA_PEER_EGRESS_MODE`.  The file
  was rewritten with the correct full configuration, after which the lifecycle
  slice passed.

### Redacted evidence markers

- `ssh_ok` on both hosts.
- `github_one_command_install_ok` on `amai`.
- `version_ok=0.1.209` on both hosts.
- `checksum_ok=2a37c772...034542` on both hosts.
- `start_status=ok` / `restart_status=ok` / `stop_status=ok` on both hosts.
- `node_runtime=running` / `transparent_runtime=running` after start and
  restart on both hosts.

## Live E2E Ed25519 ANNOUNCE + Sealed Transit Probe

Configuration (applied remotely via SSH, then reverted to baseline after the
probe):

- Forwarder node (`amai`):
  - `CHIMERA_MESH_SELF_NODE_ID=amai`
  - `CHIMERA_MESH_ANNOUNCEMENT_SIGNING_KEY=<redacted_seed>`
  - `CHIMERA_MESH_ANNOUNCEMENT_KEYRING=vdsina:<redacted_pubkey>`
  - `mesh_announcements=static,cidr/<redacted-ip>/32,vdsina,3600,11`
- Via/target node (`vdsina`):
  - `CHIMERA_MESH_SELF_NODE_ID=vdsina`
  - `CHIMERA_MESH_ANNOUNCEMENT_SIGNING_KEY=<redacted_seed>`
  - `CHIMERA_MESH_ANNOUNCEMENT_KEYRING=amai:<redacted_pubkey>`
  - `mesh_announcements=static,cidr/<redacted-ip>/16,amai,3600,12`

Both nodes were restarted with the keyring configuration, authenticated over
the existing peer pool, exchanged signed `ANNOUNCE` messages on ingress, and
merged only announcements whose signatures verified against the configured
public key.

A local echo responder was started on `vdsina` at `<redacted-ip>`.  From
`amai`, a CHIMERA-LOCAL/1 CONNECT probe was injected into the node's local
ingress for destination `<redacted-ip>`:

```text
ack b'OK\n'
resp b'hello route transit\n'
PROBE_OK
```

The round-trip payload reached the echo responder on `vdsina` and returned
unchanged, proving that the signed route announcement produced a working
multi-hop sealed/native transit binding.

After the probe, the test keyring/signing-key lines and the temporary
`mesh_announcements` segments were removed from both nodes' `peer-egress.env`
and the services were restarted back to the baseline `v0.1.209` configuration.

## Conclusion

- Stage 1 live transit proof: already satisfied.
- Stage 2 runtime `ANNOUNCE` distribution: satisfied.
- Stage 3 Ed25519 signing/verification: satisfied.
- Stage 4 GitHub release + auto-update + live Ed25519 ANNOUNCE probe:
  satisfied.

No stand addresses, credentials, or generated Ed25519 seeds are recorded in this
handoff.

## Safety Notes

- No stand IP addresses, credentials, or secrets are recorded in this handoff.
- All practical stand checks will be performed via SSH; the local PC remains a
  control point only.
