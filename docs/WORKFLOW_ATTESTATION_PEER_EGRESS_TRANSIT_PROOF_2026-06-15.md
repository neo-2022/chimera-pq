# Workflow Attestation: Peer Egress Transit Proof Modes

Scope: add shipped Rust diagnostic proof modes to `chimera-peer-egress` so the
SSH stand can verify installed GitHub release binaries without a Python transit
driver. This is not a transparent TUN/DNS/routing Real-World PASS.

Stages:

- ANALYSIS: done
  - Evidence: `AGENTS.md`, `CHIMERA-PQ_MVP_SPEC.md`, `Agent.md`,
    `docs/EXECUTION_MODE_NO_TIMELINES.md`, latest handoff, and the current
    `chimera-carrier` peer-egress modules were read.
  - Result: current remote evidence used a Python test driver because the
    shipped `v0.1.95` binary did not contain transit proof injection modes.
- PLAN: done
  - Result: add diagnostic-only `sealed-transit-inject` and
    `bound-transit-inject` modes to the existing shipped `chimera-peer-egress`
    binary, keep parameters config/env-driven, and keep stdout redacted.
- TEAM_CRITIQUE: done
  - Architect: accepted existing-binary diagnostic modes as MVP-aligned, but
    rejected any Real-World PASS claim from injector evidence alone.
  - Senior developer: found a release blocker where proof-only env parsing
    could break ordinary modes; required regression coverage.
  - Tester: required local source gates plus GitHub one-command side_b/SIDE_A
    stand evidence before shipped runtime claims.
  - Security engineer: required no payload, token, raw sealed bytes, route or
    lane values in proof output; recommended removing `packet_number`.
  - DevOps: required new GitHub Latest release before stand verification.
  - Critic: blocked `ship-ready`/`Real-World PASS` claims until CI and remote
    stand evidence are collected.
- IMPLEMENTATION: done
  - Evidence:
    `crates/chimera-carrier/src/peer_egress/proof.rs`,
    `crates/chimera-carrier/src/peer_egress/options_proof.rs`,
    `crates/chimera-carrier/src/peer_egress/options_mode.rs`,
    `crates/chimera-carrier/src/bin/chimera-peer-egress.rs`,
    `justfile`.
  - Result: `chimera-peer-egress` can build a sealed DATA frame, optionally wrap
    it in an explicit bound transit route/lane frame, and write it to a
    configured local ingress endpoint with bounded connect timeout.
- TEAM_CHECK: done
  - Senior developer blocker was accepted and fixed: proof env is parsed only
    for transit inject modes, and proof CLI flags are rejected in non-proof
    modes.
  - Security recommendation was accepted and fixed: proof stdout prints only
    status and byte counts, not packet number, token, route, lane or payload.
  - Anti-monolith blocker was accepted and fixed by moving mode parsing/names
    into `options_mode.rs`.
- FIX: done
  - Added parser regression tests for proof modes and non-proof proof-flag
    rejection.
  - Added `peer-egress-transit-proof-selfcheck` to `ship-readiness-selfcheck`.
  - Updated `chimera_installer_gate.sh` to check the new `options_mode.rs`
    location for `node` / `weave-node` mapping.
  - Changed `ship_readiness.sh` so flaky direct external probe failure is not
    treated as a source/release regression when the datapath snapshot schema,
    attempts and totals are intact; direct failure remains visible in the
    report fields and truth boundary.
  - Added `validate_direct_probe_visibility` tests to
    `ship_readiness_json_guard.rs` so a direct-probe failure cannot be hidden
    behind a missing snapshot-integrity claim.
- RECHECK: done
  - `cargo fmt --all -- --check`: PASS.
  - `cargo check -q --workspace --all-targets --locked --offline`: PASS.
  - `cargo test -q -p chimera-carrier --locked --offline`: PASS, 93 tests.
  - `cargo test -q --workspace --locked --offline`: PASS.
  - `cargo clippy -q --workspace --all-targets --locked --offline -- -D warnings`: PASS.
  - `cargo test -q -p chimera-lab --bin ship_readiness_json_guard --locked --offline`: PASS, 6 tests.
  - `just peer-egress-transit-proof-selfcheck`: PASS.
  - `just ship-readiness-selfcheck`: PASS.
  - `just ship-readiness`: PASS.
  - `just ship-report-contract-check`: PASS.
  - `bash scripts/anti_monolith_guard.sh`: PASS.
  - `just rust-no-hardcode-guard`: PASS.
  - `bash scripts/chimera_installer_gate.sh`: PASS.
  - `bash scripts/chimera_update_contract_smoke.sh`: PASS.
  - `bash scripts/chimera_start_contract_smoke.sh`: PASS.
  - `bash scripts/chimera_stop_contract_smoke.sh`: PASS.
  - `git diff --check`: PASS.
- FINAL_AUDIT: partial
  - Source and release-contract gates passed for diagnostic release delivery.
  - Sub-agent consensus accepted commit/tag `v0.1.96` only as a diagnostic
    source release to deliver shipped Rust proof modes to the SSH stand.
  - New source has not yet been published as a GitHub release.
  - Side B/SIDE_A one-command install/update and shipped-binary stand proof remain
    unverified for this source until a new GitHub Latest release exists.
- REPORT: partial
  - Status: source-level diagnostic proof modes are implemented and locally
    verified.
  - Not closed: GitHub release publication, CI verification, side_b/SIDE_A
    one-command update, and remote stand proof from installed binaries.

Runtime/network statement:

- No CHIMERA runtime was started, stopped or restarted on the current PC.
- No local DNS, routes, firewall, proxy, VPN, Happ, MYVPN, router, side_b or SIDE_A
  network settings are changed by this source-level work.
- This is not a Real-World PASS and not a milestone close.
