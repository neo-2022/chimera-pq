# CHIMERA Metadata Performance Attestation: DPS Payload Explain Capacity

## Scope

- Date: 2026-06-27
- Hot path: `dps_payload_explain_capacity`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- DPS payload explain appends a long decision + standby tail after the initial
  payload parsing phase.
- Reserving capacity up front reduces `Vec` growth on that hot path.
- The existing `metadata-perf-smoke` run already covers live DPS plan path and
  related explain work.

## Change

- `crates/chimera-mesh/src/runtime/dps_payload_explain.rs`
  - reserves room for the long DPS explain tail before appending payload lines;
  - preserves explain ordering, keys, and redaction.
- `docs/PERFORMANCE.md`
  - records the new DPS explain capacity slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh tests_dps_explain`
- `cargo test -q -p chimera-lab metadata_perf_json_is_redacted_and_metadata_only`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `just metadata-perf-smoke`
- `just metadata-perf-smoke-selfcheck`

`just metadata-perf-smoke` output after this slice:

```json
{"live_dps_plan_path_from_payload_ops_per_sec":3452,"live_dps_plan_path_from_payload_p95_ns":312488}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/load/datapath checks remain SSH-stand work.
- Other metadata hotspots may still remain.

## Rollback

- Remove the `explain.reserve(64)` line from `dps_payload_explain.rs`.
- Remove the bullet from `docs/PERFORMANCE.md`.
