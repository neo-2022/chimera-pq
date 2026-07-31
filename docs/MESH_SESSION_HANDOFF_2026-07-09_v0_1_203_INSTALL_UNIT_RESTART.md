# MESH_SESSION_HANDOFF_2026-07-09_v0_1_203_INSTALL_UNIT_RESTART

**session_id:** handoff-2026-07-09-203-inst
**version:** 0.1.203
**status:** partial

## Summary

Closed four regressions that blocked stable mesh publishing/node restarts:

- **Hot lane-document reload** (`scripts/chimera-control.sh`): sha256 hot checksum now recomputed every second from `peer-egress.state` only (removed `peer-update.state.json` from hot-trigger to stop self-exciting publish loops). Discovery poll interval default reduced from 30 s to 10 s (`crates/chimera-carrier/src/peer_egress/options.rs`).
- **node_id fallback** (`scripts/chimera-control.sh::publish_mesh_discovery_snapshot`): now reads `CHIMERA_MESH_SELF_NODE_ID` from `peer-egress.env` when deriving the origin node id, so RU advertises `vdsina` instead of hostname `v3177669`.
- **site-watch survives node restarts** (`deploy/systemd-user/chimera-site-watch.service`): removed `BindsTo=chimera-node.service` and changed `PartOf` to `chimera-runtime.service`, so systemd no longer kills site-watch when node restarts.
- **fixed `peer.listen_addr` honored with discovery** (`scripts/chimera-control.sh::heal_node_peer_egress_env_bindings`): when mode is `node` and discovery is enabled, the configured explicit `peer.listen_addr` is preserved in `CHIMERA_PEER_EGRESS_PEER_LISTEN` instead of being overwritten to `<redacted-ip>`. `auto`/empty still produce `<redacted-ip>` as before.
- **install auto-restart** (`scripts/install_desktop_control.sh`): after copying units and `daemon-reload`, active `chimera-node`, `chimera-datapath` and `chimera-site-watch` units are restarted so new unit definitions apply immediately.
- **Clippy `--all-targets` clean**: removed all `unwrap()`, `expect()` and `unwrap_err()` calls from tests in `chimera-cli`, `chimera-lab`, and `chimera-carrier`; CI `--all-targets` passes.

## Stand Evidence

- NL (`<redacted-ip>`, node_id `amai`) and RU (`<redacted-ip>`, node_id `vdsina`) both report `chimera-runtime 0.1.203` after install.
- `site_auto_watch_status=running hot_interval_sec=1` confirmed on both nodes.
- Fixed `peer.listen_addr` experiment:
  - Set RU `peer.listen_addr = <redacted-ip>`.
  - NL observed RU peer spec `<redacted-ip>`.
  - Bidirectional datapath with `--new-uid` succeeded.
  - Reverted RU to `peer.listen_addr = auto` so the stand now runs in the standard auto mode.

## Open Items / Blockers

1. **Network instability between NL and RU refused to clear during this session:**
   - SSH logins and TLS handshakes repeatedly showed `Connection timed out`, `SSL_ERROR_SYSCALL`, `unexpected eof while reading`.
   - These symptoms occurred across multiple hops (laptop → NL, laptop → RU, NL → RU, RU → NL) and coincided with period when both nodes reported connection failures.
   - Stand eventually returned to accepting connections, but the flapping prevents a clean end-to-end bidirectional datapath proof comparable to v0.1.198.

2. **site-watch loop failures still observed in logs:**
   - `site_auto_watch_loop=fail consecutive_failures=3 failure_budget=3` appears intermittently.
   - systemd restart recovers the service each time, but the failures correlate with the same network flapping/discovery/CLI errors and are not yet root-caused.

3. **End-to-end datapath stamp not final:**
   - Need final confirmation that NL UID 65534 → RU IP and RU UID 65534 → NL IP are stable, as they were after v0.1.198.

## Git / Release Status

- Local branch `main`: clean (no uncommitted changes).
- Tags `v0.1.199`, `v0.1.200`, `v0.1.201`, `v0.1.202`, `v0.1.203` pushed and signed (`chimera-bot` GPG key `B6A1A3303D939125`).
- GitHub release `v0.1.203` published with `chimera-pq-release.tar.gz` + `.sha256`.
- Hash of release tarball: `6548041c2ea9fa5e9f78fc94f0669bff95d1a43b93d4b5c1773fe2c024298525`.

## Next Recommended Steps

1. Schedule a quiet window to re-run full bidirectional datapath proof on a stable network path.
2. Capture `chimera-site-watch` debug logs during the instability to determine whether the loop failures are solely network-induced or have another source.
3. If the network path stays flaky, consider running both nodes with `CHIMERA_PEER_EGRESS_CARRIER_LOG_LEVEL=debug` and capturing a synchronized packet/CLI trace from both sides.
4. Once bidirectional datapath is stable, update `CURRENT_WORKLINE_ATTESTATION.json` to `status=ok` and produce the final handoff.

## Attestation

- `CURRENT_WORKLINE_ATTESTATION.json` points to this file and remains `status=partial` until the end-to-end proof is re-established.
