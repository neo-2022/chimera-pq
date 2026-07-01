# Workflow Attestation: GitHub Release Runtime Gate v0.1.138

## Scope

- Workline: GitHub Latest release/runtime gate.
- Version: `0.1.138`.
- Canonical delivery source: GitHub Latest one-command bootstrap.
- Runtime proof location: approved SSH stand, used only as an external test stand.
- Public evidence policy: redacted markers only.

Not claimed:

- full MVP production PASS;
- full two-host transparent traffic PASS;
- full public peer-discovery propagation PASS;
- sealed transit datapath real-world PASS;
- long-run soak/performance PASS.

## Council Summary

Real council roles used for this workline:

- Architect: GO only for the narrow GitHub Latest -> SSH stand -> runtime proof path.
- Senior developer: GO only if the target does not use `cargo` and the installed checksum matches GitHub asset checksum.
- Tester: NO-GO for closure until one-command GitHub install/update, lifecycle, rebind/reconnect, rollback and diagnostics are all proven.
- Security: GO only with redacted evidence; no stand host, user, path, port, token, password, raw log or payload may be published.
- DevOps/release: GO only through GitHub Latest assets, with failure handling and cleanup.
- Critic-skeptic: NO-GO for full gate until the old uploaded-tarball proof is replaced by this GitHub Latest SSH proof.

Council decision:

```text
go_for_next_step=true
pass_before_ssh_proof=false
pass_after_ssh_proof_scope=github_latest_release_runtime_slice_only
full_mvp_pass=false
prod_ready=false
```

## Interdisciplinary Checks Applied

- Aviation checklist: one green check is not enough; version, checksum,
  install, update, lifecycle, rollback and redaction each require separate
  evidence.
- Medical chain of custody: the same release specimen must be traceable from
  GitHub asset checksum to the installed runtime checksum.
- Financial audit separation: public proof contains only redacted markers;
  private operator data and raw logs are not published.
- Industrial lockout/tagout: rollback is accepted only after a controlled test
  network change is applied and then removed.
- Logistics: a release is not delivered when it exists at the sender; delivery
  is proven at the receiving stand.

Rejected analogies:

- local tarball proof equals GitHub Latest proof;
- published release equals runtime proof;
- checksum alone proves lifecycle behavior;
- guard PASS alone proves production readiness;
- one SSH stand PASS equals full MVP PASS.

## GitHub Latest Evidence

Redacted public evidence:

```text
remote_main_matches_release_commit=true
remote_tag_v0_1_138_matches_release_commit=true
github_latest_ok=true
github_latest_tag=v0.1.138
github_latest_draft=false
github_latest_prerelease=false
github_latest_assets_exact=true
github_latest_asset_chimera_sh_present=true
github_latest_asset_bundle_present=true
github_latest_asset_checksum_present=true
github_latest_checksum_ok=true
github_latest_bootstrap_version_ok=true
```

Evidence commands used:

```text
git ls-remote origin refs/heads/main refs/tags/v0.1.138
public GitHub releases/latest API
public GitHub Latest asset download
sha256sum -c chimera-pq-release.tar.gz.sha256
```

Observed release asset checksum:

```text
github_asset_sha256=956449dc0797b8a4a309bc52e08b18133776585ebebb76e4580719891b639bc5
```

## Remote Stand Evidence

The SSH command, host, user, temporary paths, ports and raw logs are intentionally
not published.

Redacted evidence:

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
rollback_apply_ok=true
rollback_state_modified_ok=true
rollback_route_present_ok=true
rollback_recover_ok=true
rollback_network_clean=true
rollback_emergency_cleanup_ok=true
rollback_ok=true
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

## What The Install/Update Proof Means

- The install source was GitHub Latest one-command bootstrap.
- The update proof used a previous GitHub release and then updated to GitHub
  Latest.
- A fake `cargo` trap was placed before system tools in `PATH`.
- The trap was not called during install or update.
- The installed version and installed bundle checksum matched the GitHub Latest
  asset metadata.

Boundary:

- This proves self-contained GitHub delivery for this release/runtime slice.
- This does not prove every possible distro, shell or desktop environment.

## What The Lifecycle Proof Means

- `start`, `status`, `restart`, `status`, `stop`, `status` were executed on the
  approved SSH stand.
- The node runtime reached a running state after start and restart.
- The node runtime reached a stopped state after stop.

Boundary:

- This is a release/runtime lifecycle proof on one external stand.
- It is not a long-run stability proof.

## What The Rebind/Reconnect Proof Means

- Installed `chimera-bootstrap` served the release update endpoint on an
  OS-selected local port.
- A second serve used another OS-selected port.
- The old endpoint was stopped and became unreachable.
- The new endpoint stayed reachable and served a bootstrap script for
  `0.1.138`.

Boundary:

- This proves bind/rebind and reconnect behavior for the release update helper.
- It does not prove full public discovery propagation.

## What The Rollback Proof Means

- Installed `chimera-cli` applied a temporary test TUN and policy route on the
  approved SSH stand.
- `rollback recover` removed the saved state and cleaned the test network
  objects.
- Emergency cleanup found no remaining test interface, rule or route.

Boundary:

- This proves controlled SSH-stand rollback for a temporary route/interface.
- It is not a full crash-recovery or long-running production traffic rollback
  proof.

## Diagnostics Boundary

- `doctor` failed closed for the expected unconfigured endpoint reason.
- The doctor report kept `network_state=not_modified`.
- Public diagnostic checks found no secret markers, IPv4 literals, private
  paths or raw payload markers in the redacted proof stream.

## Status

Status: PASS for the GitHub Latest SSH release/runtime slice.

Closed in this slice:

- GitHub Latest release points to `v0.1.138`;
- GitHub Latest assets and checksum verified;
- one-command GitHub install on the approved SSH stand;
- one-command GitHub update on the approved SSH stand;
- no target-side `cargo` used;
- installed checksum matches GitHub asset checksum;
- start/status/restart/status/stop/status lifecycle;
- rebind/reconnect of release update endpoint;
- controlled rollback on the SSH stand;
- fail-closed doctor result;
- redacted diagnostics proof.

Still not closed:

- full MVP production PASS;
- full two-host transparent app traffic proof;
- full peer-discovery propagation proof with public non-loopback endpoint;
- sealed transit datapath real-world proof;
- long-run soak/performance proof.
