# MESH_SESSION_HANDOFF_2026-07-06_v0_1_208_ROUTE_ANNOUNCEMENT_TRANSIT_PROOF

**session_id:** handoff-2026-07-06-208-route-announcement-transit-proof
**version:** 0.1.208
**status:** stage_1_pass / stage_2_code_complete

## Objective

Execute Stage 1 of the approved Phase 4 Route Announcement hardening plan
and Stage 2 runtime distribution of route announcements:
- Stage 1: prove that a route announcement creates a real multi-hop sealed transit data
  path on the authorized SSH-only stand.
- Stage 2: add a secure peer `ANNOUNCE` wire message, registry, planner merge,
  and unit/integration tests.

## Approach

- Use `chimera-cli mesh route-explain` with a route-announcement DPS payload
to produce a planner lane document.
- Start `chimera-peer-egress --mode node` on a forwarding node (`amai`) with the
lane document loaded as `transit-lane-bindings-file`.
- Run a local echo responder behind the transit target node (`vdsina`) on its
loopback.
- Send a `CHIMERA-LOCAL/1` CONNECT request into the forwarding node's local
ingress for the announced destination.
- Verify the round-trip payload reaches the echo responder and returns.

All practical commands were executed remotely; the local PC was used only as
an SSH control point. Stand addresses and credentials are redacted.

## Planner Input

```text
chimera-cli mesh route-explain
  --namespace stand --node <forwarder_node>
  --policy-payload "allow=mesh;mesh_multipath_mode=off;mesh_route_binding_id=11;
                    mesh_max_peers=1;mesh_max_selected_per_region=1;
                    mesh_announcements=static,cidr/127.0.0.1/32,<via_node>,3600,11"
  --peer <via_node>@<RU_PEER_LISTEN>@ru@0@100
  --transit-lane-bindings-out /tmp/chimera-lanes-stand.csv
```

Planner lane document excerpt:

```text
# chimera_transit_lane_document=v1
# chimera_plan_namespace=stand
# chimera_plan_route_binding_id=11
# chimera_plan_selected_peer	0	<via_node>	<RU_ENDPOINT>	ru	0	100	100
# chimera_plan_mode=off
# chimera_plan_carrier_binding	11	0	<via_node>	<RU_ENDPOINT>	active	100	90
# chimera_plan_execution_status=carrier_lane_binding_contract_ready
# chimera_plan_active_lane_count=1
```

## Runtime Configuration

Forwarding node (`amai`):

```text
chimera-peer-egress --mode node
  --local-listen 127.0.0.1:18190
  --peer-listen 0.0.0.0:0
  --token mesh-shared-token
  --allow-bound-transit true
  --transit-lane-bindings-file /tmp/chimera-lanes-stand.csv
  --aead chacha20poly1305
```

Log evidence (redacted):

```text
event=outbound_transit_lane_connected endpoint=<redacted>
event=outbound_transit_lane_registered binding=<opaque>
event=weave_local_ingress_accepted
event=local_ingress_destination host=<redacted> port=<redacted> destination_id=MNFKLQRGPTKPMTFQ
event=local_ingress_paired_with_peer attempt=1 destination_id=MNFKLQRGPTKPMTFQ
```

Target node (`vdsina`):

- Existing `chimera-peer-egress --mode node` peer listener on `<RU_PEER_LISTEN>`
  authenticated with `mesh-shared-token`.
- Local echo responder bound to `127.0.0.1:7777`.

## Probe and Result

Local probe via bash/python on the forwarding node:

```python
s.connect(("127.0.0.1", 18190))
s.sendall(b"CHIMERA-LOCAL/1\nCONNECT 127.0.0.1 7777\n")
ack = s.recv(16)      # b'OK\n'
s.sendall(b"hello route transit\n")
resp = s.recv(1024)   # b'hello route transit\n'
```

Result: `ack` received, payload echoed unchanged.

## Conclusion

The route-announcement policy produced a planner carrier binding; the binding
was consumed by the runtime lane registry; the local ingress selected the
outbound transit lane to the via peer; the via peer resolved the native CONNECT
request to its local echo responder; the round-trip payload returned.

Stage 1 acceptance criteria satisfied:

- Multi-hop sealed/native transit flow succeeds with route-announcement bindings.
- Evidence is recorded redacted (no raw payloads or secrets).
- No local PC network state changed.

## Stage 2 — Runtime Distribution (implementation complete)

Changes committed on `main`:

- `crates/chimera-carrier/src/peer_egress/wire.rs`:
  - Added `PeerMessage::Announce(Vec<RouteAnnouncement>)`.
  - Text-line wire form: `ANNOUNCE static,<dest>,<via>,<ttl>,<id>[,<base64_sig>]|...`.
  - Added `write_announce_message` and parser path.
- `crates/chimera-carrier/src/peer_egress/route_announcement_registry.rs`:
  - `SharedRouteAnnouncementRegistry` with deduplication by
    `(destination, via, route_binding_id)` and TTL expiry filtering.
- `crates/chimera-mesh/src/runtime.rs` + rebuild trigger/model:
  - Added `runtime_announcements` registry to `MeshRuntime` with
    `merge_runtime_announcements`, deduplication, and a new
    `RuntimeAnnouncementsChanged` rebuild cause.
- `crates/chimera-mesh/src/runtime/plan_ops_dps_eval.rs`:
  - Added `plan_path_from_dps_payload_with_announcements` so the carrier lane
    driver can merge received announcements into the DPS snapshot before
    planning.
- `crates/chimera-carrier/src/peer_egress/mesh_lane_driver.rs`:
  - Driver now passes the runtime registry into planning.
- `crates/chimera-carrier/src/peer_egress/node.rs` and `live_bindings.rs`:
  - Node sends its local announcement set after secure peer handshake on both
    inbound and outbound lane workers.
  - Pool workers consume `Announce` messages, merge them, then continue to
    serve data traffic on the same peer connection.

### Test results

```text
cargo test -p chimera-mesh --lib          336 passed
cargo test -p chimera-carrier --lib       246 passed
cargo test -p chimera-cli                 437 passed
cargo test -p chimera-carrier --test multi_hop_sealed_transit  2 passed
cargo build --release -p chimera-cli      succeeded
```

Added tests:

- `peer_egress::wire::tests::peer_wire_messages_round_trip_announce`
- `peer_egress::route_announcement_registry::tests::registry_deduplicates_by_destination_via_binding`
- `peer_egress::route_announcement_registry::tests::registry_drops_expired_announcements`
- `route_announcement::tests::format_and_parse_round_trip_preserves_announcement`
- `route_announcement::tests::format_includes_signature_when_present`
- `runtime::tests::runtime_announcements_create_transit_carrier_binding_in_plan`

### Stage 2 live verification

Not yet run on the authorized SSH stand. The code is covered by unit/integration
artifacts above; a live two-node exchange will be performed during Stage 4
integration.

## Remaining Work

- Stage 3: Ed25519 signing/verification of route announcements.
- Stage 4: combined integration, full test matrix, and lifecycle attestation.
