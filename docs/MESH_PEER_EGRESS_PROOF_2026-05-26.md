# CHIMERA Peer Egress Proof - 2026-05-26

## Scope

Rust peer-egress datapath prototype for CHIMERA mesh egress, without app proxy settings and without changing OS routes/firewall/TUN on the test hosts.

## Implemented

- Hybrid encrypted peer channel: X25519 + ML-KEM-768 derived traffic secrets.
- AEAD stream frames: ChaCha20-Poly1305 with packet-number AAD.
- Runtime-only endpoints and ports; no product hardcode for hosts, ports, devices, or resources.
- Native local protocol `CHIMERA-LOCAL/1` and SOCKS5-compatible ingress for diagnostics.
- Speed gate: `--min-throughput-mib-s` fails the command when throughput is below the requested floor.
- Multi-lane probe: `--connections N` for aggregate peer-channel throughput checks.
- Download-oriented probe: `download-echo` + `download-probe` measures the dominant response direction from egress peer back to requester.

## Local Release Evidence

Command:

```bash
cargo run --release -q -p chimera-carrier --bin chimera-peer-egress -- \
  --mode bench \
  --token bench-token-for-secure-peer-egress \
  --pool 16 \
  --bench-bytes 134217728 \
  --min-throughput-mib-s 500
```

Result:

```text
chimera_peer_egress_bench=pass bytes=134217728 elapsed_ms=115 throughput_mib_s=1105.53
```

## Remote Tandem Evidence

Topology:

- VPS temporary ingress/peer listener.
- Laptop temporary peer worker and temporary download echo target.
- No route/firewall/DNS/TUN changes.
- No app-specific proxy settings.

Passing download-direction command on VPS:

```bash
timeout 60 /tmp/chimera-peer-egress-test5 --mode download-probe \
  --server 127.0.0.1:18112 \
  --target 127.0.0.1:18113 \
  --token '<redacted>' \
  --bench-bytes 67108864 \
  --connections 8 \
  --min-throughput-mib-s 5 \
  --connect-timeout-ms 3000
```

Result:

```text
chimera_peer_egress_download_probe=pass bytes=67108864 connections=8 elapsed_ms=12196 throughput_mib_s=5.25
```

Failed higher gate:

```text
error: throughput below gate: actual_mib_s=5.26 min_mib_s=8
```

## Test Evidence

Commands:

```bash
cargo fmt --check --package chimera-crypto --package chimera-session --package chimera-carrier
cargo test -p chimera-crypto
cargo test -p chimera-session
cargo test -p chimera-carrier
cargo run -q -p chimera-lab --bin rust_no_hardcode_guard
```

Result:

```text
chimera-crypto: 11 passed
chimera-session: 15 passed
chimera-carrier lib: 4 passed
chimera-peer-egress bin: 12 passed
rust/no-hardcode guard: PASS
```

## Current Status

Partially implemented.

The encrypted mesh peer channel works and is verified on the laptop/VPS tandem for the response/download direction. The current remote path does not meet a higher 8 MiB/s gate; measured aggregate is about 5.25 MiB/s. Transparent OS/application capture is not yet proven by this artifact.
