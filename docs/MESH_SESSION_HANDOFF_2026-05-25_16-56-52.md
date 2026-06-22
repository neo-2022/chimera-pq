# CHIMERA Mesh Session Handoff

## Saved At
- Timestamp: 2026-05-25_16-56-52 local session

## Active Objective
- Validate remote non-regression for mesh nodes contract hardening on both side_b and SIDE_A; keep SSH-only execution policy and avoid any local/WEAVE network mutations.

## What Was Done
- Enforced SSH-only persistent rule in workspace policy (`AGENTS.md` section 5.2).
- Synced mesh contract-hardening changes to SIDE_A codebase (`/root/chimera-pq`):
  - `crates/chimera-cli/src/mesh_cli/nodes_cmd.rs`
  - `crates/chimera-cli/src/mesh_cli/tests_nodes_runtime_state.rs`
  - `docs/MESH_NODES_JSON_CONTRACT.md`
- Upgraded SIDE_A user-space Rust toolchain via rustup (no network policy/WEAVE/routing changes) to satisfy `resolver = 3` cargo manifest.
- Synced side_b main repo by replacing stale `<stand-repo-root>` with verified mesh-check copy and preserving backup:
  - backup path: `<stand-repo-root>.pre_mesh_sync_20260525_162509`

## Validation
- PASS (SIDE_A, earlier pass before SSH outage):
  - `cargo test -p chimera-cli tests_nodes_runtime_state -- --nocapture` (27 passed)
- PASS (Side B main repo):
  - `cargo fmt --all -- --check`
  - `cargo test -p chimera-cli tests_nodes_runtime_state -- --nocapture` (27 passed)
- PASS (Side B non-regression):
  - `cargo test -p chimera-cli tests_nodes_inventory -- --nocapture` (16 passed)
  - `cargo test -p chimera-cli tests_nodes_reenroll -- --nocapture` (5 passed)

## Known Open Items
- SIDE_A SSH currently unstable/unavailable for final paired non-regression rerun:
  - observed errors: `Connection reset by peer`, then `Connection timed out during banner exchange`.
- `AGENTS.md` sync to side_b confirmed; sync to SIDE_A could not be re-verified after SSH outage.

## Safety
- No changes to SIDE_A/PC/side_b routing tables, DNS, firewall, system proxy, or MYWEAVE settings.
- Work limited to code/test/doc sync and user-space toolchain in repository context.

## Next Step
- Restore SIDE_A SSH reachability and rerun:
  - `cargo test -p chimera-cli tests_nodes_inventory -- --nocapture`
  - `cargo test -p chimera-cli tests_nodes_reenroll -- --nocapture`
- Then finalize stage report with complete dual-host evidence bundle.
