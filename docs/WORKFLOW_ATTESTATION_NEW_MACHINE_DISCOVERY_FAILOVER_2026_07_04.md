# Workflow Attestation: New Machine Discovery Failover 2026-07-04

Status: partial

## Objective

Keep the July 4 `new machine -> public install -> plain start -> bind -> full work`
proof truthful while repairing the stale current-workline artifact and adding the
smallest MVP-safe discovery source failover needed for live stand partitions.

## Why This Workline Exists

- The latest handoff moved to the July 4 new-machine proof objective.
- `docs/CURRENT_WORKLINE_ATTESTATION.json` still pointed at the July 1
  AI-architect guard-hardening workline.
- `just session-process-guard` therefore failed by design on
  `latest_handoff is not the newest handoff`.
- Product-side discovery bootstrap was still single-source, which blocks real
  stand recovery when the primary discovery source is unreachable.

## Architectural Decision

- Preserve the July 1 guard-hardening artifact as historical evidence.
- Create a new July 4 current-workline bundle instead of repointing the old
  workline.
- Treat discovery failover as `ordered mirrors of one authority`, not as
  multiple authorities and not as a merge/quorum system.
- Fall through only on transport or unreadable-source failure.
- Stop on trust, freshness, signature, or malformed verified-envelope failure.

## Implementation Slice

- Added ordered discovery-source loading in the CLI inventory path.
- Added fallback to the next source on connect/read failure.
- Added stop-on-trust-failure behavior so invalid signed data does not fall
  through to a later source.
- Updated shell bootstrap/control helpers so a discovery source list is treated
  as authoritative input instead of being rejected up front.
- Added contract coverage for source-list parsing, transport failover, trust
  failure stop, bootstrap control flow, and installer persistence.

## Evidence

- `docs/MESH_SESSION_HANDOFF_2026-07-04_NEW_MACHINE_PROOF_V0_1_168.md`
- `crates/chimera-cli/src/mesh_cli/nodes_inventory.rs`
- `crates/chimera-cli/src/mesh_cli/nodes_inventory/discovery.rs`
- `scripts/chimera-control.sh`
- `scripts/install_desktop_control.sh`
- `scripts/chimera_start_contract_smoke.sh`
- `scripts/chimera_installer_gate.sh`
- `cargo test -q -p chimera-cli discovery_source_list_contract_values`
- `cargo test -q -p chimera-cli falls_through_to_next_source_on_connect_failure`
- `cargo test -q -p chimera-cli does_not_fall_through_after_trust_failure`
- `bash scripts/chimera_start_contract_smoke.sh`
- `bash scripts/chimera_installer_gate.sh`

## Truth Boundary

- The new current-workline bundle is local process evidence for the July 4
  objective.
- The discovery failover patch is locally validated by CLI and shell contract
  tests.
- The final live `v0.1.168` clean-room proof on the stand remains partial until
  the external reachability blocker between discovery sources is reproved in the
  real SSH stand.

## Open Blockers

- The July 4 live stand proof is still partial because the new VPS cannot yet
  reach the old discovery source directly in the failing contour.
- Auto-bind still treats cached signed-looking snapshot structure as higher
  priority than downstream runtime state; deeper signature/freshness enforcement
  for that cached-file path remains follow-up work.
