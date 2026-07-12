# MESH_SESSION_HANDOFF_2026-07-06_v0_1_208_ROUTE_ANNOUNCEMENT_TRANSIT_PROOF

**session_id:** handoff-2026-07-06-208-route-announcement-transit-proof
**version:** 0.1.208
**status:** stage_1_pass

## Objective

Execute Stage 1 of the approved Phase 4 Route Announcement hardening plan:
prove that a route announcement creates a real multi-hop sealed transit data
path on the authorized SSH-only stand.

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

## Remaining Work

- Stage 2: runtime distribution of announcements over the peer wire (`ANNOUNCE`
  message type and `MeshRuntime` registry).
- Stage 3: Ed25519 signing/verification of route announcements.
- Stage 4: combined integration, full test matrix, and lifecycle attestation.
