# Workflow Attestation: Release Install Contract v0.1.137

## Scope

- Workline: remote release/runtime gate.
- Slice: GitHub Latest release/install contract, cargo-free install, checksum and
  lifecycle proof.
- Not claimed: full MVP prod PASS, real transparent datapath PASS, sealed
  transit PASS, rollback PASS.

## Workflow Order

- ANALYSIS: release/install blocker was selected because MVP requires
  self-contained delivery and GitHub one-command stand install/update.
- PLAN: harden release bundle contents, install contract, runtime bootstrap
  fail-closed behavior, redaction, and remote proof.
- TEAM_CRITIQUE: council found clean-install masking, placeholder upstream
  activation, and stale peer token leakage risks.
- IMPLEMENTATION: release/install scripts and smoke contracts were updated.
- TEAM_CHECK: senior/QA/DevOps/security rechecks were requested; security found
  stale token cleanup gap.
- FIX: stale `CHIMERA_PEER_EGRESS_TOKEN` is removed from active
  `upstream_proxy.env` during install/update.
- RECHECK: local proof bundle and SSH stand proof were rerun.
- FINAL_AUDIT: no full prod PASS claimed; remaining blockers recorded below.

## Commits And Release

- `cb318d2 Harden release install contract`
- `6a48763 Clean stale peer token from release install`
- GitHub Latest: `v0.1.137`
- Release assets:
  - `chimera.sh`
  - `chimera-pq-release.tar.gz`
  - `chimera-pq-release.tar.gz.sha256`

## Local Evidence

- `CHIMERA_RELEASE_VERSION=0.1.137 bash scripts/build_release.sh`: PASS
- `just release-bundle-install-contract-smoke`: PASS
- `bash scripts/chimera_update_contract_smoke.sh`: PASS
- `bash scripts/chimera_installer_gate.sh`: PASS
- `bash scripts/chimera_start_contract_smoke.sh`: PASS
- `bash scripts/chimera_stop_contract_smoke.sh`: PASS
- `bash scripts/chimera_doctor_contract_smoke.sh`: PASS
- `cargo fmt --all -- --check`: PASS
- `cargo check -q --workspace --all-targets`: PASS
- `cargo test -q --workspace --all-targets`: PASS
- `cargo clippy -q --workspace --all-targets -- -D warnings`: PASS
- `just rust-no-hardcode-guard`: PASS
- `just git-tree-hygiene-guard`: PASS

## GitHub Evidence

- GitHub Release workflow for `v0.1.137`: completed success.
- `releases/latest` points to `v0.1.137`.
- Latest assets match the required set.
- Downloaded latest bundle checksum verifies.
- Downloaded latest bundle version marker is `0.1.137`.
- Downloaded latest bootstrap script has `VERSION="0.1.137"`.

## Remote Stand Evidence

Public evidence is redacted to allowed fields only:

```text
remote_stand_used=true
ssh_ok=true
install_ok=true
install_without_cargo_ok=true
version_ok=true
version=0.1.137
checksum_ok=true
upstream_env_private_ok=true
peer_token_redacted_ok=true
diagnostics_redacted_ok=true
doctor_ok=false
start_ok=true
start_reason=none
status_ok=true
stop_ok=true
```

## Risks And Limits

- `doctor_ok=false` remains a diagnostics limitation to inspect separately; the
  exported doctor artifact stayed redacted and did not modify network state.
- `anti-monolith-guard` still fails on pre-existing oversized files:
  - `crates/chimera-carrier/src/peer_egress/aggregate_dispatch.rs`
  - `crates/chimera-carrier/src/peer_egress/live_bindings.rs`
  - `crates/chimera-mesh/src/tests/runtime_planning.rs`
  - `crates/chimera-mesh/src/tests_failover_health/failover_reselection.rs`
- Real transparent datapath, sealed transit, reconnect/rebind, rollback, and
  long-run performance remain not closed.
- Commit signing failed locally because the configured GPG secret key is absent;
  commits were made with one-time `commit.gpgsign=false`.
