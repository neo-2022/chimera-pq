# CHIMERA Mesh Session Handoff

## Saved At
- Timestamp: 2026-06-13 local session

## Active Objective
- Preserve GitHub Latest as the canonical first-install path, while adding an
  update-only peer fallback for already-installed CHIMERA when GitHub is
  unavailable.

## What Was Done
- Added peer bootstrap update fallback in `scripts/chimera-sh`:
  - GitHub Latest is tried first for update checks;
  - trusted peer bootstrap URLs can be supplied via env or file;
  - peer sources are normalized to `/chimera.sh`;
  - bootstrap metadata is read from peer bootstrap scripts before install;
  - checksum is verified before extraction; if the peer reports the same
    version but a different checksum, the source is treated as inconsistent and
    held.
- Added a trusted peer URL list example:
  - `configs/update_peer_bootstrap_urls.example.list`
- Tightened installer gate coverage:
  - `scripts/chimera_installer_gate.sh`
- Documented the fallback contract:
  - `docs/OPERATIONS.md`
  - `docs/RELEASE_BUILD_SEQUENCE.md`

## Validation
- PASS: `bash -n scripts/chimera-sh scripts/chimera_installer_gate.sh scripts/install_release.sh scripts/chimera.sh`
- PASS: `bash scripts/chimera_installer_gate.sh`
- PASS: `cargo test -q -p chimera-bootstrap`
- PASS: `cargo fmt --all -- --check`

## Known Open Items
- Real-world peer-update smoke on external proof nodes still needs SSH-run evidence.
- The peer trust boundary is operational/config-based; signed peer manifests are
  still a future hardening step.

## Safety
- No PC runtime/network settings were changed.
- No local CHIMERA start/stop was performed on the current PC.

## Next Step
- Publish a new GitHub Release, then test GitHub-first install and peer
  fallback on external proof nodes over SSH.
