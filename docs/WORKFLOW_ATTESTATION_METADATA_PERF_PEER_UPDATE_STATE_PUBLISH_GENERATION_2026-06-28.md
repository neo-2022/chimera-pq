# Workflow Attestation: Metadata Perf Peer Update State Publish Generation

Status: lab_control_plane_pass
Date: 2026-06-28

## Scope

- Improve the metadata/control path that lets nodes publish current update
  endpoint state after auto bind/rebind.
- Keep the change narrow: no local product runtime start, no route/DNS/firewall
  mutation, no transit payload access.

## Council Decision

Six-role review converged on the same implementation slice:

- keep this in the private runtime/control-plane state contract;
- add generation-based publish state for rebind visibility;
- avoid manual ports and stand hardcode;
- reject invalid generation state in advertise/control-plane reads;
- do not claim Real-World PASS from local unit tests.

## Changes

- `crates/chimera-bootstrap/src/peer_update/serve_state.rs`
  - writes `endpoint_generation`;
  - upgrades fresh legacy state that lacks generation;
  - skips fresh no-op rewrites only when advertisement data is unchanged and
    already generation-tagged;
  - keeps the state file private on no-op.
- `crates/chimera-bootstrap/src/peer_update/server_auto_port_tests.rs`
  - checks auto-selected update state includes generation `1`.
- `crates/chimera-cli/src/mesh_cli/nodes_cmd/advertise_state.rs`
  - accepts legacy state without generation for compatibility;
  - rejects `endpoint_generation=0` when generation is present.
- `crates/chimera-cli/src/mesh_cli/tests_nodes_runtime_state/state.rs`
  - adds a contract test proving advertise publishes endpoint from runtime
    state and update URL from peer-update state together with a non-zero
    OS-selected test port.

## Evidence

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
- `git diff --check` for the touched code/docs files.

Latest `metadata-perf-smoke` snapshot:

```json
{"status":"ok","kind":"metadata_perf_smoke","scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","lane_document_render_parse_ops_per_sec":3066,"lane_document_render_parse_p95_ns":335956,"network_state":"not_modified"}
```

## Interpretation

- This is a Lab/control-plane PASS for the peer-update state publish-generation
  slice.
- This reduces unnecessary private state rewrites for unchanged update endpoint
  metadata and gives rebind/update readers a monotonic generation contract.
- This does not claim broad datapath performance closure.

## Not Closed

- Real-World SSH stand proof was not run in this slice.
- Full WEAVE datapath throughput/load behavior is not closed.
- End-to-end runtime rebind/reconnect still needs remote stand evidence.

## Safety

- Transit payload remains opaque/sealed and untouched.
- Local product runtime was not started.
- Local DNS/routes/firewall/proxy/VPN were not changed.
- Stand details are not embedded in product code or docs.
