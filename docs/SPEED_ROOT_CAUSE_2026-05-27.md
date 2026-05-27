# CHIMERA Speed Root Cause - 2026-05-27

## Scope

Question: where does remote VPS/laptop throughput get cut, and what was changed without weakening encryption?

## Facts

Local release crypto/peer datapath remains above the 1000 MiB/s target:

```text
aes256gcm, 1 MiB secure frame: chimera_peer_egress_bench=pass bytes=268435456 elapsed_ms=203 throughput_mib_s=1257.45
aes256gcm, TCP buffer tuned + 256 KiB test frame: chimera_peer_egress_bench=pass bytes=268435456 elapsed_ms=190 throughput_mib_s=1346.22
chacha20poly1305, TCP buffer tuned + 256 KiB test frame: chimera_peer_egress_bench=pass bytes=268435456 elapsed_ms=204 throughput_mib_s=1250.20
```

The 256 KiB secure-frame experiment was reverted because remote aggregate throughput dropped:

```text
256 KiB frame remote aggregate: chimera_peer_egress_download_probe=pass bytes=67108864 connections=8 elapsed_ms=15960 throughput_mib_s=4.01
```

The retained change is TCP socket buffer tuning:

```text
before TCP buffer tuning: chimera_peer_egress_download_probe=pass bytes=67108864 connections=8 elapsed_ms=16772 throughput_mib_s=3.82
after TCP buffer tuning:  chimera_peer_egress_download_probe=pass bytes=67108864 connections=8 elapsed_ms=7403 throughput_mib_s=8.64
```

Single-flow check after buffer tuning showed the remaining bottleneck is one WAN TCP/peer flow:

```text
single peer flow: error: throughput below gate: actual_mib_s=0.61 min_mib_s=1
8 peer lanes:      chimera_peer_egress_download_probe=pass bytes=67108864 connections=8 elapsed_ms=7403 throughput_mib_s=8.64
```

Available TCP congestion control on both VPS and laptop:

```text
reno cubic
active: cubic
```

BBR is not available on these hosts, so CHIMERA cannot enable BBR per socket without changing host kernel/module/sysctl state.

## Root Cause

The current remote speed loss is not caused by AEAD or ML-KEM. Evidence: local encrypted datapath is above 1000 MiB/s.

The measured remote loss is in the transport layer of a single WAN TCP/peer flow between VPS and laptop. Parallel lanes raise aggregate throughput, which means CPU/crypto has spare capacity and the path is congestion/window/backpressure limited.

## Implemented Fix

Product code now applies best-effort 4 MiB TCP send/receive buffers on carrier, transparent relay, and flow probe sockets:

- `crates/chimera-carrier/src/bin/chimera-peer-egress.rs`
- `crates/chimera-capture/src/bin/chimera-transparent-tcp.rs`
- `crates/chimera-capture/src/bin/chimera-flow-probe.rs`

This does not change encryption, keys, AEAD, ML-KEM, routing, DNS, firewall policy, or MyVPN.

## Remaining Work

To remove the remaining single-flow limit without weakening security, CHIMERA needs a native multi-subflow transport for one logical app flow:

```text
single app TCP flow
 -> CHIMERA ordered encrypted frame stream
 -> multiple carrier subflows
 -> ordered reassembly
 -> target TCP flow
```

That is a product transport feature, not a crypto change. It must preserve ordered delivery, backpressure, replay protection, and AEAD sequence binding.
