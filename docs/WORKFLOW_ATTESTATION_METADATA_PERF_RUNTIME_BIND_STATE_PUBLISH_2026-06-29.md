# Workflow Attestation: Runtime Bind State Publish

## Scope

- Date: 2026-06-29
- Slice: runtime/private peer update state publish after listener bind.
- Status: lab/unit evidence only.
- Network state: not modified.
- Transit payload policy: opaque/sealed payload untouched.

## Implemented

- Strengthened existing peer update private-state parser to fail closed on
  malformed JSON and invalid stored metadata instead of silently treating it as
  missing state.
- Preserved monotonic `endpoint_generation` semantics:
  - first valid publish starts at generation `1`;
  - fresh identical publish remains no-op with the same generation;
  - stale identical publish refreshes epoch without incrementing generation;
  - changed endpoint/update URL increments generation exactly once;
  - generation exhaustion fails closed instead of wrapping or saturating into a
    misleading value.
- Added file-level fail-closed tests so malformed or zero-generation existing
  state is not rewritten and does not reset generation.
- Added parent-directory permission check for private state files on Unix:
  world-writable parent directories are rejected before state write.
- Preserved existing OS-selected port behavior: listener bind to `:0` records
  the actual bound port in private state and derives the update bootstrap URL
  from that bound port.

## Role Council Summary

- Architect: require actual bound endpoint from listener state, not requested
  config; publish only after successful bind.
- Senior developer: keep generation/state logic in `peer_update` helpers, not in
  CLI/discovery.
- Tester: require no-op, changed endpoint, malformed state, zero generation,
  private permissions and OS-selected port evidence.
- Security: require fail-closed parser, private file permissions and no payload
  involvement.
- DevOps/release: lab-only; avoid stand hardcode and no Real-World/SSH claims.
- Critic: do not call automatic rebind PASS without downstream runtime/reconnect
  evidence.

## Validation

PASS:

- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `cargo check -q -p chimera-bootstrap`
- `cargo check -q -p chimera-cli`
- `cargo check -q -p chimera-mesh`
- `cargo test -q -p chimera-bootstrap serve_state_publish -- --nocapture`
- `cargo test -q -p chimera-bootstrap state_file_ -- --nocapture`
- `cargo test -q -p chimera-bootstrap peer_release_state_file_records_os_selected_update_url -- --nocapture`
- `cargo test -q -p chimera-bootstrap`
- `cargo test -q -p chimera-cli tests_nodes_runtime_state -- --nocapture`
- `cargo test -q -p chimera-cli tests_nodes_inventory -- --nocapture`
- `cargo test -q -p chimera-mesh tests_discovery_merge -- --nocapture`
- `cargo clippy -q -p chimera-bootstrap --all-targets -- -D warnings`
- `cargo clippy -q -p chimera-cli --all-targets -- -D warnings`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `just rust-no-hardcode-guard-selfcheck`
- `just release-pack-schema-guard-selfcheck`
- `just ship-structure-guard-selfcheck`
- `./scripts/release_pack_schema_guard.sh`
- `./scripts/ship_structure_guard.sh`
- `git diff --check -- <touched files>`

Observed broad checks:

- `chimera-bootstrap`: 34 tests passed.
- `tests_nodes_runtime_state`: 33 tests passed.
- `tests_nodes_inventory`: 25 tests passed.
- `tests_discovery_merge`: 19 tests passed.

## Not Claimed

- Runtime reconnect PASS is not claimed.
- SSH stand evidence is not claimed.
- Real-World PASS is not claimed.
- One-command install/update PASS is not claimed.
- Broad datapath performance PASS is not claimed.
