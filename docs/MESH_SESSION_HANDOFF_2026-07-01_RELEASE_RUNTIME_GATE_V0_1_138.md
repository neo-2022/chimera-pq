# CHIMERA Mesh Session Handoff: Release Runtime Gate v0.1.138

## Saved At

- Timestamp: 2026-07-01

## Active Objective

- Finish the user goal: release file without target-side `cargo`, install/update
  on the approved SSH stand, lifecycle checks, rebind/reconnect, rollback and
  redacted diagnostics.

## What Was Done

- Verified local release artifact `0.1.138`.
- Uploaded the release tarball and checksum to the approved SSH stand.
- Installed the release from the uploaded tarball with a fake `cargo` trap in
  `PATH`.
- Verified installed version, checksum, launcher and installed bundle SHA.
- Reinstalled/updated from the installed bundle and verified version/checksum
  stayed correct without calling `cargo`.
- Verified failed corrupt update does not replace the installed release.
- Verified start/status/restart/status/stop/status lifecycle.
- Verified doctor fails closed for the expected unconfigured endpoint reason and
  keeps network state unmodified.
- Verified logs/diagnostics redaction for secrets, IPv4 literals and private
  paths.
- Verified release update endpoint rebind/reconnect through installed
  `chimera-bootstrap`.
- Verified controlled rollback on the SSH stand using installed `chimera-cli`
  and a temporary test TUN/route.

## Evidence

- `docs/WORKFLOW_ATTESTATION_RELEASE_RUNTIME_GATE_V0_1_138_2026-07-01.md`

## Key Redacted Fields

```text
remote_stand_used=true
ssh_ok=true
release_version=0.1.138
artifact_checksum_ok=true
install_without_cargo_ok=true
update_without_cargo_ok=true
start_ok=true
restart_ok=true
stop_ok=true
rebind_ok=true
reconnect_ok=true
rollback_ok=true
diagnostics_redacted_ok=true
doctor_ok=false
doctor_reason=client_endpoint_unconfigured
```

## Truth Boundary

- This is uploaded-tarball SSH proof, not GitHub Latest proof.
- Full MVP/prod PASS is not claimed.
- Full two-host transparent user traffic is not closed.
- Full public discovery propagation with non-loopback endpoint is not closed.
- Sealed transit datapath real-world proof is not closed.
- Long-run soak/performance proof is not closed.

## Next Step

- Run the local safe guards for this docs/proof update.
- Scan the new proof for stand literals/secrets.
- Commit the proof docs if guards pass and git tree contains only intended
  documentation changes.
