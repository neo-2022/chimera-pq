# Workflow Attestation: Remote Release/Update and Local Transit Policy Proof v0.1.119

Status: partial_real_world_pass
Date: 2026-06-19

## Objective

Publish `v0.1.119` as GitHub `Latest`, update laptop/VPS only through the
GitHub one-command install path, and re-run the SSH stand proof for the real
local sealed-transit policy defect found on `v0.1.118`.

Defect under proof:

- local sealed transit on the node must fail closed when
  `allow_pool_transit=false`, even if the peer pool already has a live next hop;
- normal local ingress -> peer ingress -> local egress must keep working;
- when `allow_pool_transit=true`, the local sealed-transit branch must still
  forward to the live peer pool;
- logs must stay redacted.

## Council Notes

Real sub-agent roles were used for architecture, development, testing,
security, DevOps/release, and critic review.

Agreed:

- the source fix in `1a25689` is the correct minimal repair for the policy
  bypass;
- corrected PASS was forbidden until a real SSH stand re-proof on laptop and
  VPS;
- the proof must include both a deny-path and a positive control.

Rejected:

- claiming corrected PASS from source-only tests;
- replacing SSH stand proof with local PC runtime;
- calling the whole mesh datapath closed from the local sealed-transit slice.

## Source and Release Evidence

Source fix:

```text
commit: 1a25689e92e6663c9135bef4bf129667bf9a85fc
tag: v0.1.119
message: Block local WEAVE transit pool fallback by policy
```

Key source lines:

- `crates/chimera-carrier/src/peer_egress/node.rs:162-171`
- `crates/chimera-carrier/src/peer_egress/transit.rs:126-135`
- `crates/chimera-carrier/src/peer_egress/modes_tests/local_egress.rs:180-217`

GitHub release/latest proof:

```text
latest redirect: https://github.com/neo-2022/chimera-pq/releases/download/v0.1.119/chimera.sh
release workflow run: 27844226026
release workflow conclusion: success
published checksum: 6b3c0f7dd8d8d61d23dc45926fa35e26ea66bf78bb6e6b5116c6cd4b06b9c8d0
required assets:
  chimera.sh
  chimera-pq-release.tar.gz
  chimera-pq-release.tar.gz.sha256
```

## Source Gates

Passed before release:

```text
cargo fmt --all -- --check
cargo test -q -p chimera-carrier local_sealed_transit_fails_closed_when_pool_transit_denied
cargo test -q -p chimera-carrier
cargo test -q --workspace
cargo check -q --workspace
cargo clippy -q -p chimera-carrier --all-targets -- -D warnings
cargo clippy -q --workspace --all-targets -- -D warnings
bash scripts/anti_monolith_guard.sh
just rust-no-hardcode-guard
bash scripts/chimera_installer_gate.sh
bash scripts/chimera_update_contract_smoke.sh
bash scripts/chimera_start_contract_smoke.sh
bash scripts/chimera_stop_contract_smoke.sh
CHIMERA_RELEASE_VERSION=0.1.119 bash scripts/build_release.sh
```

## Remote Install/Update Evidence

Canonical command used on both stand hosts:

```bash
bash -o pipefail -c 'curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60 https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh | bash -s -- -install'
```

Laptop `art@192.168.31.21`:

```text
before_version=0.1.118
before_sha=e45674da938ef22e86b0fb4c1c95f444aff7116b5fe4d6c5aaa46586afe538c5
after_version=0.1.119
after_sha=6b3c0f7dd8d8d61d23dc45926fa35e26ea66bf78bb6e6b5116c6cd4b06b9c8d0
checksum_ok=true
```

VPS `root@91.124.19.180`:

```text
before_version=0.1.118
before_sha=e45674da938ef22e86b0fb4c1c95f444aff7116b5fe4d6c5aaa46586afe538c5
after_version=0.1.119
after_sha=6b3c0f7dd8d8d61d23dc45926fa35e26ea66bf78bb6e6b5116c6cd4b06b9c8d0
checksum_ok=true
```

Installed checksum matched the published release checksum on both hosts.

## Start/Status/Stop Evidence

Laptop:

```text
start_status=ok
node_service_state=active
transparent_runtime_service_state=inactive
runtime_state_status=up
route_mode=split
peer_egress_mode=node
peer_egress_resolved_local_listen=127.0.0.1:18135
peer_egress_resolved_peer_listen=0.0.0.0:38579
stop_status=ok
```

VPS:

```text
start_status=ok
node_service_state=active
transparent_runtime_service_state=inactive
runtime_state_status=up
route_mode=split
peer_egress_mode=node
peer_egress_resolved_local_listen=127.0.0.1:18135
peer_egress_resolved_peer_listen=0.0.0.0:42437
stop_status=ok
```

## Live WEAVE SSH Stand Proof

Temporary runtime only on laptop/VPS:

- VPS node:
  - local ingress: `127.0.0.1:19080`
  - peer ingress: `0.0.0.0:19081`
- laptop peer:
  - outbound peer connects to `91.124.19.180:19081`
- laptop local echo target:
  - `127.0.0.1:19091`

### Positive Control: Normal Local Ingress -> Peer Egress

Live request from VPS local ingress to laptop echo target:

```text
connect_response=$'OK\nECHO:hello-weave'
```

Peer/node runtime markers:

```text
peer_connected=true
peer_request_received=true
peer_target_connected=true
```

### Negative Path: `allow_pool_transit=false`

Setup:

- VPS node started with `--allow-pool-transit false`
- laptop peer pool was live and authenticated
- sealed transit was injected into VPS local ingress by shipped
  `chimera-peer-egress --mode sealed-transit-inject`

Observed result:

```text
deny_inject_rc=0
deny_inject_output=chimera_peer_egress_sealed_transit_inject=ok bytes=46
deny_branch=true
deny_forward=false
deny_error=true
```

Interpretation:

- injector succeeded in writing the test frame to local ingress;
- node entered the local sealed-transit branch;
- node did not forward the frame to the pool fallback;
- node emitted the transit error path instead.

This is the corrected real SSH stand proof for the `v0.1.118` policy bypass.

### Positive Transit Control: `allow_pool_transit=true`

Setup:

- the same VPS node was restarted with `--allow-pool-transit true`
- the same laptop peer stayed live
- the same sealed transit injection mode was used

Observed result:

```text
allow_inject_rc=0
allow_inject_output=chimera_peer_egress_sealed_transit_inject=ok bytes=46
allow_branch=true
allow_forward=true
allow_error=false
```

Interpretation:

- the local sealed-transit branch still works when policy allows pool fallback;
- the fix did not regress the allowed local forwarding branch.

Limit:

- this positive control proves the node-side local forwarding branch only;
- it is not a full multi-hop sealed-transit chain proof, because the laptop
  peer had no configured next-hop chain behind it.

## Redaction Evidence

VPS node log:

```text
node_payload_marker=false
node_target_literal=false
node_pppp=false
node_pppp_allow=false
```

Laptop peer log:

```text
peer_payload_marker=false
peer_target_literal=false
peer_pppp=false
peer_pppp_allow=false
```

Meaning:

- no raw `hello-weave` payload leaked into logs;
- no raw `PPPP` transit proof marker leaked into logs;
- no literal `127.0.0.1:19091` target leaked into logs.

## Status Boundary

Closed for this slice:

- `v0.1.119` GitHub `Latest` points to the fixed release;
- required release assets are present;
- laptop and VPS were updated only by the GitHub one-command install path;
- installed version/checksum match the published release on both hosts;
- `chimera.sh -start`, `-status`, and `-stop` passed on both hosts;
- live normal local ingress -> peer ingress -> local egress still works;
- real SSH deny-path confirms local sealed transit now fails closed when
  `allow_pool_transit=false` and a live peer pool exists;
- positive control confirms local sealed transit still forwards when
  `allow_pool_transit=true`;
- checked node/peer logs remained redacted in the exercised paths.

Not closed:

- full Real-World datapath PASS;
- transparent TUN/OS routing proof;
- DNS-to-route runtime binding proof;
- forced-stop/crash rollback proof;
- browser/IDE transparent workflow proof;
- full multi-hop sealed transit with a configured next-hop chain;
- long-run/load/performance proof.

## Risks and Limits

- The real stand proof here closes the local sealed-transit policy defect on
  the node path that was broken in `v0.1.118`.
- It does not close the broader mesh datapath, TUN/routing, DNS, rollback, or
  multi-hop carrier chain scopes.
- `Lab PASS` and this slice-level `partial_real_world_pass` must not be
  misreported as full MVP closure.
