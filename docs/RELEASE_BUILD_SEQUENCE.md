# CHIMERA Release Build Sequence

Use this sequence for every bundle or bootstrap update.

1. Start from the current `main` source state and identify the exact runtime fix.
2. Do not package build artifacts or local-only folders.
   - exclude `target/`
   - exclude `bin/`
   - exclude `state/`
   - exclude `.amai/`
   - exclude `.codex/`
   - exclude `fuzz/target/`
   - exclude `WEAVE_brand/`
   - exclude `.git/`
3. Build the release archive from the source tree root, then inspect its size.
4. Fail the release if the archive size is clearly wrong or contains build output.
5. Generate checksum files and verify them against the archive.
6. Update the public bootstrap repo:
   - version string
   - archive URL
   - README release note
7. Publish source changes first, then publish the install/bootstrap repo.
8. Smoke-test the install flow on the notebook `new`.
   - first install must not loop on self-update
   - start/status must work
   - if the smoke fails, fix the root cause and rebuild from step 2

Non-goals:

- do not package a release from `target/`
- do not ship `WEAVE_brand/`
- do not treat a first-install self-update loop as acceptable

