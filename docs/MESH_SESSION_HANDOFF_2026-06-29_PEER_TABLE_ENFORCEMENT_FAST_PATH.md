# CHIMERA Mesh Session Handoff: Peer Table Enforcement Fast Path

## Saved At

- Timestamp: 2026-06-29T21:36:00Z

## Active Objective

- Continue speeding metadata/control paths that help WEAVE nodes discover peers,
  choose paths, rebuild route/lane/binding metadata, publish state, and reduce
  CPU/RAM waste.
- Keep transit payload opaque/sealed and untouched.
- Keep product git tree clean between verified slices.

## What Changed

- Optimized `compute_enforcement` in `chimera-mesh`.
  - Added no-drop early return when total peers cannot exceed either global or
    per-region cap.
  - Replaced eager cloned `(String, priority)` sort entries with borrowed
    enforcement candidates.
  - Cached normalized regions as indexes, avoiding repeated
    `normalize_region_key` allocations during region/global cap passes.
  - Kept final `drop_set` owned because the caller removes from maps after the
    computation.
- Fixed and completed the already-started traffic-hints metadata perf path.
  - `traffic_hints_from_dps_payload` now parses hint fields in one pass.
  - Added one-pass vs four-pass baseline metrics.
- Added peer-table enforcement perf evidence to `metadata-perf-smoke`.
  - New hot path: `peer_table_enforcement_noop`.
  - New JSON fields include iterations, peer count, ops/sec and p95 ns.
- Added behavior tests for deterministic peer-table tie handling.
  - Region cap tie preserves current node-id order.
  - Global cap tie drops deterministically by node id.

## Measured Evidence

Latest metadata perf smoke:

```text
cargo run -q -p chimera-lab --bin chimera-lab -- metadata-perf-smoke --iterations 20000 --json
```

Key fields:

- `peer_table_enforcement_noop_iterations=10000`
- `peer_table_enforcement_peer_count=64`
- `peer_table_enforcement_noop_ops_per_sec=222876`
- `peer_table_enforcement_noop_p95_ns=4875`
- `traffic_hints_one_pass_ops_per_sec=304505`
- `traffic_hints_one_pass_p95_ns=3681`
- `traffic_hints_four_pass_baseline_ops_per_sec=107374`
- `traffic_hints_four_pass_baseline_p95_ns=9754`
- `live_binding_reload_index_ops_per_sec=204954`
- `live_binding_reload_index_p95_ns=5286`
- `live_dps_plan_core_from_payload_ops_per_sec=30206`
- `live_dps_plan_path_from_payload_ops_per_sec=4806`
- `network_state=not_modified`
- `transit_payload_policy=opaque_sealed_payload_untouched`

## Validation

PASS:

- `cargo fmt --all -- --check`
- `cargo check -q -p chimera-mesh`
- `cargo check -q -p chimera-lab`
- `cargo test -q -p chimera-mesh peer_table -- --nocapture`
- `cargo test -q -p chimera-mesh traffic_hints -- --nocapture`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only -- --nocapture`
- `cargo run -q -p chimera-lab --bin chimera-lab -- metadata-perf-smoke --iterations 20000 --json`
- `cargo test -q --workspace --all-targets`
- `cargo clippy -q --workspace --all-targets -- -D warnings`
- `just rust-no-hardcode-guard-selfcheck`
- `just release-pack-schema-guard-selfcheck`
- `just ship-structure-guard-selfcheck`
- `git diff --check`

## Review Notes

- Sub-agent review found the key invariants:
  region cap before global cap, region normalization, priority then node-id
  ordering, protected-region skips, and owned `drop_set` for caller removals.
- Accepted: add deterministic tie tests and preserve behavior.
- Rejected: treating generic critic commentary as a decision. The accepted
  basis is code plus tests plus perf evidence.

## Not Closed

- This is not Real-World PASS.
- Remote carrier reconnect/retry on the SSH stand is not verified in this
  slice.
- One-command install/update is not verified in this slice.
- Full production readiness is not claimed.

## Next Step

- Continue with the next metadata/control bottleneck, or switch to the remote
  real-runtime gate if prod-readiness is the priority:
  carrier reconnect/retry on the SSH stand, release install/update, and
  real-world proof bundle.
