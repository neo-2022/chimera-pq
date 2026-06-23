# CHIMERA Mesh Session Handoff

## Saved At

- Timestamp: 2026-06-15 23:40 local session

## Active Objective

- Continue WEAVE symmetric mesh-node MVP.
- Current slice: source-level mesh/carrier bound transit wiring without local PC
  runtime or network mutation.

## What Was Done

- Added explicit mesh route binding model in `crates/chimera-mesh/src/multipath_model.rs`.
- Moved multipath model types out of `model.rs` to satisfy anti-monolith guard.
- Changed multipath schedule behavior:
  - lanes are built for planning;
  - live carrier lane bindings are emitted only when DPS/control-plane supplies
    explicit nonzero `mesh_route_binding_id`;
  - planner no longer derives route ids from peer id, endpoint, weights or
    topology.
  - schedule status is truthful: plans without route binding stay
    `planner_only_not_carrier_bound`; only emitted carrier bindings report
    `carrier_lane_binding_contract_ready`;
  - carrier lane binding construction is fail-closed if an internal lane cannot
    be matched to a selected peer;
  - `transit_capacity_budget_pct` records transit budget separately from local
    traffic reserve.
- Added carrier lane binding adapter and config parser:
  - `crates/chimera-carrier/src/peer_egress/lane_binding.rs`;
  - supports `CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE` and
    `--transit-lane-bindings-file`;
  - converts zero-based mesh lane id to nonzero carrier lane id.
- Added separate bound transit policy:
  - `allow_bound_transit` / `CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT`;
  - unbound pool transit remains controlled only by `allow_pool_transit`.
- Added install/startup safety:
  - local bound sealed transit requires explicit bound-transit policy;
  - node startup rejects a transit lane bindings file unless
    `allow_bound_transit=true`;
  - one-command install preserves
    `CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE` when provided;
  - installer guard checks the lane-bindings env wiring.
- Preserved fail-closed behavior:
  - bound transit requires dispatcher and exact binding;
  - no fallback to `PeerPool` for bound transit.
- Cleaned local git `origin` URL so diagnostics no longer print a GitHub token.
- Updated attestations:
  - `docs/WORKFLOW_ATTESTATION_MULTIPATH_SCHEDULE_2026-06-15.md`;
  - `docs/WORKFLOW_ATTESTATION_SEALED_TRANSIT_BINDING_2026-06-15.md`.

## Validation

- PASS: `cargo fmt --all -- --check`
- PASS: `cargo check -q --workspace`
- PASS: `cargo test -q -p chimera-mesh` (163 tests)
- PASS: `cargo test -q -p chimera-carrier` (84 tests)
- PASS: `cargo test -q -p chimera-session sealed_transit` (3 tests)
- PASS: `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- PASS: `cargo clippy -q -p chimera-carrier --all-targets -- -D warnings`
- PASS: `bash scripts/anti_monolith_guard.sh`
- PASS: `bash scripts/chimera_installer_gate.sh`
- PASS: `just rust-no-hardcode-guard`

## Remote Stand Status

- Side B `<stand-user>@<stand-host-a>`: SSH reachable; has `curl`; no `cargo`, `rustc`,
  or `git` found; `~/.local/share/chimera-pq` contains `runtime` only in the
  checked path.
- SIDE_A `<stand-admin>@<stand-host-b>`: SSH reachable; has `cargo`, `rustc`, `git`, `curl`;
  installed release marker shows `0.1.69`.
- Current source was not installed on the stand because no new GitHub release
  containing this patch has been published in this session.

## Not Closed

- No Real-World PASS for this source change yet.
- Need GitHub release/update path first, then one-command install/update on SIDE_A
  and side_b over SSH.
- Need real carrier-bound transit proof after stand update.

## Safety

- No CHIMERA runtime was started, stopped or restarted on the current PC.
- No local DNS, routes, firewall, proxy, VPN, Happ, MYVPN, router, side_b or SIDE_A
  network settings were changed by this source-level work.

## Next Step

- Publish/build a GitHub release from current source, update SIDE_A and side_b only
  through the documented one-command GitHub path, then run node-to-node
  carrier-bound sealed transit proof on external remote proof nodes.
