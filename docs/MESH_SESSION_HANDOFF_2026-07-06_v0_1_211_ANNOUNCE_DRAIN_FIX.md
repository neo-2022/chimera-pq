# MESH_SESSION_HANDOFF_2026-07-06_v0_1_211_ANNOUNCE_DRAIN_FIX

**session_id:** handoff-2026-07-06-211-announce-drain-fix  
**version:** 0.1.211 (target next: 0.1.212 after verification)  
**status:** in_progress  

## Objective

Replace the temporary mesh-announcement workaround on the SSH-only stand with a
code fix: the native `CHIMERA-LOCAL/1 CONNECT` path must tolerate buffered
route `ANNOUNCE` messages that a peer node sends immediately after secure peer
authentication, before the expected `AckOk` reply.

Once fixed and verified, both directions (`amai → vdsina` and `vdsina → amai`)
must pass sealed multi-hop echo probes with normal route-announcement
configuration restored on both nodes.

## Stand / Runtime Context

- Product repository: `<redacted-path>`, branch `main`.
- Registered Amai project code: `chimera`.
- Stand nodes (addresses/logins/seeds stored in operator-local files, not in
  product):
  - NL `amai`: public address on file; runtime `<redacted-path>`;
    CLI `<redacted-path>`; peer-egress listener `<redacted-ip>`;
    local ingress `<redacted-ip>`.
  - RU `vdsina`: public address on file; runtime `<redacted-path>`;
    peer-egress listener `<redacted-ip>`; outbound bootstrap
    `amai@<redacted>:8448`.
- Common token, Ed25519 seeds, `peer-egress.env`, `mesh_bootstrap.env`,
  `mesh-node.conf` are already configured on both nodes.
- `chimera-control.sh start` is the current restart path (it bypasses the
  GitHub update gate that can time out on the stand).
- `CHIMERA_SERVICE_FWMARK=0x5244`, `CHIMERA_PEER_EGRESS_CONNECTIONS=8`.
- Echo responder on `vdsina:<redacted-ip>` was left running.

## What Already Works

- v0.1.211 is installed on both stand nodes; bundle SHA matches the previous
  release evidence (`583ec8a...`).
- Sealed transit lane registration and peer streams are active.
- `amai → vdsina` `CHIMERA-LOCAL/1 CONNECT` probe succeeds, but only because
  `vdsina` currently has its mesh announcements disabled as a workaround.

## Active Issue / Root Cause

`node.rs` sends any configured local route `ANNOUNCE` message to the remote
peer immediately after authentication and before pushing the peer into the
pool:

```rust
if !local_announcements_for_ingress.is_empty() {
    if let Err(error) = write_announce_message(&mut peer, &local_announcements_for_ingress) { ... }
}
if let Err(error) = peer_ingress_pool.push(peer) { ... }
```

When the native CONNECT handler on the *initiator* side pops a freshly
registered outbound sealed-transit lane peer and writes `CONNECT`, it then
expects `AckOk`. If the remote side already wrote an `ANNOUNCE` frame on that
same secure stream (because the remote node has route announcements enabled),
`require_peer_ack` sees `PeerMessage::Announce` first, errors out, and the
peer is marked dead.

The workaround on `vdsina` was to empty
`CHIMERA_MESH_ANNOUNCEMENT_SIGNING_KEY` and `CHIMERA_MESH_POLICY_PAYLOAD`, so
`vdsina` never sends an `ANNOUNCE`. This blocks normal route propagation in the
reverse direction and is **not** the final state.

## Code Change Already Applied

Modified:
`crates/chimera-carrier/src/peer_egress/modes_local_ingress.rs`

`require_peer_ack` now:

1. Sets a read timeout from `CHIMERA_PEER_EGRESS_HANDSHAKE_TIMEOUT_MS`
   (default 6000 ms).
2. Loops reading peer messages.
3. Skips up to 64 buffered `PeerMessage::Announce` frames.
4. Returns `Ok(())` on the first `PeerMessage::AckOk`.
5. Errors on any other message type.
6. Clears the read timeout before returning.

This mirrors the server-side behaviour in `modes.rs` where inbound pool/outbound
workers already skip `Announce` frames before reading the first `Connect` /
`SealedTransit` request.

Also modified:
`crates/chimera-carrier/src/peer_egress/modes_local_ingress_tests.rs`

- Import scaffolding added for `connect_local_client_via_peer`,
  `write_announce_message`, and the route-announcement types.
- A concrete test that sends `ANNOUNCE` before `OK` is **not yet added**.

## Verification / Next Steps (must be completed in the next session)

1. **Finish the unit test**
   - Add a test that creates a `SecurePeerStream` pair, sends one `ANNOUNCE`
     frame from the fake remote side, then sends `OK`, and confirms that
     `connect_local_client_via_peer` returns successfully and writes `OK\n` to
     the local client.

2. **Local checks (on PC, no network changes)**
   ```bash
   cd <redacted-path>
   just fmt
   just lint
   just check
   just test
   ```

3. **Build release bundle locally**
   ```bash
   just release-build
   # or: scripts/build_release.sh
   ```

4. **Deploy to stand**
   - Copy the patched `chimera-peer-egress` / `chimera-node` / bundle to both
     stand nodes into `<redacted-path>` and `<redacted-path>`
     as required.
   - On `vdsina`, restore normal route-announcement configuration
     (`CHIMERA_MESH_ANNOUNCEMENT_SIGNING_KEY` and
     `CHIMERA_MESH_POLICY_PAYLOAD`) from the operator-local seed files.
   - Restart both nodes with `chimera-control.sh start`.

5. **Run bidirectional probes**
   - `amai → vdsina` to `<redacted-ip>`.
   - `vdsina → amai` to a temporary echo responder or known service on `amai`.
   - Confirm both `PROBE_OK` and payload round-trip.

6. **If probes fail**
   - Collect `journalctl --user -u chimera-node.service` and
     stderr from both nodes.
   - Check for the new log event `peer_ack_announce_skipped`.
   - Do not re-apply the workaround; debug the drain logic instead.

7. **Evidence & handoff**
   - Run the `chimera-ssh-stand-release-proof` skill.
   - Update `MESH_SESSION_HANDOFF_*` with probe results, version, and SHA.
   - Commit the code fix with GPG signature if CI/local gates pass.

## Safety / Constraints

- PC remains a control/build point only; no local runtime checks.
- Do not modify PC network, DNS, proxy, firewall, routes, or Happ.
- Do not hard-code stand addresses, credentials, seeds, or tokens into product
  code, configs, or this handoff.
- All SSH work follows the authorized stand rules; use the operator-local
  credential file only as a working material and never expose it in logs or
  commits.

## Files of Interest

- `crates/chimera-carrier/src/peer_egress/modes_local_ingress.rs`
- `crates/chimera-carrier/src/peer_egress/modes_local_ingress_tests.rs`
- `crates/chimera-carrier/src/peer_egress/node.rs`
- `crates/chimera-carrier/src/peer_egress/modes.rs`
- `crates/chimera-carrier/src/peer_egress/live_bindings.rs`
- `crates/chimera-carrier/src/peer_egress/transit_dispatch.rs`
- Stand setup scripts (operator-local, not in product):
  `setup_amai.sh`, `setup_v3177669.sh`
