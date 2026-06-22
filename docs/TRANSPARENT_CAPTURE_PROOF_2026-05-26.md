# CHIMERA Transparent Capture Proof - 2026-05-26

## Scope

Transparent application traffic capture without browser/IDE/app proxy settings.

Implemented Rust components:

- `chimera-transparent-tcp`: TCP transparent datapath acceptor.
- `chimera-transparent-rules`: nft REDIRECT rule generator/apply/delete helper.
- `chimera-transparent-runtime`: Rust runtime supervisor that applies nft REDIRECT, starts transparent TCP, and deletes the CHIMERA nft table on normal/signal shutdown.
- `chimera-local-gateway-echo`: CHIMERA-LOCAL test gateway for isolated proof.

## Mechanism

Target model:

```text
ordinary app TCP connect
 -> nft REDIRECT output hook
 -> chimera-transparent-tcp
 -> original destination lookup via SO_ORIGINAL_DST
 -> direct attempt
 -> gateway fallback via CHIMERA-LOCAL when direct fails
 -> encrypted peer-egress channel
 -> remote peer opens destination
```

Loop prevention:

- nft rule excludes the CHIMERA runtime UID with `meta skuid <uid> return`.
- Strict tests used UID/GID `65534` for the transparent runtime child.
- Capture was restricted to one proof CIDR and one proof TCP port.

## SIDE_A/Side B Strict Auto-Failover Proof

Safety constraints:

- No SSH/default-route/DNS broad capture.
- No browser/IDE/application proxy flags.
- nft rule matched only `<stand-host-a-ip>/32` and one proof TCP port.
- nft table was deleted after the test.

Direct baseline from SIDE_A before CHIMERA runtime:

```text
side_a_direct_without_chimera_fail
```

Runtime rule:

```text
table inet chimera_runtime_held {
  chain output {
    type nat hook output priority dstnat; policy accept;
    meta skuid 65534 return
    oifname "lo" return
    ip daddr <stand-host-a-ip> tcp dport 18203 redirect to :18204
  }
}
```

Strict path logs:

```text
event=transparent_flow_accepted destination=<stand-host-a-ip>:18203
event=transparent_direct_failed reason=direct connect failed: <stand-host-a-ip>:18203: connection timed out
event=transparent_route_selected route=gateway destination=<stand-host-a-ip>:18203
event=local_ingress_destination host=<stand-host-a-ip> port=18203 native_client=true
event=peer_connect_request_sent request=CONNECT <stand-host-a-ip> 18203
event=peer_connect_ack_received
event=side_b_peer_request_received request=CONNECT <stand-host-a-ip> 18203
event=side_b_target_connected target=<stand-host-a-ip>:18203
event=side_b_peer_connect_ack_sent target=<stand-host-a-ip>:18203
```

Ordinary app/TCP command result from SIDE_A, without proxy settings:

```text
nft_runtime_auto_failover=pass bytes=67108864 elapsed_ms=68458 throughput_mib_s=0.93
```

Optimized app-level Rust probe with `--request-line` (ordinary TCP app, no proxy):

```text
transparent_flow_probe_64m=pass bytes=67108864 elapsed_ms=6953 throughput_mib_s=9.20
```

AES-256-GCM peer suite repeat on full transparent path:

```text
transparent_flow_probe_64m_aes256gcm=pass bytes=67108864 elapsed_ms=5374 throughput_mib_s=11.91
```

Invalidated earlier shell-escaped probe (kept as negative evidence):

```text
nft_runtime_auto_failover_optimized=pass bytes=67108864 elapsed_ms=78765 throughput_mib_s=0.81
reason=invalid_probe_request_literal_backslash_n
```

Short-flow repeat after strict proof:

```text
nft_runtime_auto_failover_small=pass bytes=8388608 elapsed_ms=1972 throughput_mib_s=4.06
```

Cleanup verification:

```text
ssh_ok_after_cleanup_attempt
nft_cleanup_ok
side_a_exact_cleanup_done
side_b_exact_cleanup_done
```

## Earlier Isolated REDIRECT Proof

Local echo gateway proof on SIDE_A:

```text
chimera_local_gateway_echo=ready listen=127.0.0.1:18135
chimera_transparent_tcp=ready listen=0.0.0.0:18134
nft_redirect_transparent=pass reply=gateway:hello
ssh_after_redirect_ok
nft_cleanup_ok
```

## Rust Tests

Commands:

```bash
cargo test -p chimera-capture -p chimera-carrier
cargo run -q -p chimera-lab --bin rust_no_hardcode_guard
```

Latest result:

```text
chimera-capture lib: 15 passed
chimera-local-gateway-echo: 1 passed
chimera-transparent-rules: 2 passed
chimera-transparent-runtime: 2 passed
chimera-transparent-tcp: 6 passed
chimera-carrier lib: 4 passed
chimera-peer-egress bin: 12 passed
```

## Current Status

Partially implemented.

Confirmed:

- Transparent nft REDIRECT captures ordinary app TCP without app proxy flags.
- SO_ORIGINAL_DST destination recovery works.
- Direct failure triggers gateway route selection.
- Gateway route uses encrypted peer-egress to the side_b peer.
- Cleanup was verified after tests.

Not closed:

- Long-flow app-level Rust probe reached `9.20 MiB/s` with ChaCha20-Poly1305 and `11.91 MiB/s` with AES-256-GCM for 64 MiB through transparent failover. Browser/IDE validation is still not closed because SIDE_A has no Chromium/Firefox installed.
- Browser/VSCode/Codium real UI validation has not been run under this new Rust runtime; SIDE_A has no Chromium/Firefox available, side_b browser exists but current peer topology is SIDE_A-app-side.
- Runtime packaging/system service integration is not yet finished.
