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
   external proof-node install path must not require `cargo`.
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
9. After the GitHub release is live, refresh the public Gitvers bootstrap
   mirror with the same `chimera.sh`, archive and checksum contract. Do not
   publish a Gitvers mirror that points at a different version or checksum than
   GitHub Latest.
   - preferred operator path:
     `CHIMERA_GITVERS_TOKEN=... bash scripts/sync_gitvers_release.sh`
10. After GitHub and Gitvers are aligned, install that exact release on the
   trusted peer mirror nodes and publish them with
   `chimera-bootstrap serve-release --root "${CHIMERA_HOME:-$HOME/.local/share/chimera}" --listen 0.0.0.0:0 --base-url https://node.example --state-file "${XDG_CACHE_HOME:-$HOME/.cache}/chimera/peer-update.state.json"`
   or an equivalent trusted base URL for remote proof.
11. Verify the source-order contract:
   - `chimera-sh` checks GitHub Latest first;
   - a source becomes authoritative only after CHIMERA verifies a release
     tuple: `version + archive checksum`;
   - Gitvers is tried after GitHub only when GitHub is unreachable before a
     verified tuple exists;
   - peer fallback is tried only after GitHub and Gitvers when those higher
     trust sources are unreachable before a verified tuple exists;
   - peer fallback is not tried after invalid GitHub or invalid Gitvers
     metadata/checksum/source;
   - once GitHub or Gitvers yields a verified current or stale tuple, CHIMERA
     stops the search, emits `chimera_update=no_newer_release`, and does not
     let a lower-trust source outrun that verified result in the same round;
   - if GitHub or Gitvers yielded a verified newer tuple but its archive
     delivery/install path was unavailable, a lower-trust mirror may be used
     only for the exact same `{version, sha256}` tuple;
   - a lower-trust source with a different version after a verified higher-
     trust tuple must block with `trusted_version_divergence`;
   - a lower-trust source with the same version but a different checksum after
     a verified higher-trust tuple must block with
     `trusted_checksum_divergence`;
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
   - the exact runtime matrix is documented in
     `docs/UPDATE_SOURCE_DECISION_MATRIX.md`;
   - peer update evidence is not used as first-install remote proof.
   - peer mirror serving should let CHIMERA choose a free port with
     `--listen ...:0` unless the operator has a specific reserved port.
   - `chimera mesh nodes advertise` must publish `update_bootstrap_url` from
     `--update-state-file`/`CHIMERA_PEER_UPDATE_STATE_FILE` so selected peers
     learn the current update endpoint without manual port entry.
12. Verify the start contract before release:
   - `chimera-sh -start` prepares user-cache log targets before the systemd
     user start path;
   - `chimera-sh -start` returns non-zero if either node or transparent runtime
     service fails its active check;
   - false `start_status=ok` is a release-blocking regression.
13. Smoke-test the install flow on external proof nodes only through the GitHub
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
  `cargo run` as external remote install proof
- do not extract any release archive before its checksum is verified
- do not treat peer-update fallback as GitHub first-install proof
