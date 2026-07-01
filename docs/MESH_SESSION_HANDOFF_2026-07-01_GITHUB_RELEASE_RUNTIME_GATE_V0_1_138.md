# CHIMERA Mesh Session Handoff: GitHub Release Runtime Gate v0.1.138

## Saved At

- Timestamp: 2026-07-01

## Active Objective

- Continue from the completed GitHub Latest release/runtime slice.
- Next project work must not treat this as full MVP/prod readiness.

## What Was Done

- Re-read required project rules and latest prior handoff.
- Ran the required real council for the current release/runtime workline:
  architect, senior developer, tester, security engineer, DevOps/release
  engineer and critic-skeptic.
- Council decision: GO only for the narrow GitHub Latest -> approved SSH stand
  proof; NO-GO for full MVP/prod PASS.
- Verified remote `main` and tag `v0.1.138` point to the same release commit.
- Verified GitHub Latest is `v0.1.138`, not draft, not prerelease.
- Verified GitHub Latest assets are exactly:
  - `chimera.sh`;
  - `chimera-pq-release.tar.gz`;
  - `chimera-pq-release.tar.gz.sha256`.
- Verified GitHub Latest checksum and bootstrap version.
- Ran approved SSH-stand proof using GitHub one-command path only.
- Proved install from GitHub Latest without target-side `cargo`.
- Proved update from previous GitHub release to GitHub Latest without
  target-side `cargo`.
- Verified installed version and bundle checksum match the GitHub Latest asset.
- Verified lifecycle: start, status, restart, status, stop, status.
- Verified release update helper rebind/reconnect and old endpoint closure.
- Verified controlled rollback on the SSH stand using a temporary test
  route/interface.
- Verified fail-closed doctor result for the expected unconfigured endpoint
  reason.
- Verified redacted diagnostics markers.

## Evidence

- `docs/WORKFLOW_ATTESTATION_GITHUB_RELEASE_RUNTIME_GATE_V0_1_138_2026-07-01.md`

## Key Redacted Fields

```text
remote_stand_used=true
ssh_ok=true
github_latest_ok=true
github_one_command_install_ok=true
github_one_command_update_ok=true
install_without_cargo_ok=true
update_without_cargo_ok=true
no_cargo_called=true
version_ok=true
checksum_ok=true
installed_checksum_matches_github_asset=true
start_ok=true
status_after_start_ok=true
restart_ok=true
status_after_restart_ok=true
stop_ok=true
status_after_stop_ok=true
rebind_ok=true
reconnect_ok=true
old_endpoint_closed_ok=true
rollback_ok=true
rollback_network_clean=true
doctor_fail_closed_ok=true
doctor_reason=client_endpoint_unconfigured
doctor_network_state_not_modified=true
diagnostics_redacted_ok=true
logs_secret_marker_absent=true
logs_ipv4_absent=true
logs_path_absent=true
raw_payload_absent=true
proof_failures_present=false
```

## Truth Boundary

- PASS is only for the GitHub Latest SSH release/runtime slice.
- Full MVP/prod PASS is not claimed.
- Full two-host transparent user traffic is not closed.
- Full public discovery propagation with non-loopback endpoint is not closed.
- Sealed transit datapath real-world proof is not closed.
- Long-run soak/performance proof is not closed.
- Old uploaded-tarball proof remains historical evidence only and must not be
  reused as GitHub Latest proof.

## Next Step

- Run local safe guards and scans for the new docs.
- Commit the new evidence docs if guards pass and no secret/stand literal is
  found.
- Then choose the next MVP workline from the remaining unclosed gaps, not from
  post-MVP research features.
