# MESH_SESSION_HANDOFF_2026-07-10_v0_1_203_DATAPATH_FIX_ATTEMPT

**session_id:** handoff-2026-07-10-203-datapath-fix-attempt
**version:** 0.1.203
**status:** partial

## Stand State (end of session)

- Both authorized stand nodes reinstalled from the published GitHub release `v0.1.203` tarball (`6548041c2ea9fa5e9f78fc94f0669bff95d1a43b93d4b5c1773fe2c024298525`).
- Node identities: `amai` (NL), `vdsina` (RU).
- `chimera-runtime.service`, `chimera-node.service`, `chimera-datapath.service`, `chimera-site-watch.service` active on both nodes.
- `runtime_boot_enabled_state=enabled` on both nodes (RU fixed earlier in the session).
- `CHIMERA_PEER_EGRESS_POOL=8` and `CHIMERA_PEER_EGRESS_CONNECTIONS=8` preserved by the installer from the previous env state.
- PC remains control-only.

## Code Changes Applied

Commit `672bdb0` (signed by `chimera-bot` GPG key `B6A1A3303D939125`):

```text
fix(carrier/capture): discard dead mesh peers and retry transparent transit handshake

- PeerPool: add pop_wait_timeout/pop_wait_timeout_for_flow_key so callers
  can bound how long they wait for a live peer.
- modes_local_ingress: when using the plain peer pool, keep trying peers
  within a deadline and drop dead ones before sending OK to the client.
- chimera-transparent-tcp: retry the local transit CONNECT handshake
  (configurable via CHIMERA_TRANSPARENT_TCP_CONNECT_RETRY_COUNT/DELAY_MS).
- Improves resilience against cold-start and transient peer-pool dead peers
  observed in NL/RU v0.1.203 datapath tests.
```

Files changed:
- `crates/chimera-carrier/src/peer_egress/pool.rs`
- `crates/chimera-carrier/src/peer_egress/modes_local_ingress.rs`
- `crates/chimera-capture/src/bin/chimera-transparent-tcp.rs`

All local tests pass (`cargo test -p chimera-carrier -p chimera-capture --all-targets`) and `cargo clippy --all-targets -p chimera-carrier -p chimera-capture` is clean.

## Deployment Attempt

- Hot-swapped debug builds of `chimera-peer-egress` and `chimera-transparent-tcp` on NL/RU.
- With `POOL=8` and the new code, NL reached 15/15 on `ifconfig.me`, `ipinfo.io`, and `icanhazip.com` in one run.
- RU showed strong improvement on `ipinfo.io` (15/15 in one spot-check) and `ifconfig.me` (20/20 spot-check), but `icanhazip.com` remained mixed.
- The NL↔RU carrier path is unstable: NL SSH login timed out multiple times during the session, independent of CHIMERA state. This made it impossible to obtain a clean, reproducible 20-run bidirectional proof after the code change.
- To avoid leaving the stand on mixed debug/release binaries, both nodes were reinstalled back to the signed `v0.1.203` release.

## Baseline After Reinstall

Test method: `runuser -u nobody -- curl -sS <target>`, 10 runs per target.

| Direction | ipinfo.io | ifconfig.me | icanhazip.com |
|-----------|-----------|-------------|---------------|
| RU → NL   | 7/10      | 10/10       | 6/10          |
| NL → RU   | not captured (SSH timeout) | not captured | not captured |

The RU baseline shows the same residual target-specific flakiness that the code change reduced but did not fully eliminate.

## Root-Cause Observations

1. **Peer pool was configured to 1** on the original `v0.1.203` install (`CHIMERA_PEER_EGRESS_POOL=1`). This leaves only a single inbound peer per direction; if it is dead or reconnecting, every request fails. The installer preserved existing env values, so this stuck at 1 across reinstalls until it was manually bumped to 8 in this session.
2. **Production local-egress path uses lane-bound transit dispatcher**, not the plain `SharedPeerPool` fallback that was hardened. The lane-document/dispatcher path does not currently discard dead peers or retry, so empty-reply failures can still happen there.
3. **NL↔RU carrier path flaps**: SSH logins, TLS handshakes, and datapath requests all show intermittent `Connection timed out` and `Empty reply` symptoms. This is consistent with the network flapping noted in the previous handoff and explains why even the `ifconfig.me` spot-checks are not 100% reliable across long runs.

## Residual Work

- Harden the **lane-document dispatcher path** (`handle_local_client_with_lane_document_and_first_byte`) so it also discards dead peers and retries, or add a keepalive/health-check before a binding is returned.
- Decide on a product-level default/minimum for `CHIMERA_PEER_EGRESS_POOL` so a single dead peer cannot black-hole all egress.
- Wait for a stable network window and re-run the full NL↔RU bidirectional proof with a release that includes commit `672bdb0`.

## Attestation

- `CURRENT_WORKLINE_ATTESTATION.json` updated to point to this handoff.
- Status remains `partial` because the stand network instability prevents a clean, reproducible end-to-end datapath proof, and the lane-document dispatcher path is not yet hardened.
