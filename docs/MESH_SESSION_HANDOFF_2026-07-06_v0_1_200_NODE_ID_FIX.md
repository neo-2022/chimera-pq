# MESH SESSION HANDOFF — 2026-07-06

## Active Objective

Fix RU discovery so it advertises the configured `CHIMERA_MESH_SELF_NODE_ID`
(`vdsina`) instead of the hostname fallback (`v3177669`).

## Status

- **Done**: `publish_mesh_discovery_snapshot` in `scripts/chimera-control.sh` now
  falls back to `CHIMERA_MESH_SELF_NODE_ID` from `peer-egress.env` when the value
  is not found in the runtime state file or upstream/bootstrap env.
- **Done**: `v0.1.200` signed tag created, release tarball built and deployed to
  NL and RU stand nodes; datapath verified both ways after restart.
- **Done**: GitHub release `v0.1.200` published and is now the latest release;
  assets `chimera-pq-release.tar.gz` and `.sha256` uploaded.

## Key Artifacts

| Item | Value |
|------|-------|
| Tag | `v0.1.200` |
| Main commit | `782d909` — `fix(control-plane): read CHIMERA_MESH_SELF_NODE_ID from peer-egress.env for discovery publish` |
| Release archive sha256 (tarball) | `28fc7afb9f8feb311b1cdf69142a548507f463fafb8477cc05c739a8d755d1e8` |

## Verification Evidence

Local checks:
```text
bash -n scripts/chimera-control.sh     # ok
```

RU discovery snapshot node_id after the fix:
```text
$ jq -r ".nodes[].node_id" ~/.cache/chimera/mesh_nodes.discovery.json
vdsina
```

NL discovery snapshot node_id (unchanged):
```text
$ jq -r ".nodes[].node_id" ~/.cache/chimera/mesh_nodes.discovery.json
amai
```

Hot-reload convergence after NL `chimera-node.service` restart still holds:
```text
# restart NL; poll from RU every 2 s
restart_issued
t=0s ip=
t=2s ip=
t=4s ip=
t=6s ip=91.124.19.180
converged
```

Datapath after v0.1.200 start (both directions):
```text
RU uid65534 -> ipinfo.io -> 91.124.19.180 (NL)
NL uid65534 -> ipinfo.io -> 138.16.175.96 (RU)
```

GitHub latest:
```text
$ gh release view --repo neo-2022/chimera-pq
title:  CHIMERA-PQ v0.1.200
tag:    v0.1.200
asset:  chimera-pq-release.tar.gz
asset:  chimera-pq-release.tar.gz.sha256
```

## Operational Notes

- Deployed via local-source path before the GitHub release was published; after
  the release was created, `chimera.sh -status` on the stand still reports the
  installed bundle as `v0.1.200` and its SHA-256 matches the GitHub asset.
- Stand install logs showed the update checker now sees GitHub latest as
  `v0.1.200`, so drift between stand and GitHub is closed.

## Risks / Known Limitations

1. `--all-targets` clippy still fails on pre-existing `unwrap()`/`expect()` in
   `chimera-cli` tests; product binaries are clean.
2. `chimera-site-watch.service` remains bound to `chimera-node.service`, so a
   full node restart recreates site-watch; hot reload fires on the new process’s
   initial cycle.

## Recommended Next Step

- Clean up remaining `--all-targets` clippy warnings in `chimera-cli` tests so
  the full workspace gate passes without exceptions.

## Safety

- No PC network/VPN/DNS/routes/firewall changed.
- Happ on current PC untouched.
- All practical checks executed via SSH only on authorized stand nodes.
