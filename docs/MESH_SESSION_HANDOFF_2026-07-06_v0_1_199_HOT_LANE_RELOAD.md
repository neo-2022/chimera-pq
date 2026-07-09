# MESH SESSION HANDOFF — 2026-07-09

## Active Objective

Implement hot lane-document reload on peer endpoint port change so partner
nodes converge without waiting for the next `site-watch` cycle.

## Status

- **Done**: `scripts/chimera-control.sh` now hot-watches `peer-egress.state` and
  `peer-update.state.json`. A checksum is taken every second; any change fires a
  full `site_auto_watch_run_once` cycle immediately instead of sleeping through
  the 60-second interval. A diagnostic marker is logged when the hot publish
  triggers.
- **Done**: default mesh lane driver discovery poll interval lowered from
  `30s` to `10s` so partner nodes rebuild their lane document faster after a
  discovery snapshot update.
- **Done**: `v0.1.199` signed tag created, release tarball built, deployed to NL
  and RU stand nodes, services active.
- **Done**: convergence verified after node restart:
  - Restarting NL `chimera-node.service` ⇒ RU UID-capture curl returned NL public
    IP in ~6 seconds.
  - Restarting RU `chimera-node.service` ⇒ NL UID-capture curl returned RU public
    IP in ~11 seconds.
- **Done**: local gates pass:
  - `cargo fmt --check` — pass.
  - `cargo test --workspace` — pass.
  - `cargo clippy --workspace --release` — pass.

## Key Artifacts

| Item | Value |
|------|-------|
| Tag | `v0.1.199` |
| Main commit | `ae2dc03` — `hot lane-document reload on peer endpoint change` |
| Release archive sha256 (tarball) | `77ccb9649c6d7b285180a2096701ea93f8212c7591b98ff9dd903eff042c31e3` |
| Installed `chimera-peer-egress` sha256 (both nodes) | `104332fb45ce0eeee4a9300d6dcbda6f418cd2ebea03c0ca8c472a4680ca8edd` |

## Verification Evidence

Local checks:
```text
cargo fmt --check                              # pass
cargo test --workspace                         # pass
cargo clippy --workspace --release             # pass
# Note: --all-targets clippy still fails on pre-existing chimera-cli test
#       unwraps/expect; product binaries are clean.
```

Stand services (both nodes):
```text
$ $HOME/.local/bin/chimera.sh -version
chimera-runtime 0.1.199
$ $HOME/.local/bin/chimera.sh -status | grep site_auto_watch
site_auto_watch_status=running mode=systemd_user service_state=active interval_sec=60 hot_interval_sec=1
```

NL → RU (baseline):
```text
$ setpriv --reuid 65534 --regid 65534 --clear-groups curl -sS --max-time 15 https://ipinfo.io
{ "ip": "138.16.175.96", ... }
```

RU → NL (baseline):
```text
$ setpriv --reuid 65534 --regid 65534 --clear-groups curl -sS --max-time 15 https://ipinfo.io
{ "ip": "91.124.19.180", ... }
```

Convergence after restart (stopwatch polling from the peer node every 2s):
```text
# systemctl --user restart chimera-node.service on NL; poll from RU
NL restart_issued
t=0s ip=
t=2s ip=
t=4s ip=
t=6s ip=91.124.19.180
converged

# systemctl --user restart chimera-node.service on RU; poll from NL
RU restart_issued
t=0s ip=
t=3s ip=
t=5s ip=
t=7s ip=
t=9s ip=
t=11s ip=138.16.175.96
converged
```

## Operational Notes

- Deployed via local-source path because GitHub latest still advertises
  `v0.1.196`; this is recorded as stand alignment, not GitHub first-install
  proof.
- RU discovery still advertises node_id `v3177669` (hostname default). Auth is
  token-based and unaffected; the label mismatch is carried forward as a known
  stand-only issue.

## Risks / Known Limitations

1. `chimera-site-watch.service` is bound to `chimera-node.service`; when the
   node unit restarts, site-watch is also recreated, so its hot-watch trigger
   fires on the *new* process’s initial cycle rather than a true in-process hot
   event. The measured convergence time now depends mostly on peer-egress
   startup + discovery poll interval, and is well under 15 seconds.
2. Operator node_id label drift remains open (RU advertises hostname id).
3. GitHub latest page lags behind the local stand build.

## Recommended Next Step

- Decide whether to also make `chimera-site-watch.service` resilient to a
  transient `chimera-node.service` restart without being torn down, OR keep the
  current restart-bounded behavior and document it.
- Address RU node_id mismatch if naming consistency becomes important.

## Safety

- No PC network/VPN/DNS/routes/firewall changed.
- Happ on current PC untouched.
- All practical checks executed via SSH only on authorized stand nodes.
