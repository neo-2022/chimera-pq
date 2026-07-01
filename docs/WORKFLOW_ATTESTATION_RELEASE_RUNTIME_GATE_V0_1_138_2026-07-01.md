# Workflow Attestation: Release Runtime Gate v0.1.138

## Scope

- Workline: remote release/runtime gate.
- Version: `0.1.138`.
- Artifact: local release tarball uploaded to the approved SSH stand.
- Not claimed: GitHub Latest proof, full MVP production PASS, full two-host
  transparent traffic PASS, sealed transit datapath PASS, long-run PASS.

## Council Summary

- Architect: use one chain of custody from built artifact to installed runtime;
  do not reuse `v0.1.137` proof.
- Senior developer: verify the installed release binaries, not a target-side
  `cargo` build.
- Tester: require independent fields for install/update/lifecycle/rebind/
  rollback/diagnostics instead of one green smoke.
- Security: public evidence must be redacted and must not contain stand host,
  user, path, port, password, token, raw logs, or transit payload.
- DevOps/release: classify this as uploaded-tarball SSH proof, not GitHub
  Latest proof.
- Critic-skeptic: do not promote lab/source checks to real-world PASS; record
  proof boundaries explicitly.

## Interdisciplinary Checks Applied

- Aviation safety case: checklist is not flight-ready until version, checksum,
  install, start, restart, stop, failure recovery and redaction are all checked.
- Medical chain of custody: the same release specimen must keep matching
  checksum and version through upload, install and installed runtime state.
- Financial audit separation: private operator notes and public proof are
  separated; public proof contains only redacted booleans/markers.
- Industrial lockout/tagout: rollback is accepted only after a negative path
  applies a test network change and then proves the test route/interface are
  gone.

## Local Artifact Evidence

```text
release_version=0.1.138
artifact_path=target/chimera-pq-release.tar.gz
artifact_sha256=c91d054cde1b87447f4167f836a3b731cb9a173c122dcc67c106d1f2c53fa099
local_checksum_file_ok=true
```

Evidence command:

```text
cd target && sha256sum -c chimera-pq-release.tar.gz.sha256
```

Observed result:

```text
chimera-pq-release.tar.gz: OK
```

## Remote Stand Evidence

Public evidence is redacted to allowed fields only:

```text
remote_stand_used=true
ssh_ok=true
upload_ok=true
upload_checksum_ok=true
release_version=0.1.138
artifact_checksum_ok=true
install_ok=true
install_without_cargo_ok=true
no_cargo_called=true
version_ok=true
checksum_ok=true
installed_bundle_sha_ok=true
launcher_ok=true
update_source_installed_bundle=true
update_ok=true
update_without_cargo_ok=true
post_version_ok=true
post_checksum_ok=true
failed_update_rollback_ok=true
start_ok=true
status_after_start_ok=true
restart_ok=true
status_after_restart_ok=true
stop_ok=true
status_after_stop_ok=true
doctor_ok=false
doctor_reason=client_endpoint_unconfigured
doctor_fail_reason_expected=true
doctor_network_state_not_modified=true
logs_ok=true
diagnostics_redacted_ok=true
logs_secret_marker_absent=true
logs_ipv4_absent=true
logs_path_absent=true
rebind_ok=true
reconnect_ok=true
old_endpoint_closed_ok=true
rollback_mode=remote_host_installed_release_binary_test_route
tun_probe_ok=true
rollback_apply_ok=true
rollback_state_modified_ok=true
rollback_route_present_ok=true
rollback_recover_ok=true
rollback_network_clean=true
rollback_emergency_cleanup_ok=true
rollback_ok=true
```

## What The Rebind/Reconnect Proof Means

- Installed `chimera-bootstrap` from release `0.1.138` served the release update
  endpoint on an OS-selected port.
- A second serve on another OS-selected port rewrote private peer-update state
  with a higher `endpoint_generation`.
- The old endpoint was stopped and became unreachable.
- The new endpoint remained reachable.

Boundary:

- This proves remote runtime bind/rebind and reconnect to the fresh update
  endpoint.
- This does not by itself prove a full two-host public discovery propagation or
  full user traffic datapath PASS.

## What The Rollback Proof Means

- Installed `chimera-cli` from release `0.1.138` applied a temporary test TUN
  and route on the SSH stand.
- `rollback recover` removed the state file, route rule, route table entry and
  test TUN.
- Emergency cleanup found no remaining test route/interface.

Boundary:

- This is a real SSH-stand rollback proof for a controlled test route/interface.
- It is not a full long-running production crash/traffic rollback proof.

## Diagnostics Boundary

- `doctor_ok=false` is expected in this stand configuration because the client
  endpoint is not configured.
- The doctor artifact remained fail-closed and reported
  `network_state=not_modified`.
- Redaction checks passed for public diagnostic output.

## Status

Status: partial for full MVP, PASS for this uploaded-tarball SSH release/runtime
slice.

Closed in this slice:

- release artifact exists and checksum matches;
- install/update without target-side `cargo`;
- version/checksum/launcher verification;
- start/status/restart/stop lifecycle;
- rebind/reconnect of release update endpoint;
- controlled rollback on the SSH stand;
- redacted diagnostics.

Still not closed:

- GitHub Latest proof for `v0.1.138`;
- full two-host transparent app traffic proof;
- full peer-discovery propagation proof with public non-loopback endpoint;
- sealed transit datapath real-world proof;
- long-run soak/performance proof.
