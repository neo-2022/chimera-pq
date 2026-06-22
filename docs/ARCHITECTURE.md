# Architecture

MVP data path:

```text
Application / OS
 -> Capture Manager
 -> DNS Context Manager
 -> Flow Classifier
 -> Policy Engine
 -> RouteDecision
 -> PathPlan
 -> Secure Session
 -> Carrier
 -> WEAVE Node
 -> local egress | peer transit
 -> Destination
```

WEAVE is the symmetric mesh protocol for CHIMERA-PQ MVP. A node is not a
product-level client or server: the same node must simultaneously support local
ingress, peer ingress, local egress, and peer transit/forwarding when policy
allows it.

The canonical runtime role for the shipped peer-egress binary is `node`.
Legacy `client`, `gateway`, `server`, `side_a`, and `side_b` labels remain
transitional compatibility aliases only.

Transit payload is closed third-party information. A transit node validates only
the outer sealed frame envelope needed for safe forwarding, including the safe
`DATA`/`FIN` frame kind, packet number and length. The frame body stays opaque
bytes. Transit code must not decrypt, inspect, classify, log or export forwarded
payload contents.

Product code exposes this as `WeaveNodeContract::symmetric_mesh_node()` in
`crates/chimera-mesh/src/weave_contract.rs`. The same module exposes
`validate_weave_sealed_transit_frame` and `forward_weave_transit_frame`, which
operate on sealed bytes and redact the payload in debug output.

The carrier runtime node path uses a sealed-transit branch for local ingress:
when the first byte is a sealed frame version, the runtime validates the outer
envelope and forwards the sealed bytes unchanged to the next peer. The transit
branch does not decrypt or interpret the forwarded payload.

Current implementation status (fact-based):

- M0-M6 lab/verification contour is implemented and validated by project gates
  (`just mvp-check`, `just ship-readiness`, release/readiness artifacts).
- Default validation path remains network-safe (`network_state: not_modified`).
- Explicit runtime apply path exists for controlled smoke
  (`--apply-dns`, `--apply-route`, optional TUN apply path) and is validated
  with rollback artifacts.
- Full always-on OS-wide selective datapath orchestration for arbitrary apps is
  not declared as completed here; this documentation keeps that scope explicit.
