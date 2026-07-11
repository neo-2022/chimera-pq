# Phase 3 — Sealed Multi-Hop Transit Proof Plan

**Goal:** demonstrate on the live remote stand that a CHIMERA transit node
forwards sealed opaque bytes to another CHIMERA node and never inspects the
payload.

**Stand topology:**

- `<nl-node>` — public seed / publisher (`amai`)
- `<ru-node>` — public seed / publisher (`vdsina`)
- `<laptop-node>` — NATed non-publishing member (`laptop`)

The desired path is:

```text
NL(source) -> RU(transit) -> laptop(destination)
```

RU must receive a sealed frame from NL and forward it to the laptop without
knowing the payload.

## Why the first attempt failed

A live CHIMERA mesh node does not automatically accept arbitrary IP destinations
as mesh destinations. The transparent runtime intercepts only configured
capture domains and turns them into mesh flows. Peer egress then selects a
peer based on the current `TransitLaneDocument`. Because the laptop is behind
NAT, the planner does not advertise a direct carrier endpoint for it, so NL
has no admitted lane for a flow bound to `192.168.31.31`. The result was a
DNS/tunnel timeout, not a forwarding failure.

## Correct mechanism: bound transit

`chimera-peer-egress` already supports two modes for this:

- `sealed-transit-inject` — writes a raw sealed transit frame to local ingress.
- `bound-transit-inject` — writes a bound sealed transit frame that explicitly
  carries `(route_id, lane_index)` telling the receiving node which peer to
  forward to.

The peer egress options leak internal route IDs, so an external harness should
not guess them. Instead it should use one of these operator-level approaches:

### Option A — use mesh self-healing / failover planning

`chimera-cli route-explain` can export a transit lane document from a failover
or cooldown plan (`--failed-node`, `--cooldown-node`). If we configure the mesh
planner so that a flow to a destination that is only reachable via RU produces
a lane document where RU is the active next hop, then running the same flow
through NL results in NL -> RU -> destination. This requires a destination that
is advertised through RU.

### Option B — advertise the laptop as a bound-transit service

If the laptop publishes a bound route (e.g. `192.168.31.31/32` or a domain)
through RU, RU becomes a transit advertiser. NL can then send sealed frames
for that route to RU, and RU forwards to the laptop over its existing
reverse-connection from the laptop.

### Option C — controlled endpoint behind NAT

The simplest safe runtime proof is to run a small TCP echo server on the
laptop bound to a CHIMERA-managed destination (for example a domain that
resolves to the laptop's LAN IP), configure the capture domain on NL to
include that domain, and ensure the planner admits a lane via RU. RU then
forwards the sealed TCP stream to the laptop.

## Recommended harness (Option C)

1. **On laptop**
   - Start an HTTP/TCP echo server on a LAN address.
   - Register a static host/DNS mapping so CHIMERA sees the destination as a
     named domain.

2. **On RU**
   - Ensure `allow_bound_transit=true` (already set).
   - Optionally add a bound transit advertisement so NL knows that the laptop
     destination is reachable via RU.

3. **On NL**
   - Add the laptop domain to `CHIMERA_CAPTURE_DOMAIN`.
   - Add a DNS binding or `/etc/hosts` entry for the laptop domain.
   - Wait for the planner to build a lane to RU for this destination.
   - Run `runuser -u nobody curl http://<laptop-domain>:<port>/...`.

4. **Evidence**
   - On RU collect `chimera_node.service.log`. Expect to see sealed transit
     events (`event=peer_sealed_transit`, `event=peer_pool_request_received`,
     `event=peer_pool_target_connecting`, `event=peer_pool_connect_ack_sent`)
     with redacted destination. The payload bytes must NOT appear in the log.
   - The `curl` on NL should receive the echoed response from the laptop.

5. **Rollback**
   - Remove the capture domain entry.
   - Remove the host/DNS binding.
   - Restart `chimera-datapath`.
   - Stop the laptop server.

## Missing pieces

The current MVP configuration does not expose a simple runtime command to
advertise a bound route from one node to another. A proper Phase 3 harness needs
one of the following:

- A CLI command or RPC to bind a destination CIDR/domain to a peer on a node.
- A documented transit lane bindings format and a generator that can produce a
  valid document from operator inputs.

This plan is intentionally read-only / design-only until one of those pieces is
available.

## Notes

- Stand addresses, ports and secrets are not hardcoded in this document; the
  harness that implements this plan will read them from environment variables.
- No live stand network state was permanently changed during earlier
  experiments.
