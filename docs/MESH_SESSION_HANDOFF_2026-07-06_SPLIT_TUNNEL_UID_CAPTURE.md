# CHIMERA Mesh Session Handoff: Per-App / UID Split-Tunnel Capture

## Saved At

- Timestamp: 2026-07-06T00:10:00+03:00
- Branch: `failure-hardening-2026-07-05`
- Release: `v0.1.171`

## Active Objective

- Operate CHIMERA as a user-controlled **split-tunnel VPN**:
  - default traffic goes direct;
  - only selected traffic is forced through the mesh;
  - selection criteria: destination IPv4 CIDR and/or source process UID.
- Continue using the authorized SSH-only remote stand (no local PC network
  changes).

## What Changed In This Pass

### 1. Per-UID transparent capture

- Added `--capture-skuid <uid>` CLI flag and `CHIMERA_CAPTURE_SKUID` env var to
  `chimera-transparent-runtime` and `chimera-transparent-rules`.
- `crates/chimera-capture/src/redirect.rs`
  - `TransparentRedirectPlan` now carries `capture_skuids: Vec<u32>`.
  - Rendered nftables rules insert `meta skuid <uid>` match before the redirect.
  - Root UID `0` remains explicitly bypassed to prevent redirect loops.

### 2. Config separation for split-tunnel modes

- `/root/.config/chimera/transparent-runtime.env` on both stand hosts now
  supports:
  - `CAPTURE_DOMAIN` — list of destination domains/cidrs for the
    **domain-only capture** mode;
  - `CAPTURE_SKUID` — UID list for the **per-app capture** mode.
- `chimera-manual-start.sh` consumes the env file and starts
  `chimera-transparent-runtime` with the resolved `--capture-cidr-v4` and
  `--capture-skuid` arguments.

### 3. Stand runtime deployment

- Rebuilt `chimera-transparent-runtime` on the PC (`cargo build --release`).
- Synced new binaries to `/root/.local/share/chimera/bin/` on both VPS hosts
  (primary NL `91.124.19.180`, secondary RU `138.16.175.96`).
- Verified binary checksums on the remote stand:
  - `chimera-transparent-runtime`: `24ec94c560c2fce015b318e96186655527f9f25b4598062a9482d695bff12ab3`
  - `chimera-peer-egress`: `37cb9d929b7ead1980041d39833908eb84ed47e7d334850819bd79e20df305b6`

## Known Pre-Existing Issues Still Open

1. **TUN / full-tunnel still down.** `chimera0` reports `NO-CARRIER/linkdown`;
   UDP/ICMP/DNS bypass CHIMERA. Only TCP-redirect datapath works.
2. **systemd auto-start path is broken.** `chimera-control.sh start` still hangs
   on `chimera-site-watch.service`. The remote stand is operating via the
   manual `/root/chimera-manual-start.sh` helper.
3. **No dynamic failover.** Capture is a static whitelist/blacklist, not a
   "try direct first, send to CHIMERA on failure" decision.
4. **DNS/UDP bypass.** Only TCP traffic is redirected; DNS goes direct.
5. **Laptop `192.168.31.21` remains offline** pending physical reboot.

## Remote Stand Evidence (v0.1.171)

All checks below were performed only over SSH on the authorized remote stand; the
PC was used only as a controller, no local CHIMERA or network changes were made.

### Mesh control-plane (symmetric `node` mode)

- Both VPS hosts run `chimera-peer-egress --mode node`.
- Peer addresses: primary→secondary `:18142`, secondary→primary `:18142`.
- Egress tokens: `CHIMERA_PEER_EGRESS_TOKEN=mesh-shared-token`, AEAD `aes256gcm`,
  pool 8.
- Two-way egress verified:
  - secondary→primary reported public IP `91.124.19.180` (NL);
  - primary→secondary reported public IP `138.16.175.96` (RU).

### nftables rule diff after adding `--capture-skuid`

Previous rule (captured all TCP to the target CIDR):
```text
meta l4proto tcp ip daddr 34.117.59.81 tcp dport != 443 redirect to :18134
```

Current rules (domain + per-UID selectors):
```text
meta l4proto tcp meta skuid 0 return
meta l4proto tcp ip daddr 34.117.59.81 tcp dport != 443 redirect to :18134
meta l4proto tcp meta skuid 65534 tcp dport != 443 redirect to :18134
```

This confirms the broad TCP capture was replaced by targeted selectors, and root
UID is bypassed.

### End-to-end split-tunnel scenarios (secondary VPS RU)

| Runner UID | Target | Expected path | Observed egress IP | Verdict |
|---|---|---|---|---|
| root (0) | ipinfo.io | direct | `138.16.175.96` RU | pass |
| root (0) | ifconfig.me | direct | `138.16.175.96` RU | pass |
| nobody (65534) | ipinfo.io | via mesh | `91.124.19.180` NL | pass |
| nobody (65534) | ifconfig.me | via mesh | `91.124.19.180` NL | pass |
| chimera-test (10001) | ipinfo.io | via mesh | `91.124.19.180` NL | pass |
| chimera-test (10001) | ifconfig.me | direct | `138.16.175.96` RU | pass |

These three cases map exactly to the desired user-level semantics:

- **Root / privileged tools** stay direct.
- **Dedicated per-app UID** (`nobody`, e.g. a sandboxed browser profile) routes
  **all** its TCP through CHIMERA.
- **Regular user** (`chimera-test`, e.g. a normal browser) sends only the
  **blocked/selected destinations** through CHIMERA while everything else stays
  direct.

### Commands used to reproduce the scenarios

```bash
# root → direct
ssh root@138.16.175.96 curl -sS --max-time 15 https://ipinfo.io
ssh root@138.16.175.96 curl -sS --max-time 15 https://ifconfig.me

# nobody(65534) → full per-app tunnel
ssh root@138.16.175.96 setpriv --reuid 65534 --regid 65534 --clear-groups \
  curl -sS --max-time 15 https://ipinfo.io
ssh root@138.16.175.96 setpriv --reuid 65534 --regid 65534 --clear-groups \
  curl -sS --max-time 15 https://ifconfig.me

# chimera-test(10001) → domain-only tunnel
ssh root@138.16.175.96 setpriv --reuid 10001 --regid 10001 --clear-groups \
  curl -sS --max-time 15 https://ipinfo.io
ssh root@138.16.175.96 setpriv --reuid 10001 --regid 10001 --clear-groups \
  curl -sS --max-time 15 https://ifconfig.me
```

## Limitations / Next Steps

- Capture remains IPv4/TCP-only.
- UDP and DNS are not redirected; they are direct unless another mechanism is
  added.
- Manual start only; systemd auto-start is still broken.
- Per-UID selection uses a static UID list, not dynamic `cgroup`/`net_cls`
  discovery.
- The laptop node is not yet online, so three-host mesh flow could not be
  verified.

