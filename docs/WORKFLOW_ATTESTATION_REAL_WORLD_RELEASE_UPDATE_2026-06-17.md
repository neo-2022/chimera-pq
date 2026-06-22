# Workflow Attestation: Remote Release/Update Installed Proof

Status: installed_release_update_pass
Date: 2026-06-17
Updated UTC: 2026-06-17T20:51:20Z

## Objective

Verify that GitHub Release/Latest `v0.1.107` can be installed or updated on the
authorized CHIMERA stand hosts using only the documented GitHub one-command
path over SSH.

Authorized stand hosts:

- side_b: authorized side_b SSH target, redacted in public proof
- SIDE_A: authorized SIDE_A SSH target, reached through the side_b as SSH jump host;
  target value redacted in public proof

The stand install/update command used GitHub Release/Latest only:

```bash
bash -o pipefail -c 'curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60 https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh | bash -s -- -install'
```

No `scp`, `rsync`, local tarball, `cargo`, `git clone`, local `target/`, or
local PC runtime path was used as stand proof.

## Required Cycle

```text
ANALYSIS -> PLAN -> TEAM_CRITIQUE -> IMPLEMENTATION -> TEAM_CHECK -> FIX
-> RECHECK -> FINAL_AUDIT -> REPORT
```

- ANALYSIS: completed
- PLAN: completed
- TEAM_CRITIQUE: completed with real sub-agent roles
- IMPLEMENTATION: completed on side_b and SIDE_A through GitHub one-command
- TEAM_CHECK: completed through installed-binary proof on both hosts
- FIX: not_needed
- RECHECK: completed through bad-bootstrap negative path on both hosts
- FINAL_AUDIT: completed
- REPORT: this document

## Council Consensus

Real sub-agent roles used for this cycle:

- architect
- senior Rust/release engineer
- tester
- security engineer
- DevOps engineer
- critic-skeptic

Agreed:

- `v0.1.107` can be verified as a remote installed release/update proof.
- The update source must remain GitHub Release/Latest.
- The SIDE_A SSH jump host is only a transport path, not a release source.
- Installed proof must include version, checksum, route-explain diagnostics,
  redaction, and negative bad-bootstrap behavior on both hosts.

Rejected:

- reporting this as full Real-World datapath PASS;
- reporting node-to-node transit traffic, TUN/OS routing, DNS binding, forced
  rollback, browser/IDE workflow, or real multipath carrier traffic as verified;
- stand proof through `scp`, `rsync`, local tarball, `cargo`, `git clone`, local
  binaries, or local PC runtime;
- any public log or report containing tokens, passwords, private keys, raw
  endpoints, route binding ids, or payload markers.

## GitHub Latest Evidence

GitHub API Latest:

- `tag_name`: `v0.1.107`
- release URL: `https://github.com/neo-2022/chimera-pq/releases/tag/v0.1.107`
- published: `2026-06-17T20:34:11Z`
- assets:
  - `chimera.sh`
  - `chimera-pq-release.tar.gz`
  - `chimera-pq-release.tar.gz.sha256`

Release checksum file:

```text
4ed938abf6009327445e3d1c4990bad4693204c8572d57283db0e3004fac92af  chimera-pq-release.tar.gz
```

Downloaded Latest bootstrap header:

```text
VERSION="0.1.107"
ARCHIVE_URL_DEFAULT="https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera-pq-release.tar.gz"
CHECKSUM_URL_DEFAULT="https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera-pq-release.tar.gz.sha256"
```

## SSH Preflight

Side B:

```text
host=side_b ssh_ok=true user=<redacted> home=<redacted>
curl=present
tar=present
sha256sum=present
Linux 7.0.0-22-generic x86_64
```

SIDE_A:

```text
host=side_a ssh_ok=true user=<redacted> home=<redacted>
curl=present
tar=present
sha256sum=present
Linux 6.8.0-124-generic x86_64
```

## Install/Update Evidence

Side B:

```text
host=side_b phase=before version=0.1.106 bundle_sha=621ea634eb3346f7c7ebbd2505b563036dbe87bfe57758a1f7cada4f60541a97
host=side_b install_rc=0
host=side_b phase=after version=0.1.107 bundle_sha=4ed938abf6009327445e3d1c4990bad4693204c8572d57283db0e3004fac92af network_unchanged=true
```

Observed note: several `curl` attempts to GitHub timed out on the side_b, but
the bounded retry path completed and installed `0.1.107`.

SIDE_A:

```text
host=side_a phase=before version=0.1.106 bundle_sha=621ea634eb3346f7c7ebbd2505b563036dbe87bfe57758a1f7cada4f60541a97
host=side_a install_rc=0
host=side_a phase=after version=0.1.107 bundle_sha=4ed938abf6009327445e3d1c4990bad4693204c8572d57283db0e3004fac92af network_unchanged=true
```

## Installed Binary Proof

Side B:

```text
host=side_b proof=installed version_marker=0.1.107 launcher_version="chimera-runtime 0.1.107" cli_executable=true version_ok=true marker_checksum_ok=true archive_checksum_ok=true
host=side_b proof=route_explain lane_requested=2 lane_admitted=2 lane_rejected=0 capacity_status=within_budget lane_math_ok=true sealed_opaque_ok=true execution_status_ok=true binding_status_ok=true redaction_markers_ok=true redaction_ok=true
host=side_b proof=bad_bootstrap bad_rc=22 bad_nonzero=true version_after_bad=0.1.107 checksum_after_bad=4ed938abf6009327445e3d1c4990bad4693204c8572d57283db0e3004fac92af bad_unchanged=true bad_redaction_ok=true network_unchanged=true
```

SIDE_A:

```text
host=side_a proof=installed version_marker=0.1.107 launcher_version="chimera-runtime 0.1.107" cli_executable=true version_ok=true marker_checksum_ok=true archive_checksum_ok=true
host=side_a proof=route_explain lane_requested=2 lane_admitted=2 lane_rejected=0 capacity_status=within_budget lane_math_ok=true sealed_opaque_ok=true execution_status_ok=true binding_status_ok=true redaction_markers_ok=true redaction_ok=true
host=side_a proof=bad_bootstrap bad_rc=22 bad_nonzero=true version_after_bad=0.1.107 checksum_after_bad=4ed938abf6009327445e3d1c4990bad4693204c8572d57283db0e3004fac92af bad_unchanged=true bad_redaction_ok=true network_unchanged=true
```

Route-explain proof was run with installed `chimera-cli` and reserved TEST-NET
simulation endpoints. It verified:

- `multipath_schedule_lane_admission_requested_active_lanes`;
- `multipath_schedule_lane_admission_admitted_active_lanes`;
- `multipath_schedule_lane_admission_rejected_active_lanes`;
- `multipath_schedule_lane_admission_capacity_status`;
- `multipath_schedule_transit_payload_policy=sealed_opaque_only`;
- `multipath_schedule_execution_status=carrier_lane_binding_contract_ready`;
- `multipath_schedule_carrier_binding_contract=carrier_lane_binding_contract_ready`;
- public redaction markers for peer endpoints;
- no raw test peer ids, TEST-NET endpoints, route binding id, invite token, or
  payload marker in the output.

Negative bad-bootstrap proof used a missing GitHub release asset URL and
`bash -o pipefail`. It returned nonzero on both hosts and left installed version
and checksum unchanged.

## Status Boundary

Status: installed release/update proof PASS for `v0.1.107`.

Closed:

- GitHub Latest points to `v0.1.107`.
- Required release assets are present.
- Side B and SIDE_A were updated by GitHub one-command install/update only.
- Installed version and release bundle checksum match on both hosts.
- Installed route-explain exposes lane-admission diagnostics and
  `sealed_opaque_only` transit payload policy.
- Public route-explain redaction passed on both hosts.
- Bad-bootstrap negative path failed closed and did not alter installed version.
- Local PC CHIMERA runtime was not started.

Not closed by this document:

- full Real-World datapath PASS;
- node-to-node live carrier traffic between side_b and SIDE_A;
- actual transit forwarding of third-party traffic;
- transparent TUN/OS routing behavior;
- DNS-to-route runtime binding;
- crash/forced-stop rollback;
- browser/IDE transparent workflow;
- real multipath carrier throughput or long-run stability.

## Risks And Limits

- GitHub timeouts were observed on the side_b before retry success; future
  update proofs should continue to keep bounded retries and fail-closed behavior.
- The proof uses installed route-explain simulation for diagnostics and
  redaction. It does not prove live TUN/datapath traffic.
- There is no separate signature artifact in this release proof; the verified
  supply-chain property here is checksum match against the published release
  checksum file.
