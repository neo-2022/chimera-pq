# CHIMERA Metadata Performance Attestation: Batch Allocation Pass

## Scope

- Date: 2026-06-30
- Workline: metadata/control performance
- Status: Lab/Metadata PASS
- Scope: discovery, state publish, path/rebuild metadata, lane/binding
  metadata, lane document parsing/rendering and diagnostics labels
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Why This Batch

The user rejected one-variable microsteps and asked for a broader architectural
pass over small, measurable CPU/RAM waste. This batch stays inside the MVP
mesh-node metadata/control path and avoids broad refactoring:

- do not change routing semantics;
- do not change crypto/session behavior;
- do not inspect, classify or log transit payload;
- remove only visible redundant copies, temporary vectors and avoidable string
  allocations.

## Changes

- Discovery/state publish:
  - added `MeshRuntime::remember_source`, so repeated source recording allocates
    only when the source is new;
  - reused it in discovery and published endpoint update paths;
  - kept validate-before-mutate behavior for invalid batches.
- Peer state publish:
  - changed peer performance validation to keep borrowed `node_id` references
    until mutation instead of cloning every id in the validated batch.
- Path/rebuild metadata:
  - made small metadata enums `Copy` and removed now-unneeded `.clone()` calls
    from path planning, rebuild bridge, lane binding/document and CLI explain
    output.
- Lane/binding metadata:
  - changed `active_lane_weights` and active lane construction to use slices
    instead of building a temporary `Vec<&MeshPeerState>`;
  - removed temporary `Vec` collection in weighted live lane selection and
    replaced it with iterator passes;
  - kept parity between full lane selection and fast binding selection.
- Lane document metadata:
  - changed `TransitLaneRegistration` constructors to accept borrowed strings
    and copy only once after validation;
  - render registration rows into the existing document buffer instead of
    rendering a temporary string and appending it;
  - build snapshot lanes and derived counts/sums in one pass;
  - keep parsed carrier bindings ordered by the existing `BTreeMap` lane key,
    with a regression test for mixed input order.
- Live binding metadata:
  - store the initial live lane document as one `Arc<TransitLaneDocument>` and
    reuse it for snapshot and worker setup instead of deep cloning the document
    and registrations.
- Diagnostics labels:
  - build redacted peer/endpoint CSV labels directly into a `String` instead of
    collecting temporary string vectors and joining them.

## Council Result

- Architect: accepted the package as targeted allocation/copy removal, not a
  broad refactor, but required tests for carrier binding ordering and repeated
  state-publish source count.
- Senior Rust reviewer: found and confirmed mechanical compile risks; fixes
  were applied and verified by `cargo check` and `clippy`.
- QA/security reviewer: final review was requested separately; this document
  records command evidence and does not claim real-runtime PASS.
- Main arbitration: broader fingerprint rewrites and status renderer rewrites
  were rejected for this batch because they would be larger API refactors
  without enough evidence for this turn.

## Evidence

Targeted and full commands passed on the final diff:

```text
cargo fmt --all -- --check
cargo check -q --workspace --all-targets
cargo clippy -q --workspace --all-targets -- -D warnings
cargo test -q --workspace --all-targets
cargo test -q -p chimera-mesh peer_performance -- --nocapture
cargo test -q -p chimera-mesh connect_probe -- --nocapture
cargo test -q -p chimera-mesh published_endpoint -- --nocapture
cargo test -q -p chimera-mesh merge_discovery -- --nocapture
cargo test -q -p chimera-mesh rebuild_trigger -- --nocapture
cargo test -q -p chimera-mesh rebuild_control -- --nocapture
cargo test -q -p chimera-mesh redaction -- --nocapture
cargo test -q -p chimera-mesh invalid_discovery_record_after_valid_record_keeps_batch_atomic -- --nocapture
cargo test -q -p chimera-mesh published_endpoint_repeated_source_does_not_grow_source_count -- --nocapture
cargo test -q -p chimera-carrier lane_document -- --nocapture
cargo test -q -p chimera-carrier document_snapshot_parse_orders_mixed_carrier_binding_rows_by_lane_id -- --nocapture
cargo test -q -p chimera-carrier live_lane_selection -- --nocapture
cargo test -q -p chimera-carrier live_bindings -- --nocapture
cargo test -q -p chimera-carrier transit -- --nocapture
cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only -- --nocapture
cargo run -q -p chimera-lab --bin chimera-lab -- metadata-perf-smoke --iterations 20000 --json
just metadata-perf-smoke-selfcheck
just rust-no-hardcode-guard-selfcheck
just ship-structure-guard-selfcheck
just release-pack-schema-guard-selfcheck
git diff --check
```

Final `metadata-perf-smoke --iterations 20000 --json` snapshot:

```text
discovery_update_noop_ops_per_sec=129409
discovery_update_noop_p95_ns=7826
peer_update_state_publish_noop_ops_per_sec=13289072
peer_update_state_publish_noop_p95_ns=74
peer_update_state_publish_changed_generation_ops_per_sec=113527
peer_update_state_publish_changed_generation_p95_ns=9044
fast_sorted_ops_per_sec=1133246
fast_sorted_fallback_speedup_pct=612.48
lane_document_plan_snapshot_borrowed_ops_per_sec=66110020
lane_document_plan_snapshot_owned_ops_per_sec=131360
lane_document_render_parse_ops_per_sec=3635
live_binding_reload_index_ops_per_sec=202910
live_pending_rebuild_plan_path_ops_per_sec=5476
live_pending_rebuild_plan_core_ops_per_sec=12470
network_state=not_modified
transit_payload_policy=opaque_sealed_payload_untouched
```

## Interpretation

- This is a Lab/Metadata PASS for a batch allocation/copy reduction pass.
- The strongest measured improvement in this batch is avoiding repeated
  temporary structures and preserving fast metadata paths; the smoke is not a
  real-world network benchmark.
- The full workspace and targeted tests passed after the batch changes.
- Local CHIMERA runtime was not started and local PC network settings were not
  changed.

## Limits

- This is not prod-ready, release-ready or real-runtime PASS.
- It does not close remote install/update/start/stop/reconnect/rollback.
- It does not prove transparent app behavior.
- It intentionally does not rewrite discovery fingerprint, status explain or
  broader planning APIs.
