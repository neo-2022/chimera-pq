# CHIMERA Session Handoff - Transparent Runtime

## Saved At
- Timestamp: 2026-05-26 local session

## Active Objective
- Complete invisible transparent datapath/failover for CHIMERA without app proxy flags, with speed checks and safe VPS cleanup.

## Implemented
- Added Rust `chimera-transparent-runtime` supervisor:
  - applies nft REDIRECT from Rust;
  - starts `chimera-transparent-tcp`;
  - supports `--transparent-uid/--transparent-gid` so loop-exempt UID is not root;
  - deletes the CHIMERA nft table on normal/signal shutdown.
- Extended `chimera-transparent-tcp` and peer-egress logging for strict evidence.
- Reworked nft redirect renderer to generate final redirect rules directly, without string replacement.
- Peer-egress is hybrid encrypted: X25519 + ML-KEM-768, ChaCha20-Poly1305 frames.

## Strict Remote Evidence
- VPS direct baseline to laptop test resource: `vps_direct_without_chimera_fail`.
- With transparent runtime:
  - `transparent_flow_accepted destination=192.168.31.31:18203`
  - `transparent_direct_failed ... connection timed out`
  - `transparent_route_selected route=gateway`
  - `peer_connect_request_sent request=CONNECT 192.168.31.31 18203`
  - `laptop_target_connected target=192.168.31.31:18203`
- Ordinary TCP app command without proxy flags passed:
  - 64 MiB before chunk change: `0.93 MiB/s`.
  - 8 MiB short-flow: `4.06 MiB/s`.
  - 64 MiB with Rust app probe and `--request-line`: `9.20 MiB/s`.

## Validation
- `cargo test -p chimera-capture -p chimera-carrier -p chimera-crypto -p chimera-session`: PASS.
- `cargo run -q -p chimera-lab --bin rust_no_hardcode_guard`: PASS.
- VPS nft cleanup verified after tests: `nft_cleanup_ok`.

## Open Items
- Long-flow Rust app probe improved to `transparent_flow_probe_64m=pass bytes=67108864 elapsed_ms=6953 throughput_mib_s=9.20`; browser/IDE proof still not closed.
- Browser/VSCode/Codium real UI validation under the new Rust transparent runtime is not done.
- Runtime packaging/system service integration is not done.
- Need avoid `pkill -f` in future VPS work; use exact PIDs/ports.

## Next Step
- Close browser/IDE validation. VPS currently lacks Chromium/Firefox; either install a browser on VPS or add a reverse/app-side topology that lets laptop browser be the app-side node safely.
