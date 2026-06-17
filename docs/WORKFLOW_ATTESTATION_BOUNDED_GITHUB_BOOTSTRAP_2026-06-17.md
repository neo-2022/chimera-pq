# Workflow Attestation: Bounded GitHub Bootstrap

Status: source_ready_for_release
Date: 2026-06-17

## ANALYSIS

Objective: keep laptop/VPS stand install/update on the GitHub one-command path
while preventing a network stall before the published bootstrap script starts.

Observed on the laptop stand:

- installed marker already showed `0.1.104`;
- the unbounded outer command
  `curl --disable -fsSL .../chimera.sh | bash -s -- -install` stalled in the
  outer `curl` before `bash -s -- -install` could execute the bootstrap;
- the installed launcher still reported `chimera-runtime 0.1.104`;
- no local PC CHIMERA runtime was started.

## PLAN

1. Keep GitHub Release/Latest as the canonical stand source.
2. Keep one shell command, but wrap it with `bash -o pipefail -c` and add
   timeout/retry bounds to the outer bootstrap download:
   `curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60`.
3. Update remote-cycle smoke so proof automation cannot hang on the outer curl.
4. Add installer guard checks for the bounded command.
5. Update operations/release docs.
6. Run source/release gates, then publish a new GitHub Release/Latest and
   repeat laptop/VPS installed-binary proof.

## TEAM_CRITIQUE

Real sub-agent roles were asked to review the fix:

- architect;
- senior developer;
- tester;
- security engineer;
- DevOps;
- critic-skeptic.

Consensus before implementation:

- accepted: bounded outer GitHub bootstrap, release-required fix;
- rejected: local tarball/scp/cargo proof, source-only Real-World PASS, and any
  proxy/workaround proof.

## IMPLEMENTATION

Changed files:

- `scripts/chimera_remote_cycle_smoke.sh`;
- `scripts/chimera.sh`;
- `scripts/chimera_installer_gate.sh`;
- `docs/OPERATIONS.md`;
- `docs/RELEASE_BUILD_SEQUENCE.md`.

Implementation result:

- the canonical stand command remains a single GitHub pipe command;
- the outer bootstrap download is bounded with `--disable`, retries,
  connect-timeout and max-time;
- the outer shell uses `pipefail`, so a failed download cannot be hidden by an
  empty `bash` process on the right side of the pipe;
- `chimera_remote_cycle_smoke.sh` uses the bounded command;
- `chimera_installer_gate.sh` fails if the bounded outer command is removed.

## TEAM_CHECK

Status: done.

The sub-agent team reviewed the patch after local gates.

Accepted:

- commit/release pipeline for `v0.1.106`;
- bounded outer GitHub bootstrap as the correct fix for the observed hang;
- `pipefail` as the required follow-up fix for the observed false success on
  the laptop when the outer download failed;
- no security/privacy blocker in this install/update reliability change.

Still blocked for strong claims:

- installed-binary proof on laptop and VPS for the new release;
- real runtime/datapath/transit/rollback proof.

## FIX

Status: applied.

Root cause fixed in the operator/proof command surface: the first GitHub
download can no longer hang indefinitely before the installer starts.

## RECHECK

PASS:

- `bash -n scripts/chimera_remote_cycle_smoke.sh scripts/chimera.sh scripts/chimera_installer_gate.sh`
- `bash scripts/chimera_installer_gate.sh`
- `bash scripts/chimera_update_contract_smoke.sh`
- `bash scripts/chimera_start_contract_smoke.sh`
- `bash scripts/chimera_stop_contract_smoke.sh`
- `cargo fmt --all -- --check`
- `cargo check -q --workspace`
- `cargo test -q --workspace`
- `cargo clippy -q --workspace --all-targets -- -D warnings`
- `bash scripts/anti_monolith_guard.sh`
- `just rust-no-hardcode-guard`
- `cargo test -q -p chimera-carrier peer_egress::proof`
- `cargo test -q -p chimera-mesh tests_multipath_schedule`
- `cargo test -q -p chimera-cli tests_json_operator_cross_contract`

## FINAL_AUDIT

Status: not_complete_until_release_and_stand_proof.

Accepted source facts:

- the fix is install/update reliability only;
- it does not change WEAVE payload/datapath behavior;
- no local PC runtime proof was used.

Not closed yet:

- new GitHub Release/Latest for this fix;
- laptop one-command update proof from the new release;
- VPS one-command update proof from the new release;
- installed version/checksum proof for the new release.

## REPORT

Status: partial.

This source fix is ready for commit/release pipeline. It is not a Real-World
datapath PASS.
