# CHIMERA Metadata Performance Attestation: Lane Document Render/Parse

## Scope

- Date: 2026-06-28
- Hot path: `lane_document_render_parse`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- Lane documents carry lane/binding/route metadata used by live publish/reload
  paths.
- The prior metadata smoke measured borrowed versus owned plan snapshot access,
  but did not directly measure render+parse cost.
- This slice adds a direct render/parse metric and removes allocation churn in
  the lane document metadata serializer/parser without changing the document
  format or transit payload handling.

## Council Result

- Architect: keep the slice in MVP control metadata and do not touch payload or
  datapath.
- Senior/performance: add a direct metric before relying on claims; remove
  per-row `format!`, temporary `Vec` fields, and row-buffer `join`.
- Tester: require round-trip, live binding, metadata JSON, and smoke checks.
- Security: preserve redaction and the `sealed_opaque_only` payload policy.
- DevOps: keep it as pure Rust metadata logic; no local runtime/network changes.
- Critic: do not claim a broad planner/datapath speedup and do not invent a
  before/after percentage without a previous direct metric.

## Change

- `crates/chimera-carrier/src/peer_egress/lane_document/format.rs`
  - adds direct plan comment builders;
  - adds bounded comma/tab split helpers.
- `crates/chimera-carrier/src/peer_egress/lane_document/registration.rs`
  - renders registration rows directly;
  - parses three-field registration rows without collecting a `Vec`.
- `crates/chimera-carrier/src/peer_egress/lane_document/document.rs`
  - renders plan-backed rows directly into the caller buffer;
  - parses row lines with borrowed slices and original line numbers, without
    copying rows into strings or joining row buffers.
- `crates/chimera-carrier/src/peer_egress/lane_document/snapshot_render.rs`
  - writes selected peer, explain, and carrier-binding tab comments directly
    instead of building temporary `String` arrays.
- `crates/chimera-carrier/src/peer_egress/lane_document/snapshot_parse.rs`
  - removes `format!`-built prefixes and uses bounded tab splitting.
- `crates/chimera-lab/src/metadata_perf.rs`
  - adds `lane_document_render_parse` to `metadata-perf-smoke`.
- `justfile`
  - adds the new metric to `metadata-perf-smoke-selfcheck`.
- `docs/PERFORMANCE.md`
  - records this slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo check -q -p chimera-carrier`
- `cargo check -q -p chimera-lab`
- `cargo test -q -p chimera-carrier lane_document -- --nocapture`
- `cargo test -q -p chimera-carrier live_bindings -- --nocapture`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `cargo clippy -q -p chimera-carrier --all-targets -- -D warnings`
- `cargo clippy -q -p chimera-lab --all-targets -- -D warnings`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`
- `git diff --check -- crates/chimera-carrier/src/peer_egress/lane_document/format.rs crates/chimera-carrier/src/peer_egress/lane_document/registration.rs crates/chimera-carrier/src/peer_egress/lane_document/document.rs crates/chimera-carrier/src/peer_egress/lane_document/snapshot_render.rs crates/chimera-carrier/src/peer_egress/lane_document/snapshot_parse.rs crates/chimera-lab/src/metadata_perf.rs justfile`

`just metadata-perf-smoke` output after this slice:

```json
{"status":"ok","kind":"metadata_perf_smoke","hot_paths":["multipath_flow_lane_selection","path_planner_candidate_snapshot","discovery_rebuild_fingerprint","lane_document_plan_snapshot_access","lane_document_render_parse","live_binding_reload_index","live_dps_plan_path_from_payload","status_explain"],"scope":"hot_metadata_only","transit_payload_policy":"opaque_sealed_payload_untouched","iterations":100000,"active_bindings":16,"path_planner_peer_count":64,"discovery_rebuild_peer_count":64,"lane_document_render_parse_iterations":10000,"lane_document_render_parse_ops_per_sec":2951,"lane_document_render_parse_p95_ns":356302,"network_state":"not_modified"}
```

## Interpretation

- The new smoke metric proves lane document render/parse is now tracked in the
  metadata perf suite.
- The code removes specific allocation sources in the measured path.
- No direct before/after percentage is claimed because the render/parse metric
  did not exist before this slice.
- Existing round-trip and live-binding tests passed, so the format and reload
  behavior are preserved for this slice.

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- A historical before/after percentage for render/parse is not available from
  the pre-slice metric set.

## Rollback

- Restore the previous row renderers using `format!`.
- Restore parser row collection through temporary `Vec`/`join` and collected
  comma/tab fields.
- Remove `lane_document_render_parse` from `metadata-perf-smoke`,
  `metadata-perf-smoke-selfcheck`, and `docs/PERFORMANCE.md`.
