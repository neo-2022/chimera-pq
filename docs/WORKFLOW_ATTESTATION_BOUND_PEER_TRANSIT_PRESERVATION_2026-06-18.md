# Workflow Attestation: Bound Peer Transit Preservation

Status: source_level_pass
Date: 2026-06-18

## Objective

Close the source-level carrier defect where peer-bound sealed transit selected a
next hop by binding but then forwarded the stream as unbound sealed frames.

This is a source/lab proof only. It is not a runtime stand PASS, not a
Real-World PASS, not TUN/DNS/split/rollback proof, and not a browser/IDE
workflow proof.

## Council Result

Real sub-agent roles were used for architecture, senior development, testing,
security, DevOps, and critic review.

Accepted:

- bound peer transit must preserve the bound envelope in both directions;
- source-to-next and next-to-source directions must accept only
  `BoundSealedTransit` with the same `TransitPathBinding`;
- unbound sealed frames, CONNECT, ACK, and midstream binding changes must
  fail closed;
- implementation belongs in a small carrier module, not by growing
  `transit.rs`;
- source gates are not a substitute for release or SSH stand evidence.

Rejected:

- keeping reverse direction sealed-only;
- calling source/lab proof a runtime or Real-World PASS;
- using an unbound pool fallback for bound transit;
- logging route/lane ids, endpoints, tokens, or payload markers.

## Source Changes

Changed files:

```text
crates/chimera-carrier/src/peer_egress/bound_transit.rs
crates/chimera-carrier/src/peer_egress/mod.rs
crates/chimera-carrier/src/peer_egress/transit.rs
crates/chimera-carrier/src/peer_egress/transit_tests/forwarding.rs
crates/chimera-carrier/src/peer_egress/transit_tests/helpers.rs
crates/chimera-carrier/src/peer_egress/transit_tests/midstream.rs
crates/chimera-carrier/src/peer_egress/transit_tests/mod.rs
crates/chimera-carrier/src/peer_egress/transit_tests/reverse.rs
```

Behavior:

- `forward_bound_peer_sealed_transit_to_next_hop` no longer strips the bound
  envelope through `first.into_frame()`.
- `forward_bound_peer_transit_pair` forwards bound-encoded frames in both
  directions.
- Both directions enforce one binding for the stream.
- The new helper logs only event name and direction, not binding values or
  payload.

## Tests Added Or Strengthened

- selected dispatcher next-hop receives bound-encoded frames;
- wrong registered next-hop receives no payload;
- source-to-next and next-to-source preserve bound magic, binding, and sealed
  bytes;
- same binding can be reused only after dispatcher replenishment;
- source-to-next binding change fails closed;
- source-to-next unbound frame fails closed;
- source-to-next CONNECT and ACK fail closed;
- reverse binding change fails closed;
- reverse unbound frame fails closed;
- payload markers and raw route/lane strings are absent from checked errors and
  debug output.

## Evidence

Passed:

```text
cargo fmt --all -- --check
cargo check -q --workspace
cargo test -q -p chimera-carrier transit
cargo test -q -p chimera-carrier peer_egress::transit::transit_tests::reverse::bound_peer_transit_rejects_reverse_binding_change
cargo test -q -p chimera-carrier
cargo test -q --workspace
cargo clippy -q -p chimera-carrier --all-targets -- -D warnings
cargo clippy -q --workspace --all-targets -- -D warnings
bash scripts/anti_monolith_guard.sh
just rust-no-hardcode-guard
bash scripts/chimera_installer_gate.sh
bash scripts/chimera_update_contract_smoke.sh
bash scripts/chimera_start_contract_smoke.sh
bash scripts/chimera_stop_contract_smoke.sh
git diff --check
git diff --cached --check
```

Observed test counts:

```text
chimera-carrier transit: 58 passed
chimera-carrier full: 120 passed
workspace tests: passed
```

## Not Closed

- GitHub Release/Latest publication for this change;
- laptop/VPS one-command GitHub install/update proof;
- live laptop/VPS bound multi-hop sealed transit delivery proof;
- transparent TUN/OS routing proof;
- DNS-to-route runtime binding proof;
- split-mode failover/direct-preservation proof;
- normal and forced-stop rollback proof;
- browser/IDE normal workflow proof;
- long-run/load/performance proof.

## Risks And Limits

- Mixed old/new peer versions may fail closed until all participating peers
  speak the bound-preserving stream contract.
- The release workflow still has the previously known infrastructure risk
  around package installation; if CI release blocks again, manual release from
  the exact pushed tag requires asset and checksum verification.
- This evidence is source-level. Runtime claims require a published release and
  SSH-only stand proof on laptop/VPS.
