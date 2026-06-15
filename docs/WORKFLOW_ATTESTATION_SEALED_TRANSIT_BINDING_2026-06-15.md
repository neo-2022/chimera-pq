# Workflow Attestation: Sealed Transit Binding Dispatch

Scope: add a source-level carrier contract for explicit opaque sealed-transit
path binding and dispatch. This is not a real-world multipath runtime proof and
does not change installer/release scripts or local host network settings.

Stages:

- ANALYSIS: done
  - Evidence: `AGENTS.md`, `CHIMERA-PQ_MVP_SPEC.md`, `Agent.md`,
    `docs/EXECUTION_MODE_NO_TIMELINES.md`, latest handoff, current
    `chimera-carrier` and `chimera-session` sealed-transit code were read.
  - Result: current carrier fail-closes on multiple next hops without binding.
    The next MVP-aligned step is explicit route/lane binding for sealed transit,
    not payload inspection or implicit first-peer selection.
- PLAN: done
  - Result: add focused `peer_egress/transit_binding.rs` and
    `peer_egress/transit_dispatch.rs` modules, parse bound sealed-transit peer
    messages inside the authenticated secure payload, dispatch by opaque binding,
    and keep unbound ambiguous transit fail-closed.
- TEAM_CRITIQUE: done
  - Architect: recommended explicit sealed-transit path binding/dispatch.
  - Senior developer: recommended new focused carrier modules and no session
    changes.
  - Tester: required positive, negative and privacy tests for binding dispatch.
  - Security engineer: required missing/ambiguous/unknown binding fail-closed and
    no payload/destination/endpoint leakage.
  - DevOps: source-only checks are enough for this increment; release and
    laptop/VPS proof are required before shipped runtime claims.
  - Critic: accepted only as dispatch-contract progress, not real multipath
    runtime PASS.
- IMPLEMENTATION: done
  - Evidence:
    `crates/chimera-carrier/src/peer_egress/transit_binding.rs`,
    `crates/chimera-carrier/src/peer_egress/transit_dispatch.rs`,
    `crates/chimera-carrier/src/peer_egress/wire.rs`,
    `crates/chimera-carrier/src/peer_egress/transit.rs`,
    `crates/chimera-carrier/src/peer_egress/modes.rs`,
    `crates/chimera-carrier/src/peer_egress/mod.rs`.
  - Result: the carrier now has typed opaque route/lane binding, bound sealed
    transit peer-message parsing, explicit dispatcher lookup, fail-closed
    policy/dispatcher/unknown-binding behavior, and tests proving forwarded
    inner sealed bytes remain byte-identical.
- TEAM_CHECK: done
  - Architect: accepted opaque route/lane binding as the right carrier contract,
    but rejected any real datapath/pass claim until runtime binding registration
    and stand proof exist.
  - Senior developer: found compile blockers and required module wiring,
    exhaustive `PeerMessage` handling, redacted dispatcher debug, and a writer
    round-trip test.
  - Tester: required compile gates plus positive/negative/privacy tests for
    bound dispatch and explicit warning that this is not Real-World PASS.
  - Security engineer: accepted route/lane only as opaque metadata; required no
    payload/destination/endpoint leakage and fail-closed on missing/unknown
    binding.
  - DevOps: confirmed GitHub one-command release/update and laptop/VPS SSH
    evidence are required before shipped runtime claims.
  - Critic: rejected false PASS on parser-only work; required byte-identical
    forwarding, no pool fallback for bound transit, and dirty-tree caveat.
- FIX: done
  - Fixed missing `peer_egress` module declarations for `transit_binding` and
    `transit_dispatch`.
  - Fixed non-exhaustive `PeerMessage::BoundSealedTransit` handling in carrier
    modes and transit piping.
  - Added `forward_bound_peer_sealed_transit_to_next_hop` and shared
    pair-forwarding so bound dispatch uses a matching dispatcher binding and
    never falls back to arbitrary pool selection.
  - Added redacted manual `Debug` for `TransitNextHopDispatcher`.
  - Added writer round-trip coverage for bound sealed transit under tests only;
    production keeps it closed until a real runtime sender uses it.
- RECHECK: done
  - `cargo fmt --all -- --check`: PASS.
  - `cargo check -q --workspace`: PASS.
  - `cargo test -q -p chimera-carrier`: PASS, 69 tests.
  - `cargo test -q -p chimera-session sealed_transit`: PASS, 3 tests.
  - `cargo test -q -p chimera-mesh`: PASS, 159 tests.
  - `cargo clippy -q -p chimera-carrier --all-targets -- -D warnings`: PASS.
  - `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`: PASS.
  - `bash scripts/anti_monolith_guard.sh`: PASS.
- FINAL_AUDIT: done
  - No CHIMERA runtime was started, stopped or restarted on the current PC.
  - No local DNS, route, firewall, proxy, VPN, Happ, MYVPN, router, notebook or
    VPS setting was changed.
  - Current status is source/unit/lab verified only. Real node-to-node
    carrier-bound sealed transit and GitHub one-command stand update remain
    unverified for this change.
- REPORT: done
  - Status: source-level sealed-transit binding dispatch contract is verified.
  - Not closed: real runtime registration of binding to next-hop, release
    publication, GitHub Latest verification, and SSH stand proof on notebook/VPS.

Runtime/network statement:

- Local CHIMERA runtime start/stop is not part of this change.
- Local DNS, routes, firewall, proxy, Happ, MYVPN, VPN, router, laptop and VPS
  settings are out of scope for this source-level carrier contract update.
- This is not a Real-World PASS and not a milestone close.
