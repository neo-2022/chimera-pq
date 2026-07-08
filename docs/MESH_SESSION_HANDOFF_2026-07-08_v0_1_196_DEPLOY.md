# MESH SESSION HANDOFF — 2026-07-08

## Active Objective

Cut, sign and deploy CHIMERA-PQ `v0.1.196` to the authorized SSH-only stand and
prove live datapath both ways.

## Status

- **Done**: `v0.1.196` signed tag created, local release tarball built, deployed
to NL and RU stand nodes, services restarted and active.
- **Done**: live capture-UID `curl` evidence obtained both directions:
  - NL UID 65534 -> RU public IP.
  - RU UID 65534 -> NL public IP.
- **Done**: negative-path verified (stopping datapath breaks UID capture,
restart restores it).
- **Done**: post-work verification skill updated to v3.0
  (`/home/art/.codex/skills/chimera-post-work-verification-guard/SKILL.md`).

## Key Artifacts

| Item | Value |
|------|-------|
| Tag | `v0.1.196` |
| Main fix commit | `1f1aba0` — `fix(mesh): accept mesh_traffic_profile policy shorthand and align control-plane bindings from discovery` |
| Style/lint follow-up | `0d63f51` — `style(capture): replace unwrap_err() in transparent runtime/tcp tests to satisfy clippy unwrap_used` |
| Release archive sha256 (tarball) | `a8ed7d6b03800545a75bbe33e32d5f0d7e6cc5ae611f13ed8e21afeb528ab3a6` |
| Installed `chimera-node` sha256 (both nodes) | `0cf56a050c4d4e682c480111c7f21003d286bbdce79a81c9c415927b50762e65` |

## Verification Evidence

Local checks:
```text
cargo fmt --check    # pass
cargo test --workspace # pass
cargo clippy --workspace --bins --release -- -D warnings # pass
# Note: --all-targets clippy still fails on pre-existing test unwraps/expect;
#       product binaries are clean.
```

Stand services (both nodes):
```text
systemctl --user is-active chimera-runtime chimera-node chimera-datapath chimera-site-watch
active active active active
```

NL capture UID curl (returns RU IP):
```text
$ setpriv --reuid=65534 --regid=65534 --init-groups curl --max-time 15 -4 -sS http://ipinfo.io
{
  "ip": "138.16.175.96",
  ...
}
```

RU capture UID curl (returns NL IP):
```text
$ setpriv --reuid=65534 --regid=65534 --init-groups curl --max-time 15 -4 -sS http://ipinfo.io
{
  "ip": "91.124.19.180",
  ...
}
```

Negative path:
```text
$ systemctl --user stop chimera-datapath
$ setpriv --reuid=65534 ... curl http://ipinfo.io
curl: (7) Failed to connect to ipinfo.io port 80 ...
$ systemctl --user start chimera-datapath
# curl resumes returning remote IP
```

## Operational Work Done on Stand (not product commits)

- The reinstall cleared runtime state.
- RU discovery advertised node_id `v3177669` (hostname default). Remote specs
  and lane documents were aligned to the current discovery endpoint, not the
  operator label `vdsina`. This is a stand-only label mismatch; auth is
  token-based and unaffected.
- `CHIMERA_PEER_EGRESS_PEER_LISTEN` is dynamic (`0.0.0.0:0`) when discovery is
  configured, so the actual peer port changed after each `chimera-node` restart.
  After each restart, `publish_peer_egress_transit_lane_bindings_from_control_plane`
  had to be re-run on the *other* node so that its lane document pointed at the
  current dynamic port.

## Risks / Known Limitations

1. **Dynamic peer port staleness**: because `heal_node_peer_egress_env_bindings`
   forces `0.0.0.0:0` whenever a discovery source is configured, the peer listen
   port is random. After a node restart, the partner node’s lane document may
   still reference the old port until the control-plane publish function is run
   again. The current code fix refreshes the remote peer spec from discovery,
   but the refresh is not automatic immediately after a peer port change.
2. **Operator node_id label drift**: discovery publish falls back to hostname if
   `CHIMERA_MESH_SELF_NODE_ID` is not in the discovery publisher’s upstream env.
   On the stand this produced `v3177669` instead of the desired `vdsina`. This
   does not break auth, but it makes remote specs use the hostname id.

## Recommended Next Step

Decide whether to fix the product so that:

- a configured fixed `peer.listen_addr` is honoured even when discovery is enabled,
  OR
- discovery/control-plane automatically republishes transit lane bindings as
  soon as the local peer listen port changes.

Either change would remove the need for manual post-restart refresh on the stand.

## Safety

- No PC network/VPN/DNS/routes/firewall changed.
- Happ on current PC untouched.
- All practical checks executed via SSH only on authorized stand nodes.
