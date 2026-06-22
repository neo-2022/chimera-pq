# Workflow Attestation: Remote Release/Update Installed Proof v0.1.109

Status: installed_release_update_pass
Date: 2026-06-18
Updated UTC: 2026-06-18

## Objective

Verify that GitHub Release/Latest `v0.1.109` can be installed or updated on the
authorized CHIMERA stand hosts using only the documented GitHub one-command
path over SSH.

Authorized stand hosts:

- side_b: authorized side_b SSH target, redacted in public proof
- SIDE_A: authorized SIDE_A SSH target, redacted in public proof

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
- FIX: proof_parser_corrected
- RECHECK: completed through corrected route-explain parsing and bad-bootstrap
  negative path on both hosts
- FINAL_AUDIT: completed by real sub-agent roles after evidence collection
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

- `v0.1.109` can be verified as a remote installed release/update proof after
  side_b and SIDE_A evidence is collected.
- The update source must remain GitHub Release/Latest.
- Installed proof must include version, checksum, route-explain diagnostics,
  redaction, negative bad-bootstrap behavior, and unchanged route/DNS state on
  both hosts.

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

- `tag_name`: `v0.1.109`
- release URL: `https://github.com/neo-2022/chimera-pq/releases/tag/v0.1.109`
- published: `2026-06-18T06:32:11Z`
- assets:
  - `chimera-pq-release.tar.gz`
  - `chimera-pq-release.tar.gz.sha256`
  - `chimera.sh`

Release checksum file:

```text
49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a  chimera-pq-release.tar.gz
```

## SSH Preflight

Side B:

```text
host=side_b ssh_ok=true
curl=present
tar=present
sha256sum=present
jq=present
Linux 7.0.0-22-generic x86_64
```

SIDE_A:

```text
host=side_a ssh_ok=true
curl=present
tar=present
sha256sum=present
jq=present
Linux 6.8.0-124-generic x86_64
```

## Install/Update Evidence

Side B:

```text
host=side_b proof=idempotent_update before_version=0.1.109 before_sha=49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a install_rc=0 install_ok=true after_version=0.1.109 after_sha=49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a expected_sha=49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a checksum_ok=true launcher_version=chimera-runtime 0.1.109 network_unchanged=false
host=side_b proof=idempotent_update_normalized before_version=0.1.109 before_sha=49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a install_rc=0 install_ok=true after_version=0.1.109 after_sha=49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a expected_sha=49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a checksum_ok=true raw_network_same=false normalized_network_same=true only_timer_diff=true diff_lines=24
host=side_b proof=route_dns_update before_version=0.1.109 before_sha=49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a install_rc=0 install_ok=true after_version=0.1.109 after_sha=49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a expected_sha=49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a checksum_ok=true route_dns_unchanged=true
```

The first side_b network fingerprint included live address lifetime counters,
which changed during the check. A normalized address check showed only timer
differences. A separate route/rule/DNS fingerprint stayed unchanged.

SIDE_A:

```text
host=side_a proof=idempotent_update before_version=0.1.109 before_sha=49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a install_rc=0 install_ok=true after_version=0.1.109 after_sha=49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a expected_sha=49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a checksum_ok=true launcher_version=chimera-runtime 0.1.109 network_unchanged=true
host=side_a proof=route_dns_update before_version=0.1.109 before_sha=49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a install_rc=0 install_ok=true after_version=0.1.109 after_sha=49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a expected_sha=49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a checksum_ok=true route_dns_unchanged=true
```

## Installed Binary Proof

Route-explain proof was run with installed `chimera-cli` and reserved TEST-NET
simulation endpoints. The initial proof parser incorrectly expected `.explain`
to be a JSON array and used `mesh_multipath_demand=normal`, which correctly
planned one active lane. The corrected proof parsed the installed string
`.explain` and used `mesh_multipath_demand=high` to verify two active lanes.

Side B:

```text
host=side_b proof=route_explain rc=0 schema_ok=true status=ok kind=mesh_route_explain network_state=not_modified node_redacted=<redacted> active_lanes=2 carrier_bindings=2 demand_policy=high lane_requested=2 lane_admitted=2 lane_rejected=0 capacity_status=within_budget lane_math_ok=true sealed_opaque_ok=true execution_status=carrier_lane_binding_contract_ready execution_status_ok=true binding_status=carrier_lane_binding_contract_ready binding_status_ok=true redaction_ok=true
```

SIDE_A:

```text
host=side_a proof=route_explain rc=0 schema_ok=true status=ok kind=mesh_route_explain network_state=not_modified node_redacted=<redacted> active_lanes=2 carrier_bindings=2 demand_policy=high lane_requested=2 lane_admitted=2 lane_rejected=0 capacity_status=within_budget lane_math_ok=true sealed_opaque_ok=true execution_status=carrier_lane_binding_contract_ready execution_status_ok=true binding_status=carrier_lane_binding_contract_ready binding_status_ok=true redaction_ok=true
```

Route-explain proof verified:

- `multipath_schedule_demand_policy=high`;
- `multipath_schedule_active_lanes=2`;
- `multipath_schedule_carrier_bindings=2`;
- `multipath_schedule_lane_admission_requested_active_lanes=2`;
- `multipath_schedule_lane_admission_admitted_active_lanes=2`;
- `multipath_schedule_lane_admission_rejected_active_lanes=0`;
- `multipath_schedule_lane_admission_capacity_status=within_budget`;
- `multipath_schedule_transit_payload_policy=sealed_opaque_only`;
- `multipath_schedule_execution_status=carrier_lane_binding_contract_ready`;
- `multipath_schedule_carrier_binding_contract=carrier_lane_binding_contract_ready`;
- no raw test node id, TEST-NET endpoint, route binding id, invite token, or
  payload marker in the output.

## Negative Bad-Bootstrap Proof

Negative bad-bootstrap proof used missing GitHub release asset URLs and
`bash -o pipefail`. It returned nonzero on both hosts and left installed
version, checksum, routes, and DNS unchanged.

Side B:

```text
host=side_b proof=bad_bootstrap bad_rc=1 bad_nonzero=true before_version=0.1.109 after_version=0.1.109 before_sha=49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a after_sha=49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a unchanged=true route_dns_unchanged=true bad_redaction_ok=true
```

SIDE_A:

```text
host=side_a proof=bad_bootstrap bad_rc=1 bad_nonzero=true before_version=0.1.109 after_version=0.1.109 before_sha=49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a after_sha=49238ff1ccf2adb47a597d8af7639a93f7fdae5fbf6d5b8a4adbf2c82c58490a unchanged=true route_dns_unchanged=true bad_redaction_ok=true
```

## Status Boundary

Status: installed release/update proof PASS for `v0.1.109`.

Closed:

- GitHub Latest points to `v0.1.109`.
- Required release assets are present.
- Side B and SIDE_A were updated by GitHub one-command install/update only.
- Installed version and release bundle checksum match on both hosts.
- Installed route-explain exposes lane-admission diagnostics and
  `sealed_opaque_only` transit payload policy.
- Public route-explain redaction passed on both hosts.
- Bad-bootstrap negative path failed closed and did not alter installed version.
- Routes, rules, and resolver state remained unchanged.
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

- The proof uses installed route-explain simulation for diagnostics and
  redaction. It does not prove live TUN/datapath traffic.
- There is no separate signature artifact in this release proof; the verified
  supply-chain property here is checksum match against the published release
  checksum file.
- Side B GitHub downloads can take longer than SIDE_A downloads; bounded retry and
  fail-closed behavior remain required.
