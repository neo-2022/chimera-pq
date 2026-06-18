# Workflow Attestation: Remote Release/Update Installed Proof v0.1.110

Status: installed_release_update_pass
Date: 2026-06-18
Updated UTC: 2026-06-18T09:26:30Z

## Objective

Verify that GitHub Release/Latest `v0.1.110` can be installed or updated on the
authorized CHIMERA stand hosts using only the documented GitHub one-command
path over SSH.

Authorized stand hosts:

- laptop: authorized laptop SSH target
- VPS: authorized VPS SSH target

The stand install/update command used GitHub Release/Latest only:

```bash
bash -o pipefail -c 'curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60 https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh | bash -s -- -install'
```

No `scp`, `rsync`, local tarball, `cargo`, `git clone`, local `target/`, or
local PC runtime path was used as stand proof.

## Council Consensus

Real sub-agent reviews were used for release/install path, policy, release
workflow, and host-assumption checks.

Agreed:

- `v0.1.110` is the current GitHub Latest release.
- The one-command GitHub install/update path is the required stand path.
- GitHub Latest is the source of truth for install/update proof.
- Negative bootstrap failure must fail closed and leave the installed version
  unchanged.

Rejected:

- reporting this as full Real-World datapath PASS;
- reporting live node-to-node transit, TUN/OS routing, DNS binding, forced
  rollback, or browser/IDE workflow as verified;
- any public log or report containing tokens, passwords, private keys, raw
  endpoints, route binding ids, or payload markers.

## GitHub Latest Evidence

GitHub API Latest:

- `tag_name`: `v0.1.110`
- release URL: `https://github.com/neo-2022/chimera-pq/releases/tag/v0.1.110`
- published: `2026-06-18T08:05:36Z`
- assets:
  - `chimera.sh`
  - `chimera-pq-release.tar.gz`
  - `chimera-pq-release.tar.gz.sha256`

Release archive digest:

```text
sha256:90b7bd93f81db4eb98679e71d084e53293d873791be3b2e52621522216348c60
```

## SSH Preflight

Laptop:

```text
ssh_ok=true
curl=present
tar=present
sha256sum=present
installed_version=0.1.110
```

VPS:

```text
ssh_ok=true
curl=present
tar=present
sha256sum=present
installed_version=0.1.110
```

## Install/Update Evidence

Laptop:

```text
before_version=0.1.110
before_sha=90b7bd93f81db4eb98679e71d084e53293d873791be3b2e52621522216348c60
install_rc=0
after_version=0.1.110
after_sha=90b7bd93f81db4eb98679e71d084e53293d873791be3b2e52621522216348c60
```

Observed note: several bounded `curl` attempts timed out on the laptop, but the
install still completed successfully.

VPS:

```text
before_version=0.1.110
before_sha=90b7bd93f81db4eb98679e71d084e53293d873791be3b2e52621522216348c60
install_rc=0
after_version=0.1.110
after_sha=90b7bd93f81db4eb98679e71d084e53293d873791be3b2e52621522216348c60
```

## Installed Binary Proof

Laptop:

```text
start_status=ok mode=systemd_user node_runtime=running node=started transparent_runtime=stopped endpoint=unconfigured
node_service_state=active
transparent_runtime_service_state=inactive
runtime_state_status=up
route_mode=split
```

VPS:

```text
start_status=ok mode=systemd_user node_runtime=running node=started transparent_runtime=stopped endpoint=unconfigured
node_service_state=active
transparent_runtime_service_state=inactive
runtime_state_status=up
route_mode=split
```

## Negative Bootstrap Proof

Laptop:

```text
bad_rc=22
before_version=0.1.110
after_version=0.1.110
before_sha=90b7bd93f81db4eb98679e71d084e53293d873791be3b2e52621522216348c60
after_sha=90b7bd93f81db4eb98679e71d084e53293d873791be3b2e52621522216348c60
```

VPS:

```text
bad_rc=22
before_version=0.1.110
after_version=0.1.110
before_sha=90b7bd93f81db4eb98679e71d084e53293d873791be3b2e52621522216348c60
after_sha=90b7bd93f81db4eb98679e71d084e53293d873791be3b2e52621522216348c60
```

## Status Boundary

Status: installed release/update proof PASS for `v0.1.110`.

Closed:

- GitHub Latest points to `v0.1.110`.
- Required release assets are present.
- Laptop and VPS were updated by GitHub one-command install/update only.
- Installed version and release bundle checksum match on both hosts.
- `chimera.sh -start`, `-status`, and `-stop` passed on both hosts.
- Bad-bootstrap negative path failed closed and did not alter installed version.

Not closed by this document:

- full Real-World datapath PASS;
- node-to-node live carrier traffic between laptop and VPS;
- actual transit forwarding of third-party traffic;
- transparent TUN/OS routing behavior;
- DNS-to-route runtime binding;
- crash/forced-stop rollback;
- browser/IDE transparent workflow;
- real multipath carrier throughput or long-run stability.

## Risks And Limits

- GitHub timeouts were observed on the laptop before retry success.
- The proof uses installed route-explain/runtime diagnostics and does not prove
  live TUN/datapath traffic.
- The negative path was checked against a missing GitHub release asset URL, not
  against a malformed archive body.
