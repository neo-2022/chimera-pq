# CHIMERA Mesh Session Handoff

## Saved At

- Timestamp: 2026-06-26

## Active Objective

- Close the live launch/update contour for CHIMERA on the real stand.
- Keep the product path one-command, GitHub-first, peer-fallback-capable, and
  auto-port selecting.

## What Was Done

- Verified the canonical GitHub one-command install/update path on external
  stand side A for `v0.1.135`.
- Verified peer-fallback update on external stand side B when GitHub was blocked.
- Confirmed the peer mirror can front its public update origin with a relay
  while using an auto-selected internal listen port.
- Fixed the private-state validation so a relay-fronted public update origin is
  no longer rejected just because the listen bind and public origin differ.
- Added durable proof text in:
  - `docs/WORKFLOW_ATTESTATION_REAL_WORLD_RELEASE_UPDATE_V0_1_135_2026-06-26.md`
  - `docs/OPERATIONS.md`

## Validation

- PASS: `cargo fmt --all`
- PASS: `cargo test -q -p chimera-cli read_update_state_`
- PASS: `cargo test -q -p chimera-cli selected_update_bootstrap_url_`
- PASS: `cargo clippy -q -p chimera-cli --all-targets -- -D warnings`

## Not Closed

- Full WEAVE datapath and service-speed work remain separate from the release
  / update proof.
- Broad perf work still needs its own proof bundle.

## Next Step

- Continue with the service-data speed work after this release/update contour is
  treated as closed in the evidence trail.
