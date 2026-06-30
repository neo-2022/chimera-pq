# CHIMERA Mesh Session Handoff: Metadata Allocation Batch

## Saved At

- Timestamp: 2026-06-30

## Active Objective

- Speed metadata/control paths that help mesh nodes find peers, choose paths,
  rebuild lane/binding/route state, publish state and avoid wasted CPU/RAM.
- Keep transit payload opaque/sealed and untouched.

## What Was Done

- Replaced repeated discovery/state-publish source allocation with
  `remember_source`.
- Removed borrowed-batch `node_id` clones in peer performance updates.
- Made small mesh metadata enums `Copy` and removed clone calls in mesh,
  carrier and CLI explain paths.
- Removed temporary vectors in active lane planning and live lane selection.
- Changed lane registration constructors/parsers/renderers to borrow input and
  copy only after validation.
- Reused the initial live lane document through `Arc` instead of deep cloning
  document/registration state.
- Built redacted diagnostic CSV labels directly without temporary `Vec<String>`.
- Added regression tests for invalid discovery atomicity, repeated
  state-publish source count, and mixed-order lane snapshot parsing.

## Validation

PASS on the final diff:

- `cargo fmt --all -- --check`
- `cargo check -q --workspace --all-targets`
- `cargo clippy -q --workspace --all-targets -- -D warnings`
- `cargo test -q --workspace --all-targets`
- targeted mesh tests: `peer_performance`, `connect_probe`,
  `published_endpoint`, `merge_discovery`, `rebuild_trigger`,
  `rebuild_control`, `redaction`
- targeted carrier tests: `lane_document`, `live_lane_selection`,
  `live_bindings`, `transit`
- `cargo run -q -p chimera-lab --bin chimera-lab -- metadata-perf-smoke --iterations 20000 --json`
- `just metadata-perf-smoke-selfcheck`
- `just rust-no-hardcode-guard-selfcheck`
- `just ship-structure-guard-selfcheck`
- `just release-pack-schema-guard-selfcheck`
- `git diff --check`

## Current Metadata Snapshot

- `discovery_update_noop_ops_per_sec=129409`
- `peer_update_state_publish_noop_ops_per_sec=13289072`
- `fast_sorted_ops_per_sec=1133246`
- `lane_document_plan_snapshot_borrowed_ops_per_sec=66110020`
- `lane_document_render_parse_ops_per_sec=3635`
- `live_binding_reload_index_ops_per_sec=202910`
- `network_state=not_modified`
- `transit_payload_policy=opaque_sealed_payload_untouched`

## Interpretation

- This is Lab/Metadata PASS for allocation/copy reduction across several
  metadata/control paths.
- It is not a prod-ready, real-runtime, datapath or transparent app PASS.
- Local CHIMERA runtime was not started and local PC network state was not
  changed.
- Transit payload remained opaque/sealed and untouched.

## Next Step

- Best architectural next step remains the remote release/runtime gate:
  release artifact, install/update without `cargo`, start/stop/restart,
  reconnect/rebind, rollback and redacted diagnostics on the SSH stand.
- If continuing local metadata work first, use fresh perf evidence and avoid
  broad API rewrites unless a hotspot is measured.
