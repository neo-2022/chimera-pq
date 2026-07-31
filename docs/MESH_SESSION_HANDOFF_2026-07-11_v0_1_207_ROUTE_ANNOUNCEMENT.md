# MESH_SESSION_HANDOFF_2026-07-11_v0_1_207_ROUTE_ANNOUNCEMENT

**session_id:** handoff-2026-07-11-207-route-announcement
**version:** 0.1.207
**status:** partial-to-pass

## Objective

Implement Phase 4 "Route Announcement" (approach A) for the CHIMERA-PQ / WEAVE
mesh node MVP:

- model and parse a minimal static route announcement;
- wire announcements into the multipath schedule planner so that `via` peers
  generate transitive `MeshCarrierLaneBinding`s when a `route_binding_id` is
  configured;
- expose the feature through the new `chimera mesh route-announce` CLI;
- prove the planner output on the authorized three-node stand (laptop, NL, RU).

## Code Changes

Commit `3f1cb62` on `main`:

- `crates/chimera-mesh/src/route_announcement.rs` — new model/parser.
  - `RouteAnnouncement::Static { destination, via, route_binding_id, ttl, auth }`
  - `RouteDestination::{Cidr, Domain}`
  - `parse_route_announcements(raw)` accepts pipe-separated entries:
    `static,<cidr/domain>,<via>,<ttl>,<route_binding_id>[,<base64_sig>]`
- `crates/chimera-mesh/src/dps_payload_snapshot.rs` — recognizes
  `mesh_announcements=...`, stores parsed `Vec<RouteAnnouncement>` in the DPS
  snapshot, and exposes `snapshot.route_announcements()`.
- `crates/chimera-mesh/src/multipath_model.rs` — adds
  `MeshMultipathLaneRole::Transit` and `route_announcements` field to
  `MeshMultipathSchedule`.
- `crates/chimera-mesh/src/runtime/multipath_schedule.rs` —
  `build_multipath_schedule` / `replace_multipath_schedule` now consume
  `&[RouteAnnouncement]`. When a `route_binding_id` is present, every static
  announcement whose `via` peer is in the selected peer set receives a carrier
  binding. If that peer already has a lane binding, the existing binding is
  kept; otherwise a synthetic `Transit` binding is appended.
- `crates/chimera-mesh/src/runtime/{plan_ops_dps_eval,multipath_rebuild_bridge,path_planner,multipath_schedule_tests}.rs` —
  updated call sites and added planner unit tests.
- `crates/chimera-cli/src/mesh_cli/route_announce_cmd.rs` —
  `chimera mesh route-announce` command.
- `crates/chimera-cli/src/main.rs` and `crates/chimera-cli/src/mesh_cli/mod.rs` —
  usage/help strings updated.
- Cargo carriers/lab test fixtures updated for the new schedule field.

Commit `3fa94f6` on `main`:

- `crates/chimera-cli/src/mesh_cli/route_announce_cmd.rs` —
  added optional `--out <file>` flag to `mesh route-announce`; writes the same
  text/JSON report to a file in addition to stdout.
- `crates/chimera-cli/src/main.rs` — updated English and Russian help lines
  to show `[--out <file>] / [--out <файл>]`.

## Stand Proof

All practical checks were performed remotely via SSH, with the local PC acting
only as a control point.

Build used for the stand check:

```text
cargo build --release -p chimera-cli
artifact: target/release/chimera-cli
```

### Laptop: laptop → RU → NL route announcement

Command (node `laptop`, destination NL network via RU peer `vdsina`):

```text
chimera-cli mesh route-announce
  --namespace stand --node laptop
  --destination cidr/<redacted-ip>/24
  --via vdsina --route-binding-id 11
  --peer <redacted-login>:443@nl@10@95
  --peer <redacted-login>:443@ru@12@93
  --json
```

Result:

```json
{
  "carrier_binding_count": 2,
  "execution_status": "carrier_lane_binding_contract_ready",
  "multipath_mode": "off",
  "policy_payload": "allow=mesh;mesh_multipath_mode=off;mesh_route_binding_id=11;mesh_max_peers=2;mesh_max_selected_per_region=2;mesh_announcements=static,cidr/<redacted-ip>/24,vdsina,3600,11",
  "route_announcement_count": 1,
  "status": "ok",
  "transit_binding_count": 1
}
```

The planner selected both peers, kept the active lane binding for `amai`, and
added a transitive `Transit` carrier binding for `vdsina` because the
announcement designated `vdsina` as the `via` peer for the NL destination.

### NL node: destination laptop network via RU

```text
chimera-cli mesh route-announce
  --namespace stand --node amai
  --destination cidr/<redacted-ip>/24
  --via vdsina --route-binding-id 7
  --peer <redacted-login>:443@nl@10@95
  --peer <redacted-login>:443@ru@12@93
```

Result: `carrier_binding_count=2`, `transit_binding_count=1`,
`execution_status=carrier_lane_binding_contract_ready`.

### RU node: destination dummy target via NL

```text
chimera-cli mesh route-announce
  --namespace stand --node vdsina
  --destination cidr/<redacted-ip>/16
  --via amai --route-binding-id 9
  --peer <redacted-login>:443@nl@10@95
  --peer <redacted-login>:443@ru@12@93
  --json
```

Result: `carrier_binding_count=1`, `transit_binding_count=0`,
`execution_status=carrier_lane_binding_contract_ready`. Only one binding
appears because the planner excludes the local node `vdsina` from the selected
peer set, so `amai` becomes the single selected peer and receives the normal
active binding. The announcement is still parsed and stored.

## Test Status

```text
 cargo test -p chimera-mesh --lib          336 passed
 cargo test -p chimera-carrier --lib       (passed with new schedule field)
 cargo test -p chimera-cli                 437 passed
 cargo test --workspace --lib              passed
 cargo test -p chimera-carrier --test multi_hop_sealed_transit  2 passed
 cargo build --release -p chimera-cli      succeeded
```

## Remaining / Next

- Seal the announcement auth blob with a real PKI check (currently parsed but
  only used as a warn/minimal placeholder).
- Distribute announcements through the runtime instead of only via DPS payload:
  add a discovery/advertisement path so peers can exchange capability tokens.
- Run an actual end-to-end data-plane transit using the new route-announcement
  bindings (beyond the planner proof and the existing in-process multi-hop
  sealed-transit test).
- Decide whether to evolve from approach A to approach B (full distributed
  capability exchange) once the MVP mesh node is stable.
