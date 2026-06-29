# Workflow Attestation: Metadata Perf Published Endpoint Consumption

## Status

Status: partial, lab/control-plane PASS only.

This slice proves that published endpoint state with `endpoint_generation` can
be consumed for existing peers without expanding the public discovery contract.
It does not claim runtime bind/rebind, SSH-stand, install/update, datapath, or
Real-World PASS.

## Scope

- Hot path: `published_endpoint_state_consumption_existing_peer`.
- Runtime layer touched: `chimera-mesh` control metadata and multipath rebuild
  invalidation.
- Public discovery contract: unchanged.
- Transit payload rule: opaque sealed payload untouched.
- Network state: not modified.

## Behavior Proven

- `MeshPublishedEndpointUpdate` validates node id, host:port endpoint,
  optional `update_bootstrap_url`, and non-zero `endpoint_generation`.
- Same generation plus same endpoint/update URL is a no-op and keeps pending
  rebuild clear.
- Newer generation for an existing peer updates the private endpoint state and
  marks `dirty_scope=peer_set` with exact `affected_peer_count`.
- Stale/lower generation is ignored deterministically and does not roll back
  the accepted endpoint.
- Same generation with different endpoint/update URL is rejected as a
  generation conflict.
- Invalid endpoint or zero generation is rejected before mutation, preserving
  the previous peer snapshot and pending rebuild state.
- Mixed batches count only changed existing peers; unknown peers do not inflate
  dirty metadata.
- Discovery records without generation no longer downgrade an endpoint accepted
  from published endpoint state; non-endpoint metadata can still update through
  the existing discovery policy.
- Rebuild fingerprints now include endpoint and endpoint-generation-sensitive
  metadata so endpoint changes can drive planner invalidation.
- Debug/rebuild signal output redacts raw endpoint, update URL and raw peer id.

## Role Council

- Architect: approved separate internal consumption path; required no public
  `MeshDiscoveryRecord` expansion and explicit downgrade protection after an
  accepted endpoint generation.
- Senior developer: required a small adapter near discovery/update logic and no
  runtime/network changes.
- Tester: required no-op, newer, stale, zero, invalid, mixed exact-count and
  redaction tests.
- Security: required generation monotonicity, conflict rejection, URL/endpoint
  validation, redaction, and no transit payload involvement.
- DevOps/release: classified this as lab-only; remote stand is not a gate for
  this slice.
- Critic: required proving the pending rebuild comes from actual applied state,
  not leftover signals or diagnostics-only output.

## Evidence

Commands passed:

```text
cargo fmt --all -- --check
cargo check -q -p chimera-mesh
cargo check -q -p chimera-lab
cargo test -q -p chimera-mesh tests_discovery_merge -- --nocapture
cargo test -q -p chimera-mesh tests_multipath_schedule::rebuild_trigger -- --nocapture
cargo test -q -p chimera-mesh tests_multipath_schedule::rebuild_control -- --nocapture
cargo test -q -p chimera-mesh
cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only -- --nocapture
cargo clippy -q -p chimera-mesh --all-targets -- -D warnings
cargo clippy -q -p chimera-lab --all-targets -- -D warnings
just metadata-perf-smoke
just metadata-perf-smoke-selfcheck
just rust-no-hardcode-guard-selfcheck
just release-pack-schema-guard-selfcheck
just ship-structure-guard-selfcheck
./scripts/release_pack_schema_guard.sh
./scripts/ship_structure_guard.sh
git diff --check -- <touched files>
```

Latest `just metadata-perf-smoke` excerpt:

```text
status=ok
kind=metadata_perf_smoke
scope=hot_metadata_only
transit_payload_policy=opaque_sealed_payload_untouched
discovery_rebuild_fingerprint_ops_per_sec=3168
discovery_rebuild_fingerprint_p95_ns=331524
discovery_update_noop_ops_per_sec=120184
discovery_update_noop_p95_ns=8535
peer_update_state_publish_noop_ops_per_sec=13001889
peer_update_state_publish_noop_p95_ns=116
peer_update_state_publish_changed_generation_ops_per_sec=115116
peer_update_state_publish_changed_generation_p95_ns=8998
network_state=not_modified
```

## Changed Files

- `crates/chimera-mesh/src/model.rs`
- `crates/chimera-mesh/src/model_debug.rs`
- `crates/chimera-mesh/src/lib.rs`
- `crates/chimera-mesh/src/runtime.rs`
- `crates/chimera-mesh/src/runtime/peer_endpoint_update.rs`
- `crates/chimera-mesh/src/runtime/peer_discovery_update.rs`
- `crates/chimera-mesh/src/runtime/auto_recovery/discovery.rs`
- `crates/chimera-mesh/src/runtime/multipath_rebuild_trigger.rs`
- `crates/chimera-mesh/src/runtime/multipath_rebuild_model.rs`
- `crates/chimera-mesh/src/tests_discovery_merge/mod.rs`
- `crates/chimera-mesh/src/tests_discovery_merge/published_endpoint_update.rs`

## Not Closed

- Runtime bind/rebind/reconnect is not checked.
- SSH stand evidence was not collected.
- Install/update one-command flow is not checked by this slice.
- Real-World PASS is not claimed.
- Broad WEAVE datapath performance is not closed.
- Selective/partial planner rebuild is still not implemented.

## Risks

- `endpoint_generation` and endpoint-change metadata are topology metadata and
  must remain aggregate/redacted in diagnostics.
- `update_bootstrap_url` is private control-plane state; it must not become
  public planner/explain output.
- Perf numbers are smoke evidence, not a stable benchmark promise.
