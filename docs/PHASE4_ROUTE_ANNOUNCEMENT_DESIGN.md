# Phase 4 — Capability-Based Route Announcement (control plane design)

**Status:** design only — not implemented in Phase 3.

**Purpose:** give the mesh control plane a safe, explicit way to advertise
transitive reachability so that a node behind NAT (or any node that does not
publish a direct carrier endpoint) can be reached through a chosen transit
peer. This replaces manual route-id guessing and hidden/internal binding hacks.

## Core abstraction

```rust
use std::time::Duration;

/// A capability-signed route advertisement from one peer to another.
/// Implemented inside `chimera-mesh` or `chimera-policy` (control plane).
pub enum RouteAnnouncement {
    Static {
        /// Destination CIDR or domain that the `via` peer claims it can reach.
        destination: RouteDestination,

        /// Explicit transit peer. The node receiving this announcement forwards
        /// traffic for `destination` to this peer, never to an inferred one.
        via: PeerId,

        /// Mandatory expiration. Static routes in a mesh are operational risk;
        /// they must not persist silently.
        ttl: Duration,

        /// Cryptographic attestation that `via` itself agreed to forward.
        auth: CapabilityToken,
    },
}

pub enum RouteDestination {
    Cidr(IpCidr),
    Domain(String),
}

/// Stable public node identifier. Must not be internal planner route id.
pub struct PeerId(String);

/// Signed capability issued by the transit peer.
pub struct CapabilityToken {
    issuer: PeerId,
    scope: CapabilityScope,
    signature: Vec<u8>,
}

pub enum CapabilityScope {
    ForwardTrafficTo(RouteDestination),
}
```

## Required security properties

1. **Explicit, not implicit**
   - The `via` field names a single concrete peer. There is no algorithmic
     inference that "because one peer can see a NATed node, another peer should
     use it" unless the
     operator or the peer explicitly publishes that fact.

2. **Bounded TTL**
   - Every static announcement expires. Missing TTL is a parse error, not a
     default. Renewal must be an explicit re-announcement.

3. **Capability attestation**
   - The peer named in `via` must sign `CapabilityScope::ForwardTrafficTo(dest)`
     with its own key. Other nodes must verify the signature before admitting
     the announcement into `MeshPathPlan` or any transit lane document.

4. **No internal route-id surface**
   - Operators and API consumers use `PeerId` and `CIDR`/`domain`. The
     `route_binding_id` / `lane_index` encoding remains an internal planner
     detail and must never be accepted from untrusted input.

5. **Revocation**
   - A peer may publish a superseding announcement with a shorter TTL or a
     `Revoke` variant. The design reserves `RouteAnnouncement::Revoke { ... }`
     for future versions.

## Lifecycle

```text
via peer (e.g. RU)                     control plane on (e.g. NL)
       |                                      |
       | sign RouteAnnouncement               |
       |------------------------------------->| store in pending queue
       |                                      | verify signature + TTL
       |                                      | admit into MeshPathPlan
       |                                      | rebuild TransitLaneDocument
       |                                      |   -> binding route -> via
       |                                      v
                           local ingress sees flow to 10.42.0.31
                           planner selects routed lane via RU
```

## Files likely touched

- `crates/chimera-mesh/src/route_announcement.rs` — model, validation,
  capability verification.
- `crates/chimera-mesh/src/multipath_model.rs` — extend `MeshCarrierLaneBinding`
  or add a separate routed-reachability table so planner can emit a transitive
  lane.
- `crates/chimera-carrier/src/peer_egress/lane_document.rs` — consume transitive
  reachability when building `TransitLaneDocument`.
- `crates/chimera-cli/src/main.rs` — add `route-announce` / `route-revoke`
  subcommands, gated by operator auth.

## Out of scope for Phase 4 design

- Distributed consensus on which peer may announce which destination.
- Economic/reputation weighting of announcements.
- Fully automated discovery without invitation/seed control.

These remain post-MVP per `AGENTS.md` §3.
