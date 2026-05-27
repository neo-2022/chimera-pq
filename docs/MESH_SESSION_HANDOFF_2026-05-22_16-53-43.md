# CHIMERA Mesh Session Handoff

## Saved At
- Timestamp: 2026-05-22 16:53:43 local session

## Active Objective
- Continue CHIMERA-PQ MVP mesh-node work: user-facing available-node list grouped by country, best-node selection, explain, and anti-flap logic.

## What Was Done
- Added mesh-node domain modules in `crates/chimera-mesh/src/`:
  - `nodes_model.rs`
  - `nodes_policy.rs`
  - `nodes_scoring.rs`
  - `nodes_grouping.rs`
  - `nodes_runtime.rs`
  - `nodes_explain.rs`
- Exported mesh-node API from `crates/chimera-mesh/src/lib.rs`.
- Added CLI subcommand module:
  - `crates/chimera-cli/src/mesh_cli/nodes_cmd.rs`
- Wired `chimera mesh nodes` into `crates/chimera-cli/src/mesh_cli/mod.rs`.
- Updated mesh usage text in `crates/chimera-cli/src/main.rs`.
- Added tests in:
  - `crates/chimera-mesh/src/tests_mesh_nodes/mod.rs`

## Implemented Behavior
- `mesh nodes list` groups nodes by country.
- Unknown country is shown as `Неизвестная страна` and is kept visible.
- Country groups are deterministic, unknown group sorts last.
- Nodes sort by status, score, latency, and node id.
- Filters: `--country`, `--status`, `--available-only`, `--search`.
- `mesh nodes best` selects best non-down node.
- `mesh nodes explain --id <node>` shows score breakdown and country reason codes.
- Runtime logic includes autoconnect, manual connect, pin/unpin, emergency switch, hold-down, hysteresis, and switch-rate guard.

## Validation
- PASS: `cargo fmt --all -- --check`
- PASS: `cargo clippy -p chimera-mesh --all-targets -- -D warnings`
- PASS: `cargo clippy -p chimera-cli --all-targets -- -D warnings`
- PASS: `cargo test -p chimera-mesh`
- PASS: `cargo test -p chimera-cli`
- PASS: CLI smoke:
  - `cargo run -q -p chimera-cli -- mesh nodes list ...`
  - `cargo run -q -p chimera-cli -- mesh nodes best ...`
  - `cargo run -q -p chimera-cli -- mesh nodes explain --id ...`

## Known Open Items
- `anti_monolith_guard.sh` still fails on pre-existing oversized files:
  - `crates/chimera-cli/src/mesh_cli/tests_connect_launch_error_parity.rs`
  - `crates/chimera-cli/src/mesh_cli/tests_connect_launch_error_parity_options.rs`
  - `crates/chimera-cli/src/mesh_cli/tests_connect_launch_error_parity_options_required.rs`
- `rust_no_hardcode_guard` still fails on pre-existing script Python usage:
  - `scripts/chimera_load_5m_laptop.sh:103`
- `chimera-pq` is not registered as an Amai project code; Amai handoff was also recorded under project code `amai`.
- Current CLI node source is explicit `--node` records. Persistent config-backed discovery/listing should be the next implementation step.

## Safety
- No OS routes, DNS, firewall, system proxy, Happ, MYVPN, router, VPS runtime network settings changed by this code path.
- New `list`, `best`, and `explain` paths are read-only over supplied node records.

## Next Step
- Bind `mesh nodes` to real config/runtime node inventory instead of only CLI-supplied `--node` records, then package/release after release guards are green or known guard debts are fixed.
