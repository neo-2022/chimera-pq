# Workflow Attestation: Remote Release/Update and Live WEAVE Proof v0.1.112

Status: partial_real_world_pass
Date: 2026-06-18

## Objective

Publish `v0.1.112` as GitHub Release/Latest and verify laptop/VPS install/update
using only the GitHub one-command path over SSH.

Also run a bounded SSH stand proof for the shipped WEAVE peer-egress runtime:

- local ingress on VPS;
- peer ingress on VPS;
- peer egress on laptop;
- sealed transit local-ingress forwarding branch on VPS.

## Council Notes

Real sub-agent roles were used for architecture, development, testing,
security, DevOps/release and critic review.

Accepted:

- release from exact pushed tag when GitHub Actions release workflow was stuck
  on the infrastructure `apt-get` step;
- cancel the stuck workflow before manual release publication to avoid asset
  races;
- keep stand install/update proof strictly on GitHub Release/Latest
  one-command install.

Rejected:

- calling `v0.1.111` laptop stand PASS after laptop install hung in the
  unbounded `wget` fallback;
- using `scp`, `rsync`, local tarball, target artifacts, `git clone`, `cargo`
  or local PC runtime as stand proof;
- reporting full TUN/DNS/rollback/browser workflow as closed.

## Source and Release Evidence

Source commits:

```text
v0.1.111 commit: 46ea30f42acb31a41a838140af9fba14d8375690
v0.1.112 commit: ac403fa14cc5536cc489f08814564caf5f0de777
```

`v0.1.111` result:

- VPS updated and started successfully.
- Laptop install found a real defect: `curl` timed out and `wget` fallback had
  no timeout, leaving a 0-byte archive download in progress.
- The stuck laptop install was killed; laptop remained at `0.1.110` with the
  previous checksum.

Fix in `v0.1.112`:

- bounded `wget` fallback in:
  - `scripts/chimera.sh`;
  - `scripts/install_release.sh`;
  - `scripts/chimera_runtime_bootstrap.sh`;
- `scripts/chimera_installer_gate.sh` now checks the bounded `wget` contract.

GitHub Latest after publication:

```text
tag: v0.1.112
url: https://github.com/neo-2022/chimera-pq/releases/tag/v0.1.112
assets:
  chimera-pq-release.tar.gz
  chimera-pq-release.tar.gz.sha256
  chimera.sh
sha256: fc510f75ea292c8eed784b66480973b8ca4d23067a3d05b840c2362355b16050
```

CI release workflow note:

- GitHub Actions release runs for `v0.1.111` and `v0.1.112` were cancelled
  after they stuck on `Install Release Gate Tools`.
- Manual release publication was done from the exact pushed tag after local
  release bundle verification.

## Source Gates

Passed before release:

```text
cargo fmt --all -- --check
cargo check -q --workspace
cargo test -q --workspace
cargo test -q -p chimera-carrier
cargo test -q -p chimera-mesh
cargo clippy -q -p chimera-carrier --all-targets -- -D warnings
cargo clippy -q -p chimera-mesh --all-targets -- -D warnings
bash scripts/anti_monolith_guard.sh
bash scripts/chimera_installer_gate.sh
bash scripts/chimera_update_contract_smoke.sh
bash scripts/chimera_start_contract_smoke.sh
bash scripts/chimera_stop_contract_smoke.sh
just rust-no-hardcode-guard
```

## Remote Install/Update Evidence

Canonical command used on both stand hosts:

```bash
bash -o pipefail -c 'curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60 https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh | bash -s -- -install'
```

Laptop:

```text
before_version=0.1.110
before_sha=90b7bd93f81db4eb98679e71d084e53293d873791be3b2e52621522216348c60
after_version=0.1.112
after_sha=fc510f75ea292c8eed784b66480973b8ca4d23067a3d05b840c2362355b16050
checksum_ok=true
start_status=ok
node_service_state=active
transparent_runtime_service_state=inactive
runtime_state_status=up
route_mode=split
stop_status=ok
```

VPS:

```text
before_version=0.1.111
before_sha=14214f3c004adb28ac94e1392382ee3232f4ef4f632e44d8cd60fdc193ca1123
after_version=0.1.112
after_sha=fc510f75ea292c8eed784b66480973b8ca4d23067a3d05b840c2362355b16050
checksum_ok=true
start_status=ok
node_service_state=active
transparent_runtime_service_state=inactive
runtime_state_status=up
route_mode=split
stop_status=ok
```

Negative bootstrap proof on both:

```text
bad_rc=1
before_version=0.1.112
after_version=0.1.112
before_sha=fc510f75ea292c8eed784b66480973b8ca4d23067a3d05b840c2362355b16050
after_sha=fc510f75ea292c8eed784b66480973b8ca4d23067a3d05b840c2362355b16050
unchanged=true
```

## Live WEAVE Stand Proof

Temporary runtime only on laptop/VPS:

- VPS WEAVE node:
  - local ingress: `127.0.0.1:19080`;
  - peer ingress: `0.0.0.0:19081`;
- laptop WEAVE peer connected to VPS peer ingress;
- laptop local echo target: `127.0.0.1:19091`.

VPS node reported:

```text
chimera_peer_egress=node_ready
capabilities=local_ingress,peer_ingress,local_egress,peer_transit
```

Local ingress to peer egress proof:

```text
ack=OK\n
body=ECHO:hello-weave
```

CHIMERA node logs:

```text
event=weave_peer_ingress_authenticated
event=weave_local_ingress_accepted
event=local_ingress_destination host=<redacted> port=<redacted>
event=local_ingress_paired_with_peer
event=peer_connect_request_sent request=<redacted>
event=peer_connect_ack_received
raw_destination_in_node_logs=false
```

CHIMERA peer logs:

```text
chimera_peer_egress=laptop_connecting server=<redacted>
event=outbound_peer_connected
event=outbound_peer_request_received request=<redacted>
event=outbound_peer_target_connecting target=<redacted>
event=outbound_peer_target_connected target=<redacted>
event=outbound_peer_connect_ack_sent target=<redacted>
raw_destination_in_peer_logs=false
```

Sealed transit local-ingress forwarding branch:

```text
chimera_peer_egress_sealed_transit_inject=ok bytes=46
event=weave_local_ingress_transit_branch
event=weave_transit_frame_forwarded
payload_marker_in_node_logs=false
payload_marker_in_peer_logs=false
```

Limit:

- the sealed transit proof verified the local-ingress sealed forwarding branch
  and redaction on the stand;
- it did not prove a full multi-hop transit chain because no next-hop chain was
  configured for the peer after receiving the sealed transit frame.

Cleanup:

```text
vps_temp_pid_alive=false
peer.pid alive=false
echo.pid alive=false
```

## Status Boundary

Closed:

- `v0.1.112` GitHub Latest points to the new release;
- required Latest assets are present;
- release checksum matches on GitHub and installed hosts;
- laptop and VPS install/update used GitHub one-command only;
- `chimera.sh -start`, `-status`, `-stop` passed on both hosts;
- bad bootstrap failed closed on both hosts;
- live local-ingress -> peer-ingress -> peer-egress proof passed on laptop/VPS;
- CHIMERA logs redacted raw destination and proof payload in the checked paths;
- sealed transit local-ingress forwarding branch was exercised on the stand.

Not closed:

- full Real-World datapath PASS;
- transparent TUN/OS routing proof;
- DNS-to-route runtime binding proof;
- forced-stop/crash rollback proof;
- browser/IDE transparent workflow proof;
- full multi-hop sealed transit with a configured next-hop chain;
- long-run/load/performance proof.

## Risks and Limits

- GitHub Actions release workflow was not usable in this run because it stuck on
  an infrastructure package-install step. Release was published manually from
  exact tag after local artifact verification.
- Laptop GitHub download path is slow and initially timed out through `curl`;
  bounded `wget` fallback in `v0.1.112` completed successfully.
- The live proof used temporary high ports and explicit test echo target. It is
  a real SSH stand proof for WEAVE peer-egress, but not a full transparent VPN
  user workflow proof.
