# CHIMERA Mesh Session Handoff: Failure Hardening Pass

## Saved At

- Timestamp: 2026-07-06

## Active Objective

- Reduce broad runtime breakage classes before the next remote stand proof:
  - preserve valid operator settings;
  - stop stale or unsafe sourced env files from silently entering runtime;
  - make publication/binding truth stricter when transit/publication is required;
  - harden service lifecycle around port conflicts, stale state and boot
    recovery intent;
  - prove the packaged release on the authorized SSH-only remote stand.

## What Changed In This Pass

- `scripts/chimera-control.sh`
  - validates sourced env files before service start;
  - seeds bootstrap defaults only when keys are absent;
  - clears stale discovery and transit-lane runtime artifacts when strict
    publication/binding cannot be produced;
  - makes bound-transit publication strict on start/watch/doctor paths;
  - validates datapath prestart env;
  - treats a broken `ss` probe as a sensor failure and falls back to
    `lsof`/`netstat` instead of silently skipping listener self-heal;
  - publishes stricter start status fields for runtime publication outcomes.

- `scripts/install_desktop_control.sh`
  - preserves existing bootstrap defaults instead of overwriting them from
    templates;
  - preserves existing peer-env listen settings except when they conflict with
    auto-listen mode and must be reset to free-port mode;
  - preserves disabled boot-recovery intent on reinstall/update instead of always
    re-enabling it;
  - only reports `boot_recovery_status=armed` when runtime unit enable state is
    actually confirmed.

- `scripts/install_release.sh`
  - snapshots external operator state before installer mutation;
  - restores external state on installer failure or launcher-link failure;
  - reuses the recorded local-bin target when the operator did not override it.

- `deploy/systemd-user/chimera-runtime.service`
  - now sets `Environment=CHIMERA_FAIL_CLOSED_ON_PARTIAL_START=0` so a
    listener-only partial start during boot recovery does not enter a restart
    loop.

- contract tests
  - start/installer guards now cover strict publication failure, stale
    discovery cleanup, auto-listen legacy migration and preserved disabled
    boot recovery;
  - port-conflict recovery smoke now deterministically proves fixed-port
    listener override and acceptable listener-only partial start.

## Local Evidence

- Syntax:
  - `bash -n scripts/chimera_start_contract_smoke.sh`
  - `bash -n scripts/chimera_stop_contract_smoke.sh`
  - `bash -n scripts/chimera_installer_gate.sh`
  - `bash -n scripts/chimera_update_contract_smoke.sh`
  - `bash -n scripts/chimera_port_conflict_recovery_smoke.sh`
  - `bash -n scripts/install_desktop_control.sh`
  - `bash -n scripts/install_release.sh`

- Proof bundle:
  - `bash scripts/chimera_start_contract_smoke.sh`
  - `bash scripts/chimera_stop_contract_smoke.sh`
  - `bash scripts/chimera_update_contract_smoke.sh`
  - `bash scripts/chimera_installer_gate.sh`
  - `bash scripts/chimera_port_conflict_recovery_smoke.sh`
  - `bash scripts/chimera_reboot_persistence_smoke.sh`
  - `bash scripts/chimera_peer_endpoint_config_smoke.sh`

- Process / guard bundle:
  - `just session-process-guard`

## Remote Stand Evidence (v0.1.170)

- GitHub release `v0.1.170` published and marked Latest.
- Remote stand hosts used: laptop + secondary VPS + primary VPS (PC used as an
  SSH control host; no local CHIMERA runtime or network change on the PC).
- GitHub one-command install succeeded on all three stand hosts.
- Installed version and checksum match `v0.1.170` release assets:
  `b35795d0b0852c61204488f297953dfcdc816172a551facaa658fea22f9d2426`.
- Lifecycle proof on configured node (laptop): start → status → restart →
  status → stop → status all green.
- Reboot recovery proof on fresh/unconfigured nodes (both VPS hosts): after
  reboot `runtime_boot_service_state=active`, `node_service_state=active`,
  `node_runtime=running`, `runtime_state_status=up`.
- Preserved disabled boot recovery: after `systemctl --user disable
  chimera-runtime.service`, reinstall left unit disabled on both VPS hosts.
- Stale publication recovery: fake stale `peer-egress.state`,
  `peer-update.state.json`, `mesh_nodes.discovery.json` removed after service
  start on both VPS hosts.
- Port-conflict recovery: observed auto-listen fallback when configured fixed
  peer listen port was occupied; deterministic local contract smoke now proves
  the same override path end-to-end.

## Truth Boundary

- The local contract bundle for start/stop/update/install, broken-sensor
  listener self-heal, deterministic port-conflict recovery, local
  reboot-persistence recovery, and two-node peer endpoint configuration is
  green in this session.
- Remote install/checksum/lifecycle, reboot recovery, disabled boot recovery,
  and stale-publication cleanup are proved on the three authorized stand hosts
  for `v0.1.170`.
- This pass proves the installer can propagate a peer endpoint from node A to
  node B in a deterministic local smoke, but does **not** yet prove live
  two-node datapath continuity.
- This pass does **not** close the remaining false-green publication semantics
  around `auto_reconcile=armed`.

## Mesh Two-Node Preflight Proof (Added 2026-07-06)

- Secondary VPS (public listener on port 18142) and laptop both run CHIMERA
  v0.1.170 installed from GitHub Latest.
- `chimera-cli mesh connect-probe` from laptop to secondary VPS returned
  `success=true` (TCP/TLS handshake succeeded).
- `chimera-cli mesh launch-preflight` from laptop to secondary VPS returned
  `status=ready`, `ready_for_real_launch=true`, `connect_probe_success=true`.
- `chimera-cli mesh launch-preflight-verify` with:
  - side-a report: laptop → secondary VPS
  - side-b report: secondary VPS → primary VPS listener
  returned `status=ready`, `all_ready=true`, `blockers=[]`.
- Captured artifacts:
  - `docs/mesh_evidence_2026-07-06/laptop_to_vpsb_preflight.json`
  - `docs/mesh_evidence_2026-07-06/laptop_to_vpsb_connect_probe.json`
  - `docs/mesh_evidence_2026-07-06/vpsb_to_vpsa_preflight.json`
  - `docs/mesh_evidence_2026-07-06/pair_verify.json`
  - `docs/CHIMERA_PATH_PROOF.json`
  - `docs/CHIMERA_CHANNEL_AUDIT.json`
- The PC was used only as an SSH control host; no local CHIMERA runtime or PC
  network state was changed.

## Truth Boundary

- Peer egress from the laptop and peer ingress on the secondary VPS are now
  proven with real cross-Internet handshakes.
- The primary VPS was used only as an additional listener so that the pair
  verify command had a ready side-b report; it is not claimed as the target
  two-node datapath.
- Transparent tunneled IP forwarding, DNS-to-route binding, and sealed transit
  are not yet proven because the full datapath apply fails on the VPS due to
  `/etc/resolv.conf` being a systemd-resolved symlink and because the bound
  transit start contract fails when enabled without a bindings file.
- Runtime/publication truth and stale-artifact precedence remain open.

## Remaining High-Value Gaps

- Close the DNS-apply blocker on systemd-resolved hosts (skip or external DNS
  path) so `chimera-cli up --apply-dns` does not abort datapath apply.
- Close the bound-transit start contract failure (`ALLOW_BOUND_TRANSIT=true`
  without a generated transit-lane-bindings file causes node service failure).
- Prove transparent tunneled IP forwarding and DNS-to-route binding between two
  real stand hosts.
- Runtime/publication truth can still look healthier than it is in some
  background-reconcile paths; this still needs deeper negative-path coverage.
- Consumer-side stale artifact precedence is still weaker than it should be for
  some persisted selection/publication artifacts; stale-but-present files are
  not yet universally treated as degraded.
- Reboot persistence is now covered by a local deterministic contract; the next
  remote stand pass should confirm the same behavior on real hardware/cloud.

## Next Step

1. Fix DNS-apply and bound-transit start contract blockers with minimal
   product changes and targeted tests.
2. Re-run full two-host datapath proof through the transparent WEAVE tunnel.
3. Decide whether start should continue to allow `auto_reconcile=armed` success
   semantics or switch that path to a stricter degraded/non-zero contract.
