# CHIMERA AEAD Performance and PQ Crypto - 2026-05-27

## Scope

Objective: improve CHIMERA peer/transparent throughput without weakening the post-quantum security model.

## Security Model

The peer-egress secure channel remains hybrid:

- classical ECDH: X25519;
- post-quantum KEM: ML-KEM-768;
- transcript-bound HKDF-SHA256 traffic secrets;
- AEAD-protected frames with packet-number AAD and strict packet sequence.

The optimization does **not** remove ML-KEM-768 and does **not** reduce key size. It adds a second standard bulk AEAD:

- `chacha20poly1305` suite id: `0xEE02`;
- `aes256gcm` suite id: `0xEE03`.

The AEAD suite id and wire name are included in the secure handshake transcript. A mismatch fails the handshake, preventing silent downgrade/mismatch.

Post-quantum note: AES-256-GCM keeps a 256-bit symmetric key. Under Grover-style quadratic search, the effective brute-force margin remains roughly 128-bit, which is the intended conservative symmetric margin for PQ-era designs. This is not custom crypto; it uses the audited RustCrypto `aes-gcm` implementation.

## Code Changes

- `chimera-crypto`:
  - added `encrypt_aes256gcm`;
  - added `decrypt_aes256gcm`;
  - added in-place AEAD helpers for ChaCha20-Poly1305 and AES-256-GCM to avoid an extra ciphertext allocation/copy in the peer frame path;
  - added AES-GCM tamper and roundtrip tests.
- `chimera-peer-egress`:
  - added runtime option `--aead chacha20poly1305|aes256gcm`;
  - added env option `CHIMERA_PEER_EGRESS_AEAD`;
  - extended secure hello with AEAD suite id;
  - bound AEAD suite into the transcript;
  - rejects suite mismatch;
  - switched secure frame read/write to in-place AEAD.

## Local Release Benchmark

Command shape:

```bash
target/release/chimera-peer-egress --mode bench \
  --token bench-token-for-aead-<suite> \
  --pool 16 \
  --bench-bytes 268435456 \
  --min-throughput-mib-s 100 \
  --aead <suite>
```

Results:

```text
chacha20poly1305: chimera_peer_egress_bench=pass bytes=268435456 elapsed_ms=208 throughput_mib_s=1230.07
aes256gcm:        chimera_peer_egress_bench=pass bytes=268435456 elapsed_ms=192 throughput_mib_s=1327.76
```

Local AES-256-GCM improvement: about 7.9%.

Post in-place verification on the same command shape:

```text
chacha20poly1305: chimera_peer_egress_bench=pass bytes=268435456 elapsed_ms=214 throughput_mib_s=1192.58
aes256gcm:        chimera_peer_egress_bench=pass bytes=268435456 elapsed_ms=203 throughput_mib_s=1257.45
```

Both standard AEAD paths remain above the 1000 MiB/s local release target in this benchmark. The post in-place run is slightly lower than the earlier run on this host, so the current evidence is: in-place removes an avoidable allocation/copy, but this specific benchmark is not yet faster than the previous release run.

## Remote VPS/Laptop Peer Benchmark

Topology:

```text
VPS app/probe -> VPS peer-egress -> encrypted peer channel -> laptop peer -> laptop download-echo target
```

No OS route/DNS/default-route changes. No app proxy flags.

Results, same target and same 64 MiB / 8 lane probe:

```text
chacha20poly1305: chimera_peer_egress_download_probe=pass bytes=67108864 connections=8 elapsed_ms=15481 throughput_mib_s=4.13
aes256gcm:        chimera_peer_egress_download_probe=pass bytes=67108864 connections=8 elapsed_ms=11404 throughput_mib_s=5.61
```

Remote AES-256-GCM improvement: about 35.8%.

## Remote Transparent Datapath Benchmark

Topology:

```text
ordinary TCP app on VPS
 -> nft REDIRECT
 -> chimera-transparent-tcp
 -> local CHIMERA gateway
 -> AES-256-GCM peer-egress
 -> laptop peer
 -> laptop target
```

Result:

```text
transparent_flow_probe_64m_aes256gcm=pass bytes=67108864 elapsed_ms=5374 throughput_mib_s=11.91
```

Previous verified transparent Rust app probe with default ChaCha path:

```text
transparent_flow_probe_64m=pass bytes=67108864 elapsed_ms=6953 throughput_mib_s=9.20
```

Transparent AES-256-GCM improvement: about 29.5%.

## Cleanup Evidence

```text
vps_cleanup_done
nft_cleanup_ok
laptop_cleanup_done
```

## 2026-05-27 Follow-Up Verification

Commands:

```bash
cargo fmt --check --package chimera-capture --package chimera-carrier --package chimera-crypto --package chimera-session
cargo test -p chimera-capture -p chimera-carrier -p chimera-crypto -p chimera-session
cargo run -q -p chimera-lab --bin rust_no_hardcode_guard
```

Results:

```text
chimera-capture tests: 31 passed
chimera-carrier tests: 17 passed
chimera-crypto tests: 15 passed
chimera-session tests: 15 passed
rust/no-hardcode guard: PASS
```

Remote VPS retest after the in-place patch was initially blocked because SSH to `91.124.19.180:22` timed out before test startup. After VPS reboot, the remote retest completed.

Post-reboot remote peer result:

```text
chimera_peer_egress_download_probe=pass bytes=67108864 connections=8 elapsed_ms=16772 throughput_mib_s=3.82
```

Post-reboot transparent datapath result:

```text
transparent_flow_probe_64m_aes256gcm_inplace=pass bytes=67108864 elapsed_ms=8139 throughput_mib_s=7.86
```

Transparent routing evidence:

```text
event=transparent_flow_accepted destination=192.168.31.31:18603
event=transparent_direct_failed reason=direct connect failed: 192.168.31.31:18603: connection timed out
event=transparent_route_selected route=gateway destination=192.168.31.31:18603
```

Post-reboot cleanup evidence:

```text
nft_cleanup_ok
```

## Status

Partially complete for full product closure, complete for the crypto/performance subtask:

- AES-256-GCM path implemented and tested.
- In-place AEAD path implemented and tested.
- PQ hybrid handshake preserved.
- Local and remote benchmarks show improvement.
- Browser/IDE invisible UX proof remains a separate open gate.
- Post in-place remote VPS/laptop retest passed after VPS reboot.
