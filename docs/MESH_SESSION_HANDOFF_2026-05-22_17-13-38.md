# CHIMERA Mesh Session Handoff

## Saved At
- Timestamp: 2026-05-22_17-13-38 local session

## Active Objective
- Continue CHIMERA-PQ MVP mesh-node work by binding user-facing node listing to config-backed inventory while preserving read-only diagnostics and no live OS mutation.

## What Was Done
- Added config-backed mesh node inventory loading:
  - `crates/chimera-cli/src/mesh_cli/nodes_inventory.rs`
  - Supports `--config <path>`, `CHIMERA_MESH_NODES_CONFIG`, and existing `--node` records.
  - Config format uses `mesh.nodes.ids` plus `mesh.node.<id>.*` fields.
- Split mesh node CLI rendering:
  - `crates/chimera-cli/src/mesh_cli/nodes_render.rs`
  - `mesh nodes list` now shows Recommended, Current, Pinned, and Autoconnect.
- Slimmed dispatcher:
  - `crates/chimera-cli/src/mesh_cli/nodes_cmd.rs`
- Added tests:
  - `crates/chimera-cli/src/mesh_cli/tests_nodes_inventory.rs`
- Added example inventory:
  - `configs/mesh_nodes.example.conf`
- Refactored old oversized connect/launch parity test files into submodules under:
  - `crates/chimera-cli/src/mesh_cli/tests_connect_launch_error_parity/`
  - `crates/chimera-cli/src/mesh_cli/tests_connect_launch_error_parity_options/`
  - `crates/chimera-cli/src/mesh_cli/tests_connect_launch_error_parity_options_required/`
- Removed Python/device-specific defaults from:
  - `scripts/chimera_load_5m_laptop.sh`
  - Script now requires env/config values and uses shell/curl on the remote side.

## Validation
- PASS: `cargo fmt --all -- --check`
- PASS: `cargo clippy -p chimera-mesh --all-targets -- -D warnings`
- PASS: `cargo clippy -p chimera-cli --all-targets -- -D warnings`
- PASS: `cargo test -p chimera-mesh` (143 tests)
- PASS: `cargo test -p chimera-cli` (305 tests)
- PASS: `cargo test -p chimera-cli tests_nodes_inventory -- --nocapture` (4 tests)
- PASS: `cargo test -p chimera-cli tests_connect_launch_error_parity -- --nocapture` (92 tests)
- PASS: `bash scripts/anti_monolith_guard.sh`
- PASS: `bash scripts/ship_structure_guard.sh`
- PASS: `cargo run -q -p chimera-lab --bin rust_no_hardcode_guard`
- PASS: CLI smoke with temp config for `mesh nodes list`, `best`, and `explain`.
- PASS: `cargo run -q -p chimera-cli -- mesh nodes list --config configs/mesh_nodes.example.conf`
- PASS: targeted grep for forbidden machine-specific literals in changed mesh-node/config/load script files returned no matches.

## Known Open Items
- This still does not mutate live routes/DNS/firewall/proxy and does not prove real-world datapath connectivity.
- `probe --all` remains not bound to the health probe backend.
- `connect/pin/autoconnect` are still command-local decisions, not persistent runtime state.
- `chimera-pq` is still not a registered Amai project code; Amai continuity is recorded under project code `amai`.
- Git status cannot be reported because `/home/art/Archives/VPN/chimera-pq` and `/home/art/Archives/VPN` are not git repositories in this workspace.

## Safety
- No OS routes, DNS, firewall, system proxy, Happ, MYVPN, router, or VPS settings were changed.
- Config inventory listing/best/explain remains read-only.

## Next Step
- Bind `mesh nodes probe --all` to the existing health/connect-probe backend and persist current/pinned/autoconnect state through the runtime config/state layer.
