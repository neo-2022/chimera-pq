# Workflow Attestation: Remote Release/Update Installed Proof v0.1.108

Status: installed_release_update_pass
Date: 2026-06-17
Updated UTC: 2026-06-17T22:14:57Z

## Objective

Verify that GitHub Release/Latest `v0.1.108` can be installed or updated on the
authorized CHIMERA stand hosts using only the documented GitHub one-command
path over SSH.

Authorized stand hosts:

- laptop: authorized laptop SSH target, redacted in public proof
- VPS: authorized VPS SSH target, reached through the laptop as SSH jump host;
  target value redacted in public proof

The stand install/update command used GitHub Release/Latest only:

```bash
bash -o pipefail -c 'curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60 https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh | bash -s -- -install'
```

No `scp`, `rsync`, local tarball, `cargo`, `git clone`, local `target/`, or
local PC runtime path was used as stand proof.

Peer-update fallback was not used in this proof. It remains an update-only
fallback for already installed CHIMERA nodes when GitHub Latest is unreachable;
it is not a valid first-install or stand proof source.

## Required Cycle

```text
ANALYSIS -> PLAN -> TEAM_CRITIQUE -> IMPLEMENTATION -> TEAM_CHECK -> FIX
-> RECHECK -> FINAL_AUDIT -> REPORT
```

- ANALYSIS: completed
- PLAN: completed
- TEAM_CRITIQUE: completed with real sub-agent roles
- IMPLEMENTATION: completed on laptop and VPS through GitHub one-command
- TEAM_CHECK: completed through installed-binary proof on both hosts
- FIX: proof_parser_corrected
- RECHECK: completed through corrected route-explain parsing and bad-bootstrap
  negative path on both hosts
- FINAL_AUDIT: pending at document creation
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

- `v0.1.108` can be verified as a remote installed release/update proof after
  laptop and VPS evidence is collected.
- The update source must remain GitHub Release/Latest.
- The VPS SSH jump host is only a transport path, not a release source.
- Installed proof must include version, checksum, route-explain demand
  diagnostics, redaction, negative bad-bootstrap behavior, and unchanged network
  state on both hosts.
- Peer-update fallback is update-only for already installed CHIMERA nodes when
  GitHub Latest is unreachable. It is not accepted as first-install or stand
  proof, and it must not run after invalid GitHub metadata, checksum, or source.

Rejected:

- reporting this as full Real-World datapath PASS;
- reporting node-to-node transit traffic, TUN/OS routing, DNS binding, forced
  rollback, browser/IDE workflow, or real multipath carrier traffic as verified;
- stand proof through `scp`, `rsync`, local tarball, `cargo`, `git clone`, local
  binaries, or local PC runtime;
- peer-update fallback as first-install proof, stand proof, or trust source
  after invalid GitHub metadata, checksum, or source;
- any public log or report containing tokens, passwords, private keys, raw
  endpoints, route binding ids, or payload markers.

## GitHub Latest Evidence

GitHub API Latest:

- `tag_name`: `v0.1.108`
- release URL: `https://github.com/neo-2022/chimera-pq/releases/tag/v0.1.108`
- published: `2026-06-17T21:57:24Z`
- assets:
  - `chimera.sh`
  - `chimera-pq-release.tar.gz`
  - `chimera-pq-release.tar.gz.sha256`

Release checksum file:

```text
eded683686dcc089a683079e91d412bf38030ccc3564fa77aca1a19ccd2f6bd9  chimera-pq-release.tar.gz
```

Downloaded Latest bootstrap header:

```text
VERSION="0.1.108"
ARCHIVE_URL_DEFAULT="https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera-pq-release.tar.gz"
CHECKSUM_URL_DEFAULT="https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera-pq-release.tar.gz.sha256"
```

Release workflow evidence:

- CI run `27722275765`: `completed`, `success`
- Release run `27722277575`: `completed`, `success`
- release commit: `d2b2ce3bf14570047e086420c3f08b30f3a92ad9`

## SSH Preflight

Laptop:

```text
host=laptop ssh_ok=true
curl=present
tar=present
sha256sum=present
jq=present
Linux 7.0.0-22-generic x86_64
```

VPS:

```text
host=vps ssh_ok=true via_jump=true
curl=present
tar=present
sha256sum=present
jq=present
Linux 6.8.0-124-generic x86_64
```

Direct SSH from the local PC to the VPS timed out during banner exchange in this
session. The VPS proof was therefore run through the authorized laptop jump
host. The jump host was not used as a release source.

## Install/Update Evidence

Laptop:

```text
host=laptop phase=before version=0.1.107 bundle_sha=4ed938abf6009327445e3d1c4990bad4693204c8572d57283db0e3004fac92af expected_sha=4ed938abf6009327445e3d1c4990bad4693204c8572d57283db0e3004fac92af
host=laptop install_rc=0
host=laptop phase=after version=0.1.108 bundle_sha=eded683686dcc089a683079e91d412bf38030ccc3564fa77aca1a19ccd2f6bd9 expected_sha=eded683686dcc089a683079e91d412bf38030ccc3564fa77aca1a19ccd2f6bd9 archive_checksum_ok=true
host=laptop launcher_version=chimera-runtime 0.1.108
host=laptop cli_executable=true
```

VPS:

```text
host=vps phase=before version=0.1.107 bundle_sha=4ed938abf6009327445e3d1c4990bad4693204c8572d57283db0e3004fac92af expected_sha=4ed938abf6009327445e3d1c4990bad4693204c8572d57283db0e3004fac92af
host=vps install_rc=0
host=vps phase=after version=0.1.108 bundle_sha=eded683686dcc089a683079e91d412bf38030ccc3564fa77aca1a19ccd2f6bd9 expected_sha=eded683686dcc089a683079e91d412bf38030ccc3564fa77aca1a19ccd2f6bd9 archive_checksum_ok=true
host=vps launcher_version=chimera-runtime 0.1.108
host=vps cli_executable=true
```

## Installed Binary Proof

Route-explain proof was run with installed `chimera-cli` and reserved TEST-NET
simulation endpoints. The first laptop proof parser incorrectly treated
`.explain` as an array and reported missing demand fields. The installed JSON
stores `.explain` as a string, so the proof parser was corrected to parse pipe
and semicolon separated explain lines. The corrected proof is recorded below.

Laptop:

```text
host=laptop corrected_proof=route_explain status_ok=true kind_ok=true contract_ok=true network_state=not_modified demand_policy=high demand_requested=2 demand_planned=2 demand_capacity_pct=90 demand_status=within_budget demand_fields_ok=true demand_math_ok=true lane_requested=2 lane_admitted=2 lane_rejected=0 capacity_status=within_budget lane_math_ok=true sealed_opaque_ok=true execution_status_ok=true binding_status_ok=true redaction_markers_ok=true redaction_ok=true
host=laptop proof=bad_bootstrap bad_rc=22 bad_nonzero=true version_after_bad=0.1.108 checksum_after_bad=eded683686dcc089a683079e91d412bf38030ccc3564fa77aca1a19ccd2f6bd9 bad_unchanged=true bad_redaction_ok=true
host=laptop corrected_proof=network routes_unchanged=true addr_normalized_unchanged=true resolv_unchanged=true network_unchanged=true
host=laptop corrected_summary version_ok=true checksum_ok=true archive_checksum_ok=true route_explain_demand_ok=true redaction_ok=true bad_bootstrap_nonzero=true bad_unchanged=true network_unchanged=true
```

VPS:

```text
host=vps proof=route_explain route_rc=0 status_ok=true kind_ok=true contract_ok=true network_state=not_modified demand_policy=high demand_requested=2 demand_planned=2 demand_capacity_pct=90 demand_status=within_budget demand_fields_ok=true demand_math_ok=true lane_requested=2 lane_admitted=2 lane_rejected=0 capacity_status=within_budget lane_math_ok=true sealed_opaque_ok=true execution_status_ok=true binding_status_ok=true redaction_markers_ok=true redaction_ok=true
host=vps proof=bad_bootstrap bad_rc=22 bad_nonzero=true version_after_bad=0.1.108 checksum_after_bad=eded683686dcc089a683079e91d412bf38030ccc3564fa77aca1a19ccd2f6bd9 bad_unchanged=true bad_redaction_ok=true
host=vps proof=network routes_unchanged=true addr_normalized_unchanged=true resolv_unchanged=true network_unchanged=true
host=vps summary version_ok=true checksum_ok=true archive_checksum_ok=true route_explain_demand_ok=true redaction_ok=true bad_bootstrap_nonzero=true bad_unchanged=true network_unchanged=true
```

Route-explain proof verified:

- `multipath_schedule_demand_policy=high`;
- `multipath_schedule_demand_requested_active_lanes=2`;
- `multipath_schedule_demand_planned_active_lanes=2`;
- `multipath_schedule_demand_admitted_lane_capacity_pct=90`;
- `multipath_schedule_demand_status=within_budget`;
- demand math is consistent;
- lane admission math is consistent;
- `multipath_schedule_transit_payload_policy=sealed_opaque_only`;
- `multipath_schedule_execution_status=carrier_lane_binding_contract_ready`;
- `multipath_schedule_carrier_binding_contract=carrier_lane_binding_contract_ready`;
- public redaction markers for peer endpoints;
- no raw test peer ids, TEST-NET endpoints, route binding id, invite token, or
  payload marker in the output.

Negative bad-bootstrap proof used a missing GitHub release asset URL and
`bash -o pipefail`. It returned nonzero on both hosts and left installed version
and checksum unchanged.

Network unchanged proof compared routes, resolver checksum, and normalized
address state. Address lifetime fields were normalized to avoid false changes
from timer countdowns.

## Status Boundary

Status: installed release/update proof PASS for `v0.1.108`.

Closed:

- GitHub Latest points to `v0.1.108`.
- Required release assets are present.
- Laptop and VPS were updated by GitHub one-command install/update only.
- Installed version and release bundle checksum match on both hosts.
- Installed route-explain exposes demand-aware lane planning diagnostics,
  lane-admission diagnostics, and `sealed_opaque_only` transit payload policy.
- Public route-explain redaction passed on both hosts.
- Bad-bootstrap negative path failed closed and did not alter installed version.
- Routes, resolver state, and normalized address state remained unchanged.
- Local PC CHIMERA runtime was not started.

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

- Direct SSH from local PC to VPS timed out during banner exchange; VPS proof
  used the authorized laptop jump host.
- The proof uses installed route-explain simulation for diagnostics and
  redaction. It does not prove live TUN/datapath traffic.
- There is no separate signature artifact in this release proof; the verified
  supply-chain property here is checksum match against the published release
  checksum file.
- A private credentials note was read during the session. The proof document
  does not contain secrets, but exposed credentials should be rotated outside
  this proof cycle.
