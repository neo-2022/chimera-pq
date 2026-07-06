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
  two-node live datapath proof, and consumer-side stale artifact precedence are
  not yet fully proved.

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
  - clears stale discovery and transit-lane runtime artifacts when strict
    publication/binding cannot be produced.
  - makes bound-transit publication strict on start/watch/doctor paths.
- `deploy/systemd-user/chimera-runtime.service`
  - now sets `Environment=CHIMERA_FAIL_CLOSED_ON_PARTIAL_START=0` so a listener-
    only partial start during boot recovery does not enter a restart loop.
- contract coverage
  - installer gate now proves preserved transparent UID/GID/exempt UID;
  - start contract smoke now proves blocked fixed-listener recovery still
    happens when `ss` is unavailable but fallback probes are present;
  - update contract smoke now proves external-state rollback on failed install;
  - update contract smoke now proves external-state rollback on failed launcher
    link after installer mutation;
  - update contract smoke now proves recorded local-bin reuse;
  - port-conflict recovery smoke now proves deterministic listener override
    when a fixed peer-listen port is blocked;
  - release bundle install contract still passes on the packaged path.

## Evidence

- `docs/MESH_SESSION_HANDOFF_2026-07-05_FAILURE_HARDENING_PASS.md`
- `scripts/install_release.sh`
- `scripts/install_desktop_control.sh`
- `scripts/chimera-control.sh`
- `scripts/chimera_start_contract_smoke.sh`
- `scripts/chimera_stop_contract_smoke.sh`
- `scripts/chimera_installer_gate.sh`
- `scripts/chimera_update_contract_smoke.sh`
- `scripts/chimera_port_conflict_recovery_smoke.sh`
- `scripts/release_bundle_install_contract_smoke.sh`
- `deploy/systemd-user/chimera-runtime.service`
- `bash scripts/chimera_start_contract_smoke.sh`
- `bash scripts/chimera_stop_contract_smoke.sh`
- `bash scripts/chimera_installer_gate.sh`
- `bash scripts/chimera_update_contract_smoke.sh`
- `bash scripts/chimera_port_conflict_recovery_smoke.sh`
- `bash scripts/chimera_reboot_persistence_smoke.sh`
- `bash scripts/chimera_peer_endpoint_config_smoke.sh`
- `bash scripts/release_bundle_install_contract_smoke.sh`
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
  peer listen port was occupied; deterministic contract smoke now proves the
  same override path locally.

## Live Mesh Preflight Evidence (2026-07-06)

- `chimera-cli mesh connect-probe` from the laptop to the secondary VPS
  returned `success=true`.
- `chimera-cli mesh launch-preflight` from the laptop to the secondary VPS
  returned `status=ready`, `ready_for_real_launch=true`,
  `connect_probe_success=true`.
- `chimera-cli mesh launch-preflight-verify` across the laptop → secondary VPS
  report and the secondary VPS → primary VPS listener report returned
  `status=ready`, `all_ready=true`, `blockers=[]`.
- Captured artifacts:
  - `docs/CHIMERA_PATH_PROOF.json`
  - `docs/CHIMERA_CHANNEL_AUDIT.json`
  - `docs/mesh_evidence_2026-07-06/`
- These artifacts prove real peer egress from the laptop and peer ingress on the
  secondary VPS. They do **not** yet prove transparent tunneled IP forwarding.

## Live Transparent Datapath Start Evidence (2026-07-06)

- `scripts/chimera-control.sh` patched on the secondary VPS to use
  `CHIMERA_APPLY_DNS` and degrade the bound-transit start contract.
- Secondary VPS start returned `start_status=ok` with
  `transparent_runtime=running` and `datapath_proof=ok`.
- Runtime state captured: `tun_applied=true`, `route_applied=true`,
  `dns_applied=true`, TUN `chimera0`.
- Stop/rollback completed cleanly and restored `/etc/resolv.conf`.
- `chimera-cli state proof --require-flow false` returned `datapath_proof=ok`.
- Flow-proof sidecar is not yet produced by the runtime, so `--require-flow
  true` still reports `missing_flow_proof`.

## Truth Boundary

- Local contract evidence for start/stop/update/install rollback, operator-
  setting preservation, broken-sensor listener self-heal, deterministic
  port-conflict listener override, local reboot-persistence recovery, and
  two-node peer endpoint configuration is green in this session.
- Remote install/checksum/lifecycle, reboot recovery, disabled boot recovery,
  and stale-publication cleanup are proved on all three stand hosts for
  `v0.1.170`.
- This workline proves the installer/orchestrator can configure a peer
  endpoint on node B from a known node A endpoint, but does not yet exercise
  live two-node datapath continuity.
- This workline does not close the remaining false-green publication semantics
  around `auto_reconcile=armed`.

## Open Blockers

- Start/publication truth is still too optimistic in some `auto_reconcile`
  paths and can look healthier than it is.
- Consumer-side stale artifact precedence is still weaker than the verified CLI
  discovery path in `mesh_launch_preflight_auto_bind.sh`.
- Real two-node live datapath proof is partially complete: peer egress from
  the laptop and peer ingress on the secondary VPS are proven; the secondary
  VPS now starts the transparent datapath (TUN/route/DNS applied) and rolls
  back cleanly, but packet forwarding through the peer and external reachability
  are not yet proven.
- The runtime does not yet generate a datapath flow-proof sidecar, so strict
  flow proof cannot pass even though the datapath state is valid.
- `chimera-cli up --apply-dns` requires an operator workaround on
  systemd-resolved hosts; `CHIMERA_APPLY_DNS=false` is an escape hatch, but
  then `dns_applied=false` prevents state proof from passing.
- Enabling `CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true` without a generated
  transit-lane-bindings file is tolerated by the control script but still
  hard-fails inside the Rust runtime.
- Reboot persistence is now covered locally; a future pass should align remote
  stand proof with the same deterministic contract if the implementation
  changes.

## Related Stand-Only Reports

- `docs/WORKFLOW_ATTESTATION_REMOTE_STAND_V0_1_170_2026-07-06.md` (stand-specific
  redacted evidence for v0.1.170)
- `docs/MESH_SESSION_HANDOFF_2026-07-06_REMOTE_STAND_PROOF.md` (stand-specific
  handoff for v0.1.170)
