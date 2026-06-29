# CHIMERA Mesh Session Handoff: Peer Update State Publish Generation

## Saved At

- Timestamp: 2026-06-28

## Active Objective

- Keep speeding and hardening metadata/control paths that help nodes find peers,
  choose paths, rebuild lane/binding/route state, publish state, and avoid
  wasted CPU/RAM.
- Keep sealed transit payload opaque and untouched.

## What Was Done

- Added `endpoint_generation` to peer-update runtime/private state.
- Made fresh no-op peer-update state publish skip rewrites only when the current
  state is unchanged and already generation-tagged.
- Made fresh legacy state upgrade to the generation contract instead of being
  treated as a no-op.
- Kept no-op state files private (`0600` on Unix).
- Made CLI advertise state validation reject zero endpoint generation when the
  generation field is present.
- Added a CLI contract test proving advertise publishes endpoint from runtime
  state and update URL from update state together with a non-zero OS-selected
  test port.
- Removed existing stand-specific device wording from product docs so the
  no-hardcode guard passes.

## Validation

PASS:

- `cargo fmt --all -- --check`
- `cargo check -q -p chimera-bootstrap`
- `cargo check -q -p chimera-cli`
- `cargo test -q -p chimera-bootstrap peer_update -- --nocapture`
- `cargo test -q -p chimera-cli tests_nodes_runtime_state -- --nocapture`
- `cargo test -q -p chimera-cli advertise_state -- --nocapture`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `cargo clippy -q -p chimera-bootstrap --all-targets -- -D warnings`
- `cargo clippy -q -p chimera-cli --all-targets -- -D warnings`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`
- `just rust-no-hardcode-guard-selfcheck`
- `git diff --check` for touched code/docs files.

Current metadata snapshot:

- `lane_document_render_parse_ops_per_sec=3066`
- `lane_document_render_parse_p95_ns=335956`
- `network_state=not_modified`
- `transit_payload_policy=opaque_sealed_payload_untouched`

## Not Closed

- Real-world runtime rebind/reconnect was not checked in this slice.
- Broad WEAVE datapath performance is not closed.
- SSH stand evidence remains required before any Real-World PASS claim.

## Next Step

- Continue with measured metadata/control paths.
- Good next slice: add a direct metadata-perf metric for peer-update state
  publish no-op vs changed-generation publish, or wire generation into the
  planner/control-plane reload path if a current hot path consumes update
  snapshots.
