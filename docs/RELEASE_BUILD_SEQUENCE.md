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
9. After the GitHub release is live, install that exact release on the
   notebook/VPS mirror nodes and publish them with
   `chimera-bootstrap serve-release --root "${CHIMERA_HOME:-$HOME/.local/share/chimera}" --listen 0.0.0.0:18179 --base-url http://node.example:18179`
   or an equivalent trusted base URL for the stand.
10. Verify the peer-update fallback contract remains update-only:
   - `chimera-sh` checks GitHub Latest first;
   - peer fallback is tried after GitHub Latest if it is unreachable, or if it
     is valid but not newer; it is not tried after invalid GitHub metadata/
     checksum/source;
   - configured peer URLs are tried only as fallback for already installed
     CHIMERA;
   - `chimera-sh -connect <peer>` uses only that selected peer's
     `update_bootstrap_url` as peer fallback, not the general peer list;
   - the peer path reads `/metadata.json`, verifies `kind`, `status`,
     same-origin archive/checksum URLs and `sha256`, and does not execute the
     peer bootstrap script as a trust source;
   - the peer path verifies the release checksum before extraction and matches
     metadata `sha256` to the checksum file;
   - if no trusted update source is reachable, CHIMERA keeps the installed
     version and emits `chimera_update=unavailable`;
   - peer update evidence is not used as first-install stand proof.
11. Verify the start contract before release:
   - `chimera-sh -start` prepares user-cache log targets before the systemd
     user start path;
   - `chimera-sh -start` returns non-zero if either node or transparent runtime
     service fails its active check;
   - false `start_status=ok` is a release-blocking regression.
12. Smoke-test the install flow on the notebook/VPS only through the GitHub
   one-command bootstrap.
   - the command must be wrapped with `bash -o pipefail -c`
   - the outer bootstrap download must use `curl --disable -fsSL --retry 3
     --connect-timeout 10 --max-time 60`
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
