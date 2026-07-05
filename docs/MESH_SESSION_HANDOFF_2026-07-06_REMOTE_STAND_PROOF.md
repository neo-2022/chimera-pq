# CHIMERA Mesh Session Handoff: Remote Stand Proof v0.1.170

## Saved At

- Timestamp: 2026-07-06

## Active Objective

- Publish the failure-hardening pass through the normal release path and prove
  install/checksum/lifecycle on the authorized SSH-only remote stand.
- Fix the boot-recovery regression discovered during reboot proof
  (listener-only partial start fail-closed in systemd user unit).

## What Changed In This Pass

- Committed the 2026-07-05 failure hardening pass as `deaaa3c`.
- Tagged and published `v0.1.169`.
- Discovered during reboot proof: on a fresh install without a peer endpoint,
  `chimera-runtime.service` entered a restart loop because
  `CHIMERA_FAIL_CLOSED_ON_PARTIAL_START` default caused listener-only partial
  start to exit with status 2.
- Fixed in `deploy/systemd-user/chimera-runtime.service` by adding
  `Environment=CHIMERA_FAIL_CLOSED_ON_PARTIAL_START=0` so boot recovery accepts
  listener-only mode without stopping the node runtime.
- Added installer gate check requiring the env var in the shipped runtime unit.
- Committed fix as `2095d55`, tagged `v0.1.170`, published release assets.

## Local Evidence

- `cargo check` green.
- `bash -n` green for all modified shell scripts.
- `bash scripts/chimera_start_contract_smoke.sh` → pass
- `bash scripts/chimera_stop_contract_smoke.sh` → pass
- `bash scripts/chimera_update_contract_smoke.sh` → pass
- `bash scripts/chimera_installer_gate.sh` → pass
- `just session-process-guard` → pass

## Remote Stand Evidence

- GitHub release `v0.1.170` published and marked Latest.
- Remote stand hosts used: laptop + secondary VPS.
- Primary VPS (91.124.19.180) unreachable by SSH; documented as blocker, no
  evidence claimed for it.
- GitHub one-command install succeeded on reachable stand hosts.
- Installed version and checksum match `v0.1.170` release assets:
  `b35795d0b0852c61204488f297953dfcdc816172a551facaa658fea22f9d2426`.
- Lifecycle proof on configured node (laptop): start → status → restart →
  status → stop → status all green.
- Reboot recovery proof on fresh/unconfigured node (secondary VPS): after
  reboot `runtime_boot_service_state=active`, `node_service_state=active`,
  `node_runtime=running`, `runtime_state_status=up`.
- Preserved disabled boot recovery: after `systemctl --user disable
  chimera-runtime.service`, reinstall left unit disabled.
- Stale publication recovery: fake stale `peer-egress.state`,
  `peer-update.state.json`, `mesh_nodes.discovery.json` removed after service
  start.
- Port-conflict recovery: observed auto-listen fallback when configured fixed
  peer listen port was occupied; peer listen binding reset to `0.0.0.0:0` and
  node service retried. Controlled negative-path logs lost due to SSH session
  disconnect, so evidence is observational.

## Truth Boundary

- Remote proof is green for reachable stand hosts on lifecycle, reboot,
  disabled-boot preservation, and stale-publication cleanup.
- Primary VPS (91.124.19.180) could not be reached; any claim for it is
  explicitly `not checked`.
- Port-conflict recovery is observed but not fully instrumented with a clean
  end-to-end log due to the SSH session reset.
- Real mesh datapath proof between the two stand hosts was not exercised.
- No production GUI/mobile work was started.

## Remaining High-Value Gaps

- Investigate and restore SSH access to primary VPS 91.124.19.180.
- Add a contract smoke that deterministically proves port-conflict recovery
  without requiring a live SSH blocker process.
- Add a contract smoke that proves reboot persistence using a local VM or
  container to avoid depending on physical stand reboots.
- Two-node live datapath / peer discovery proof between laptop and VPS.
- Runtime/publication truth still needs deeper negative-path coverage.

## Next Step

1. Restore SSH access to primary VPS or document it as a stand blocker.
2. Add deterministic port-conflict and reboot contract tests.
3. Re-run full three-host proof once all stand hosts are available.
4. Decide whether to keep `auto_reconcile=armed` success semantics or switch to
  stricter degraded/non-zero contract per the 2026-07-05 handoff.
