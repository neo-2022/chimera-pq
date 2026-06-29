# CHIMERA Metadata Performance Attestation: Selection Policy Region Lookup

## Scope

- Date: 2026-06-27
- Hot path: `selection_policy_region_lookup`
- Scope boundary: hot metadata only
- Status: Lab/Metadata PASS
- Transit payload policy: opaque sealed payload untouched
- Network state: not modified

## Selection Rationale

- This slice stays on the live path-planner candidate selection path.
- It removes repeated normalized-region cloning from region-cap bookkeeping and
  resilient spread counting.
- It keeps selected peer order, region-cap behavior, and redaction unchanged.

## Change

- `crates/chimera-mesh/src/runtime/selection_policy_select.rs`
  - region-cap lookups now borrow normalized region keys and only clone on
    first insert into the owned maps/sets;
  - the first-pass/backlog order is unchanged.
- `crates/chimera-mesh/src/runtime/selection_policy_spread.rs`
  - resilient spread counting now borrows normalized region keys for lookups
    and only clones on first insert.
- `docs/PERFORMANCE.md`
  - records the new `selection_policy_region_lookup` slice.

## Evidence

Commands passed locally without changing network state:

- `cargo fmt --all -- --check`
- `cargo test -q -p chimera-mesh tests_selection_behavior`
- `cargo test -q -p chimera-mesh runtime_planning`
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings`
- `just metadata-perf-smoke`

`just metadata-perf-smoke` output after this slice:

```json
{"path_planner_candidate_snapshot_ops_per_sec":6784,"path_planner_candidate_snapshot_p95_ns":158812}
```

## What Is Not Closed

- Broad WEAVE datapath performance is not closed.
- Real-world runtime/datapath checks remain SSH-stand work.
- Further planner hot spots may still exist.

## Rollback

- Restore the old `region_key.clone()` / `entry(region_key.to_string())` path
  in `selection_policy_select.rs` and `selection_policy_spread.rs`.
- Remove the `selection_policy_region_lookup` bullet from `docs/PERFORMANCE.md`.
