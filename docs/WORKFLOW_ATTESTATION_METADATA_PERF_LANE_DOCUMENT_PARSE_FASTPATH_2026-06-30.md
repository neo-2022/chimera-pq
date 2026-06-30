# CHIMERA Metadata Performance Attestation: Lane Document Parse Fast Path

## Scope

- Date: 2026-06-30
- Workline: metadata/control performance
- Hot path: `lane_document_render_parse`
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Why This Slice

- Lane documents carry route/lane/binding metadata used by state publish, live
  reload, path choice and peer transit setup.
- The measured path was one of the slower metadata paths in
  `metadata-perf-smoke`.
- This slice removes repeated parser work and render allocation churn in that
  measured path without changing transit payload handling.

## Change

- `crates/chimera-carrier/src/peer_egress/lane_document/document.rs`
  - preallocates the lane document render buffer from a bounded estimate;
  - parses snapshot-backed document rows directly as 6-field rows;
  - keeps 3-field rows only for registration-only documents.
- `crates/chimera-carrier/src/peer_egress/lane_document/snapshot_parse.rs`
  - parses `chimera_plan_*` key/value comments through one `split_once('=')`
    and `match`, after tab-record handling;
  - redacts unknown plan-key error values by reporting only the key name.
- `crates/chimera-carrier/src/peer_egress/lane_document/tests.rs`
  - adds negative checks for snapshot-backed short rows;
  - adds unknown-key redaction checks for `key=value`, tab and whitespace
    malformed rows.

## Council Result

- Architect: accepted as a measured control-metadata slice, not a random
  micro-step; it supports mesh convergence metadata and does not touch payload.
- Senior developer: found a tab-form unknown-key redaction gap; fixed with
  `split_plan_key` and a tab negative test.
- QA: required control-plane smoke because stricter 6-field snapshot rows could
  break shell publish paths; `mesh-control-plane-env-smoke` passed.
- Security: found a whitespace-form unknown-key redaction gap; fixed by cutting
  unknown keys at first ASCII whitespace and adding a space negative test.
- Critic: required fresh perf evidence before making a speedup claim; final
  `metadata-perf-smoke` was rerun.

## Evidence

Commands passed on the final diff:

- `cargo fmt --all -- --check`
- `cargo check -q --workspace --all-targets`
- `cargo test -q --workspace --all-targets`
- `cargo clippy -q --workspace --all-targets -- -D warnings`
- `cargo test -q -p chimera-carrier lane_document -- --nocapture`
- `cargo test -q -p chimera-carrier planned_lane_document -- --nocapture`
- `cargo test -q -p chimera-carrier registration_only_lane_document -- --nocapture`
- `cargo test -q -p chimera-carrier live_bindings -- --nocapture`
- `cargo test -q -p chimera-cli lane_export -- --nocapture`
- `cargo test -q -p chimera-cli connect_probe_flow -- --nocapture`
- `just metadata-perf-smoke-selfcheck`
- `just mesh-control-plane-env-smoke-selfcheck`
- `just mesh-control-plane-env-smoke`
- `just rust-no-hardcode-guard-selfcheck`
- `just release-pack-schema-guard-selfcheck`
- `just ship-structure-guard-selfcheck`
- `git diff --check`

Baseline evidence from
`docs/WORKFLOW_ATTESTATION_METADATA_PERF_LANE_DOCUMENT_RENDER_PARSE_2026-06-28.md`:

```text
lane_document_render_parse_ops_per_sec=2951
lane_document_render_parse_p95_ns=356302
network_state=not_modified
```

Final `metadata-perf-smoke --iterations 20000 --json` evidence:

```text
lane_document_render_parse_ops_per_sec=3769
lane_document_render_parse_p95_ns=281434
network_state=not_modified
```

Interpretation of the measured hot path:

- throughput improved by about 27.7%;
- p95 latency improved by about 21.0%;
- this is a lab metadata/control measurement, not a real-runtime/datapath PASS.

## Risks And Limits

- Snapshot-backed lane documents now reject 3-field rows by design; this is
  covered by a negative test and the control-plane smoke.
- The lane document file still contains private endpoint metadata and must stay
  a private runtime binding file, not public diagnostic output.
- Broad WEAVE datapath performance, real-runtime mesh convergence, rollback and
  transparent application behavior are not closed by this slice.

## Rollback

- Remove the render capacity estimate and return to header-initialized render
  string allocation.
- Restore snapshot-backed row parsing to try 3-field rows before 6-field rows.
- Restore the prior repeated `strip_prefix` parser chain for plan key/value
  comments.
- Remove the new negative tests for strict snapshot rows and unknown-key
  redaction.
