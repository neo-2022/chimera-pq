# CHIMERA Release Runtime Gate v0.1.151

Date: 2026-07-04

## Status

Status: partial

This attestation closes the one-host published release/update slice for
`v0.1.151`. It does not close the full laptop + two VPS + optional PC mesh
stand goal.

## Scope

- Published release source: GitHub Release/Latest.
- Secondary public mirror: Gitvers.
- Proof host class: authorized SSH stand host.
- Local PC runtime/network: not used.
- Cargo on target host: not used.
- Manual tarball/scp alignment: not used.

## Release Provenance

- Signed commit: `17b5ac443fb26e25089c9bea145e6488c3b2f312`.
- Signed tag: `v0.1.151`.
- GitHub Actions release run: `28688314598`, conclusion `success`.
- GitHub latest: `v0.1.151`.
- Required assets present:
  - `chimera.sh`
  - `chimera-pq-release.tar.gz`
  - `chimera-pq-release.tar.gz.sha256`
- Public checksum: `789052d605b3fc4656c6e0ef59bf0c7c0ed69bd81b970522cfe57d6221e3c2d0`.

## Gitvers Mirror

- Gitvers sync: `gitvers_sync=ok sync_changed=true version=0.1.151 checksum_ok=true`.
- Gitvers public version: `0.1.151`.
- Gitvers public checksum matches GitHub:
  `789052d605b3fc4656c6e0ef59bf0c7c0ed69bd81b970522cfe57d6221e3c2d0`.
- Gitvers remains an operator-trusted public mirror with checksum integrity.
  It is not an independent cryptographic provenance authority until signed
  release manifests exist.

## Local Guards

- `cargo test -q -p chimera-cli guard_`: PASS.
- `cargo test -q -p chimera-cli guard_listen_default_uses_os_selected_port`: PASS.
- `cargo test -q -p chimera-carrier parse_node_options_defaults_ingress_listeners_to_auto_bind`: PASS.
- `bash scripts/chimera_update_contract_smoke.sh`: PASS.
- `bash scripts/chimera_installer_gate.sh`: PASS.
- `bash scripts/chimera_start_contract_smoke.sh`: PASS.
- `bash scripts/product_language_guard.sh`: PASS.
- `bash scripts/public_artifact_redaction_guard.sh`: PASS.
- `bash scripts/release_bundle_install_contract_smoke.sh <absolute archive> <absolute checksum>`:
  PASS, `version=0.1.151`, `install_without_cargo_ok=true`,
  `artifact_checksum_ok=true`, `installed_state_proof_ok=true`,
  `uninstall_cleanup_ok=true`.

Known local limitation:

- `cargo fmt --all --check` reports pre-existing formatting drift outside the
  changed file. The `v0.1.151` changed file diff itself has `git diff --check`
  PASS.

## Remote Published-Source Proof

On the authorized SSH proof host:

- Previous installed release stopped cleanly.
- Update-first start upgraded `0.1.150 -> 0.1.151` from GitHub.
- Installed version after update: `0.1.151`.
- Installed checksum after update:
  `789052d605b3fc4656c6e0ef59bf0c7c0ed69bd81b970522cfe57d6221e3c2d0`.
- `-uninstall`: PASS.
- Cleanup after uninstall:
  - launcher absent: PASS.
  - user unit inactive: PASS.
- GitHub one-command install: PASS.
- GitHub installed version: `0.1.151`.
- GitHub installed checksum:
  `789052d605b3fc4656c6e0ef59bf0c7c0ed69bd81b970522cfe57d6221e3c2d0`.
- `-start`: PASS.
- `-status`: PASS.
- `-restart`: PASS.
- `-stop`: PASS.
- Final user unit inactive: PASS.

## Auto-Port Proof

- Installed runtime peer listen config uses auto bind.
- Runtime resolved peer port was dynamically selected by the OS.
- Runtime resolved peer port was not `8443`.
- `mesh nodes guard-listen` default was changed from a fixed `8443` bind to an
  OS-selected bind.
- Release binary proof for `guard-listen`: output included `bind=<redacted>:0`
  and a non-zero `resolved_bind=<redacted>`.

## Gitvers Fallback Proof

On the authorized SSH proof host:

- Gitvers one-command install: PASS.
- Gitvers installed version: `0.1.151`.
- Gitvers installed checksum:
  `789052d605b3fc4656c6e0ef59bf0c7c0ed69bd81b970522cfe57d6221e3c2d0`.
- Start with GitHub intentionally unreachable: PASS.
- Fallback used Gitvers: PASS.
- Fallback did not use peer: PASS.
- Stop after fallback start: PASS.
- Final user unit inactive: PASS.

## Stand Availability

- One SSH proof host: `ssh_ok=true`.
- First candidate from private SSH note matches the same proof host by machine
  hash: not a second VPS.
- Second candidate from private SSH note: `tcp22=unreachable`.
- Laptop SSH access: not found in the checked local SSH config/operator notes.
- Current PC: not used as runtime node; Happ/VS Code network was not touched.

## Not Closed

- Full laptop + two VPS mesh proof is not closed.
- Three-node transit/repath/failure proof is not closed.
- Peer update fallback from another live CHIMERA node is not physically proven
  in this slice.
- Real rebind continuity across multiple live nodes is not physically proven in
  this slice.
- Signed release manifests for Gitvers/peer provenance are not implemented.

## Truth Boundary

This attestation proves the published-source release/update/install/lifecycle
slice on one authorized SSH proof host. It does not prove production readiness
or the full multi-node mesh goal.
