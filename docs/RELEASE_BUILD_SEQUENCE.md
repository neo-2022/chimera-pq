# CHIMERA Release Build Sequence

Use this sequence for every bundle or bootstrap update.

1. Start from the current `main` source state and identify the exact runtime fix.
2. Do not package build artifacts or local-only folders.
   - exclude `target/`
   - exclude `state/`
   - exclude `.amai/`
   - exclude `.codex/`
   - exclude `fuzz/target/`
   - exclude `WEAVE_brand/`
   - exclude `.git/`
   - keep `bin/` because it carries the shipped executables
3. Build the release archive from the source tree root, then inspect its size.
   The release build must compile ready Linux binaries into `bin/`; the
   notebook/VPS install path must not require `cargo`.
4. Fail the release if the archive size is clearly wrong or contains build output.
5. Generate checksum files and verify them against the archive.
6. Update the public GitHub release/bootstrap path:
   - version string
   - archive URL
   - README release note
7. Publish source changes first, then publish the GitHub Release.
   The CI release workflow is allowed to publish only from an existing tag.
8. Verify that `releases/latest` points to the new version and that the release
   contains exactly the public bootstrap, bundle and checksum.
   Required Latest assets:
   - `chimera.sh`
   - `chimera-pq-release.tar.gz`
   - `chimera-pq-release.tar.gz.sha256`
9. Verify the peer-update fallback contract remains update-only:
   - `chimera-sh` checks GitHub Latest first;
   - configured peer bootstrap URLs are tried only as fallback for already
     installed CHIMERA;
   - the peer path still verifies the release checksum before extraction;
   - if no trusted update source is reachable, CHIMERA keeps the installed
     version and emits `chimera_update=unavailable`;
   - peer update evidence is not used as first-install stand proof.
10. Verify the start contract before release:
   - `chimera-sh -start` prepares user-cache log targets before the systemd
     user start path;
   - `chimera-sh -start` returns non-zero if either node or transparent runtime
     service fails its active check;
   - false `start_status=ok` is a release-blocking regression.
11. Smoke-test the install flow on the notebook/VPS only through the GitHub
   one-command bootstrap.
   - first install must not loop on self-update
   - start/status must work
   - if the smoke fails, fix the root cause and rebuild from step 2

Non-goals:

- do not package a release from `target/`
- do not ship `WEAVE_brand/`
- do not treat a first-install self-update loop as acceptable
- do not use `rsync`, `scp`, local tarballs, `git clone`, `cargo build`, or
  `cargo run` as laptop/VPS stand install proof
- do not extract any release archive before its checksum is verified
- do not treat peer-update fallback as GitHub first-install proof
