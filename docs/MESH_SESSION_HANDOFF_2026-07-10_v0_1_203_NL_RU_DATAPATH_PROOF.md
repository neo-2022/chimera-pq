# MESH_SESSION_HANDOFF_2026-07-10_v0_1_203_NL_RU_DATAPATH_PROOF

**session_id:** handoff-2026-07-10-203-nl-ru-datapath-proof
**version:** 0.1.203
**status:** partial

## Stand State

- Both authorized stand nodes updated to GitHub release `v0.1.203`.
- Node identities: `amai` (NL), `vdsina` (RU).
- `chimera.sh -version` reports `chimera-runtime 0.1.203` on both nodes.
- `chimera-runtime.service`, `chimera-node.service`, `chimera-datapath.service`, `chimera-site-watch.service` are active.
- `runtime_boot_enabled_state=enabled` confirmed on both nodes (RU was previously disabled and has been fixed with `systemctl --user enable chimera-runtime.service`).
- PC remains control-only; no CHIMERA runtime/network changes on the PC.

## site-watch Status

- `site_auto_watch_status=running mode=systemd_user service_state=active` on both nodes.
- No `site_auto_watch_loop=fail` or `consecutive_failures` events in the last 6 hours on either node.
- RU and NL journal entries for `chimera-site-watch.service` show normal start/stop cycles from earlier install/restart work; no sustained failure loop.

## Bidirectional Datapath Proof

Mesh capture policy on both nodes redirects TCP from UID `65534` (`nobody`) through the local transparent TCP proxy (`<redacted-ip>`) into the WEAVE peer-egress tunnel. Root traffic is exempt and exits directly from the local node.

Test method (run as `root` and as `nobody` via `runuser -u nobody -- curl -sS <target>`):

### NL → RU (10 runs per target)

| target | pass | fail | notes |
|--------|------|------|-------|
| http://ipinfo.io/ip | 9 | 1 | first request returned empty reply; remainder returned RU IP |
| http://ifconfig.me | 10 | 0 | all returned RU IP |
| http://icanhazip.com | 10 | 0 | all returned RU IP |

### RU → NL (10 runs per target)

| target | pass | fail | notes |
|--------|------|------|-------|
| http://ipinfo.io/ip | 8 | 2 | first two requests returned empty reply; remainder returned NL IP |
| http://ifconfig.me | 10 | 0 | all returned NL IP |
| http://icanhazip.com | 5 | 5 | intermittent empty replies, mixed with successful NL IP returns |

### Stability Spot Check on `ifconfig.me` (20 runs per direction)

- **NL → RU:** 20/20 passed, every request returned RU IP.
- **RU → NL:** 20/20 passed, every request returned NL IP.

### Conclusion from Proof

- Bidirectional mesh datapath is functional and stable for the `ifconfig.me` target.
- `ipinfo.io` on both sides and `icanhazip.com` on the RU → NL direction show intermittent `curl: (52) Empty reply from server`, most often on the first few sequential requests. These correlate with `event=transparent_flow_error reason=read line failed` in the datapath log and likely reflect occasional peer-pool hand-off latency or transient carrier hiccups, not a fundamental routing failure.
- Root direct traffic consistently exits via the local node IP, confirming split-tunnel bypass works.

## Observed Residuals / Risks

- **Target-specific flakiness:** `ipinfo.io` and `icanhazip.com` are not as stable as `ifconfig.me` over the mesh. This appears to be a reliability/resilience issue rather than a configuration problem.
- **Public peer ingress noise:** Both nodes log frequent `event=weave_peer_ingress_auth_failed reason_class=runtime_error`, which is consistent with unauthorized scanning of the public peer listen ports. Authorized mesh peers still authenticate and carry traffic.
- **Datapath cold-start behavior:** First request(s) after a quiet period are more likely to fail. A keepalive or retry-on-no-peer mechanism may be needed for production-grade consistency.

## Attestation

- `CURRENT_WORKLINE_ATTESTATION.json` updated to point to this handoff.
- Status kept as `partial` until the residual empty-reply flakiness is root-caused and the datapath is proven stable across all common probe targets.
