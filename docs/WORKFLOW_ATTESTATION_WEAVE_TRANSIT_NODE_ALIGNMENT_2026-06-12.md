# Workflow Attestation: WEAVE Transit Node Alignment

Saved at: 2026-06-12 20:39:33 MSK

## Scope

Continue aligning CHIMERA-PQ with the documented WEAVE node model:

- one symmetric node, not a product-level client/server/gateway split;
- local ingress, peer ingress, local egress and peer transit capability;
- transit payload remains sealed/opaque and is not inspected or logged;
- laptop/VPS real-world verification must use GitHub Release/Latest one-command
  install/update only.

## Decision Council

- Architect: keep `gateway` only as transitional binary/config compatibility,
  not as the product route role.
- Senior developer: update active policy/CLI/config/proof surfaces to canonical
  `transit`, keep legacy policy token parsing for old files.
- Tester: require source-level parser/CLI/policy/crypto/session/mesh checks and
  do not convert them into real-world PASS.
- Security engineer: keep sealed transit forwarding as opaque bytes; do not
  change traffic key labels in this slice because that would alter wire
  compatibility.
- DevOps: no local CHIMERA install/start/stop; laptop/VPS proof requires a new
  GitHub Release and `releases/latest`.
- Critic: previous attestation was too strong: `docs/route_explain_latest.json`
  still contained `default-gateway` before this correction.

## Corrections Made

- Corrected active runtime policy from `*-gateway => gateway` to
  `*-transit => transit`:
  - `configs/policy.runtime.conf`
- Added canonical manual transit domain file and kept old gateway file as a
  legacy alias:
  - `configs/manual_transit_domains.txt`
  - `configs/manual_gateway_domains.txt`
- Updated probe route recommendation so failed direct access recommends
  `transit` and can write `=> transit` policy, without proxy fallback:
  - `crates/chimera-cli/src/main.rs`
  - `docs/PROBE_ACCESS.md`
- Updated site-auto/status shell surfaces to use
  `MANUAL_TRANSIT_DOMAINS_FILE` while still reading the old gateway file as
  legacy input:
  - `scripts/chimera-control.sh`
- Replaced product-facing default carrier placeholder from
  `gateway.example.org` to `node.example.org`, and test status placeholder from
  `gateway.local` to `node.local`:
  - `crates/chimera-cli/src/main.rs`
  - `crates/chimera-config/src/lib.rs`
  - `crates/chimera-carrier-tls/src/lib.rs`
  - `crates/chimera-carrier-quic/src/lib.rs`
  - `configs/client.example.conf`
  - `configs/upstream_proxy.env.example`
- Updated advertised node capabilities from `gateway` to `transit`:
  - `crates/chimera-cli/src/mesh_cli/nodes_cmd/advertise.rs`
- Added neutral traffic-secret direction accessors and moved touched session /
  peer-egress code to `initiator_to_responder` /
  `responder_to_initiator` accessors without changing HKDF labels:
  - `crates/chimera-crypto/src/lib.rs`
  - `crates/chimera-session/src/lib.rs`
  - `crates/chimera-carrier/src/peer_egress/handshake.rs`
- Refreshed safe, non-mutating proof artifacts:
  - `docs/route_explain_latest.json` now uses `default-transit`.
  - `docs/datapath_latest.json` uses `transit_explain`.
  - `docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json` uses `transit_ok`.
  - `docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json` remains transit/direct/block.

## Evidence

Source-level checks passed:

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --quiet`
- `cargo test -p chimera-policy --quiet`
- `cargo test -p chimera-config --quiet`
- `cargo test -p chimera-crypto --quiet`
- `cargo test -p chimera-cli probe --quiet`
- `cargo test -p chimera-cli route_render_snapshot --quiet`
- `cargo test -p chimera-session sealed_transit --quiet`
- `cargo test -p chimera-mesh weave_contract --quiet`
- `bash scripts/anti_monolith_guard.sh`
- `bash scripts/ship_structure_guard.sh`
- `cargo run -q -p chimera-lab --bin rust_no_hardcode_guard`
- `git diff --check`
- `bash -n scripts/chimera-control.sh scripts/chimera-autofix.sh scripts/install_desktop_control.sh scripts/ship_readiness.sh scripts/runtime_datapath_multiflow_smoke.sh scripts/runtime_policy_precedence_smoke.sh`

Grep audit over active source/proof surfaces found only legacy policy parser
compatibility for `=> gateway`:

- `crates/chimera-policy/src/lib.rs` accepts `gateway` as old input and maps it
  to `OutboundMode::Transit`.

## Not Done / Limits

- This is not a real-world laptop/VPS PASS.
- No CHIMERA install/start/stop was run on the local PC.
- No local DNS/routes/firewall/proxy/VPN/Happ/MYVPN/router settings were
  changed.
- Laptop/VPS verification is still required after publishing a GitHub Release
  and confirming `releases/latest` points to it.
- Stand verification must be GitHub one-command install/update only; local
  tarballs, `scp`, `rsync`, `git clone`, `cargo build`, and `cargo run` on the
  target do not count.
