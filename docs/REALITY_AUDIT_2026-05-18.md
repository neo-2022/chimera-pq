# CHIMERA Reality Audit (2026-05-18)

## What actually works (fact-checked)

1. Local code quality gates pass:
- `just check`
- `just test`
- `just lint`
- `just deny`
- `just fuzz-smoke`
- `just net-sim`
- `just perf-smoke`

2. Laptop practical proxy path works in current setup:
- `chimera-gateway.service` / `chimera-client.service` are active.
- `chimera-socks-tunnel.service` is active.
- SOCKS listener `127.0.0.1:11080` is present.
- `curl --proxy socks5h://127.0.0.1:11080 https://www.youtube.com` returns `HTTP/2 200`.
- direct route sanity check (`https://example.org`) returns `HTTP/2 200`.

3. VPS service binaries run in current doctor/state style:
- user services are active.
- default verification artifacts still mostly report `network_state":"not_modified"`.

## Critical reality gaps vs strict MVP interpretation

1. CLI `up/down` now has explicit apply/rollback runtime paths, but not full automatic policy-driven OS datapath yet.
- `up` validates runtime profile from config and writes capture/carrier facts
  into the state file (mode/reason/profile/address).
- by default, `up/down` keep network state unchanged (`not_modified` path).
- `up` now supports explicit OS-level TUN apply path:
  `--apply-tun true --tun-name <name> [--tun-local-cidr <cidr> --tun-peer-cidr <cidr>]`.
- `up` supports explicit route apply path:
  `--apply-route true --route-cidr <cidr>`.
- `up` supports explicit DNS apply path:
  `--apply-dns true --dns-server <ip> --resolv-conf <path>`.
- `down`/`rollback clean`/`rollback recover` perform state-driven rollback
  (DNS/route/TUN best-effort) before state cleanup.
- in unprivileged shell this path fails honestly with permission error
  (`ioctl(TUNSETIFF): Operation not permitted`) and does not return fake success.
- TUN apply path now also includes interface address assignment attempt
  (`ip addr add <local> peer <peer> dev <tun>`), with rollback on any failure.
- route apply uses `ip route add/del ... dev <tun>` with rollback.
- DNS apply writes resolver file + backup/restore rollback.
- OS firewall/proxy automatic lifecycle is still not implemented.

2. Gateway run listener gap is fixed.
- `chimera-gateway run` now starts a real TCP listener for non in-memory carrier profiles.
- Evidence:
  `CHIMERA_GATEWAY_IDLE_EXIT_MS=300 cargo run -q -p chimera-gateway -- run --config /tmp/chimera_gateway_test.conf`
- Observed output includes:
  `Gateway listener started on 127.0.0.1:18443`.

3. M4/M5 closure still depends partly on simulation/report artifacts.
- default report stack is still mostly `network_state":"not_modified"`.
- explicit apply-path checks now cover real `network_state":"modified"` runtime flow,
  but full automatic policy-driven system datapath remains incomplete.

4. Current practical internet behavior on laptop is achieved by browser/proxy orchestration scripts and system proxy/PAC, not by native TUN datapath in core runtime.

## Verdict

- Lab/proof/report contour: PASS.
- Real OS-level datapath closure for strict M4/M5: PARTIAL / NOT CLOSED.
