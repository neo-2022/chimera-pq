# Workflow Attestation: Metadata Perf Peer Update State Publish Metric

Status: lab_control_plane_pass
Date: 2026-06-28

## Scope

- Add a direct metadata/performance metric for peer-update state publish
  decisions.
- Measure no-op publish decisions separately from changed-generation publish
  decisions.
- Keep the measured path in-memory: no local CHIMERA runtime start, no socket
  bind, no SSH stand action, no route/DNS/firewall/proxy/VPN mutation.

## Council Decision

Six-role review converged on this slice with these constraints:

- extract a pure publish decision API from the peer-update file writer;
- keep filesystem permissions, temp-file write, rename, and `sync_all` in the
  writer layer only;
- measure the decision path in `chimera-lab metadata-perf-smoke`, not a binary
  process loop and not temp-file I/O;
- expose only the narrow bootstrap library API needed by the binary and lab;
- keep metrics redacted: counters/latencies/booleans only.

Rejected:

- local CHIMERA runtime checks on the PC;
- manual port selection as proof;
- payload/datapath inspection;
- raw endpoint, URL, checksum, token, secret, or stand values in metrics;
- Real-World PASS claims from lab-only evidence.

## Changes

- `crates/chimera-bootstrap/src/lib.rs`
  - adds a narrow library entrypoint for bootstrap code reuse.
- `crates/chimera-bootstrap/src/peer_update/serve_state_publish.rs`
  - adds `PeerUpdateStateAdvertisement`;
  - adds `decide_peer_update_state_publish`;
  - parses existing state once and returns `Noop` or `Changed` with the next
    endpoint generation and serialized body when needed.
- `crates/chimera-bootstrap/src/peer_update/serve_state.rs`
  - keeps the file writer as the only filesystem layer;
  - calls the shared decision API;
  - preserves private file permissions and atomic temp-file replacement.
- `crates/chimera-lab/src/metadata_perf.rs`
  - adds hot path `peer_update_state_publish_generation`;
  - adds no-op and changed-generation decision metrics;
  - keeps metadata-perf JSON redacted and metadata-only.
- `justfile`
  - makes the new JSON metric fields mandatory in
    `metadata-perf-smoke-selfcheck`.

## Evidence

PASS:

- `cargo fmt --all`
- `cargo check -q -p chimera-bootstrap`
- `cargo check -q -p chimera-lab`
- `cargo test -q -p chimera-bootstrap peer_update -- --nocapture`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only -- --nocapture`
- `cargo clippy -q -p chimera-bootstrap --all-targets -- -D warnings`
- `cargo clippy -q -p chimera-lab --all-targets -- -D warnings`
- `just metadata-perf-smoke-selfcheck`
- `just metadata-perf-smoke`
- `just rust-no-hardcode-guard-selfcheck`
- `just release-pack-schema-guard-selfcheck`
- `just ship-structure-guard-selfcheck`
- `git diff --check` for the touched code/docs files.

Latest `metadata-perf-smoke` snapshot:

```json
{"status":"ok","kind":"metadata_perf_smoke","hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint","lane_document_plan_snapshot_access","lane_document_render_parse","peer_update_state_publish_generation","live_binding_reload_index","live_dps_plan_path_from_payload","status_explain"],"scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","peer_update_state_publish_iterations":10000,"peer_update_state_publish_noop_ops_per_sec":13038745,"peer_update_state_publish_noop_p95_ns":76,"peer_update_state_publish_changed_generation_ops_per_sec":117124,"peer_update_state_publish_changed_generation_p95_ns":8671,"network_state":"not_modified"}
```

## Interpretation

- This is a Lab/control-plane PASS for direct peer-update state publish decision
  metrics.
- The no-op metric covers unchanged, fresh, generation-tagged update state.
- The changed-generation metric covers a metadata change that increments the
  endpoint generation and produces a new state body.
- This does not prove broad datapath performance, runtime reconnect, or remote
  stand behavior.

## Not Closed

- Planner/control-plane consumption of `endpoint_generation` is still a later
  slice.
- Real-World SSH stand proof was not run in this slice.
- End-to-end runtime bind/rebind/reconnect remains unverified here.

## Safety

- Transit payload remains opaque/sealed and untouched.
- Local product runtime was not started.
- Local DNS/routes/firewall/proxy/VPN were not changed.
- No stand host, user, path, port, token, or secret is embedded in product code
  or product docs.
