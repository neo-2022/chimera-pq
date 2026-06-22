# Workflow Attestation: Multipath Rebuild Control

Date: 2026-06-18
Updated UTC: 2026-06-18T06:13:00Z
Base commit observed before this source slice: `d2b2ce3`

Scope: source-level WEAVE multipath rebuild freshness/debounce/fail-closed
control in `chimera-mesh`.

This is not live carrier traffic proof, not TUN/OS routing proof, not DNS
binding proof, not browser/IDE workflow proof, and not Real-World datapath
PASS.

## ANALYSIS

Status: done

- MVP scope requires a symmetric WEAVE mesh node where each node can accept,
  egress, and forward sealed transit traffic.
- Transit payload opacity remains mandatory: transit payload is closed third
  party information and must stay opaque/sealed to transit nodes.
- Previous source-level multipath flow assignment could mark
  `rebuild_recommended`, but had no runtime-owned state to coalesce duplicate
  soft rebuild signals or reject stale telemetry.
- Council consensus accepted only a narrow runtime-owned guard:
  - pure planners stay pure;
  - debounce applies only to soft rebuild signals;
  - stale telemetry and hard-safety signals must not silently continue
    assignment;
  - diagnostics must stay aggregate and redacted.

## PLAN

Status: done

- Add a small runtime-owned rebuild control state to `MeshRuntime`.
- Keep public schedule and flow planners pure.
- Split rebuild model/types from rebuild state-machine to preserve
  anti-monolith structure.
- Add source tests for duplicate suppression, window expiry, changed reason or
  generation/fingerprint, stale telemetry, urgent failover, hard safety, and
  redaction.
- Do not run CHIMERA runtime on the local PC and do not mutate local network
  state.

## TEAM_CRITIQUE

Status: done

- Architect: accepted a runtime-owned guard, but blocked any Real-World claim
  without side_b/SIDE_A proof.
- Senior Rust: required state in `MeshRuntime`, a separate module, no mutation
  of pure `plan_path(&self)` or `plan_multipath_flow()`.
- Tester: required tests for duplicate suppression, stale telemetry, urgent
  failover, hard-safety bypass, and redaction.
- Security: required only aggregate diagnostics: action, reason enum, booleans,
  counters and policy labels; no raw payload, peer id, endpoint, route id, flow
  id or fingerprint.
- DevOps: required full source gates and a new GitHub Release/Latest before
  side_b/SIDE_A installed proof.
- Critic: rejected a cache-as-truth design and required fail-closed behavior for
  stale/unsafe inputs.

## IMPLEMENTATION

Status: done

Files changed or added:

- `crates/chimera-mesh/src/runtime/multipath_rebuild_model.rs`
- `crates/chimera-mesh/src/runtime/multipath_rebuild_control.rs`
- `crates/chimera-mesh/src/runtime.rs`
- `crates/chimera-mesh/src/lib.rs`
- `crates/chimera-mesh/src/tests_multipath_schedule/mod.rs`
- `crates/chimera-mesh/src/tests_multipath_schedule/rebuild_control.rs`

Implemented behavior:

- `MeshRuntime` now owns private `MeshMultipathRebuildState`.
- `MeshRuntime::evaluate_multipath_rebuild(&mut self, ...)` evaluates a
  rebuild signal against a validated policy.
- First soft signal is allowed.
- Duplicate soft signal with the same reason, generation, fingerprint and
  telemetry epoch is suppressed inside the debounce window.
- Same soft signal is allowed again after the debounce window.
- Changed reason, generation, fingerprint, or telemetry epoch allows rebuild
  instead of suppressing as a duplicate.
- Stale telemetry fails closed.
- Telemetry from the future fails closed.
- Hard-safety signals fail closed and are not debounced.
- Urgent failover bypasses soft debounce.
- Public diagnostics remain aggregate-only.
- Rebuild reason labels reject uppercase, spaces, endpoints, punctuation,
  surrounding whitespace and newline injection.

## FIX

Status: done

Senior audit found one real blocker:

- `validate_rebuild_reason()` originally validated `reason.trim()` but stored
  the raw reason. This could allow explain/debug injection and duplicate
  debounce bypass through whitespace variants.

Fix:

- Rebuild reason validation now rejects surrounding whitespace instead of
  silently trimming it.
- Regression tests now reject leading space and trailing newline reasons.

Security audit then found a second blocker:

- Rebuild reason still accepted arbitrary lowercase labels such as `route_7009`,
  `peer_123`, `dead_beef` or `payload_secret`, which could leak route-like,
  peer-like, fingerprint-like or secret-like material through public
  diagnostics.

Fix:

- Rebuild reason validation now requires a closed allowlist of known reason
  labels.
- Regression tests now reject `route_7009`, `peer_123`, `dead_beef` and
  `payload_secret`.

Security audit then found a third blocker:

- `MeshMultipathRebuildSignal` fields were public, so external code could bypass
  validating constructors with a struct literal and leak route-like or
  fingerprint-like data through `Debug` before runtime evaluation.

Fix:

- `MeshMultipathRebuildSignal` fields are now private.
- Read-only accessor methods expose only validated values.
- A `compile_fail` doctest proves external code cannot construct the signal with
  a raw `reason` field.

Security audit then found a fourth blocker:

- `MeshRuntime` derived `Debug`, which could print peer table endpoints and the
  private multipath rebuild state. The rebuild state derived `Debug` and could
  print `schedule_fingerprint`.

Fix:

- `MeshRuntime` now has a manual redacted `Debug` implementation that prints
  counts instead of peer maps and redacts namespace.
- `MeshMultipathRebuildState` and `AllowedRebuild` now have manual `Debug`
  implementations; `schedule_fingerprint` is rendered as `<redacted>`.
- Regression test verifies `format!("{runtime:?}")` does not expose the
  fingerprint, endpoint or peer id after a rebuild decision.

## RECHECK

Status: done

Commands run from `<repo-root>`:

- `cargo fmt --all -- --check` PASS
- `cargo test -q -p chimera-mesh tests_multipath_schedule::rebuild_control`
  PASS
  - 10 tests passed
- `cargo test -q -p chimera-mesh --doc` PASS
  - 1 compile-fail doctest passed
- `cargo check -q --workspace` PASS
- `cargo test -q -p chimera-mesh` PASS
  - 218 tests passed
- `cargo test -q --workspace` PASS
- `cargo clippy -q -p chimera-mesh --all-targets -- -D warnings` PASS
- `cargo clippy -q --workspace --all-targets -- -D warnings` PASS
- `bash scripts/anti_monolith_guard.sh` PASS
- `just rust-no-hardcode-guard` PASS
- `bash scripts/chimera_installer_gate.sh` PASS
- `bash scripts/chimera_update_contract_smoke.sh` PASS
- `bash scripts/chimera_start_contract_smoke.sh` PASS
- `bash scripts/chimera_stop_contract_smoke.sh` PASS
- `git diff --check` PASS

Anti-monolith status:

- `crates/chimera-mesh/src/runtime/multipath_rebuild_control.rs`: 218 lines.
- `crates/chimera-mesh/src/runtime/multipath_rebuild_model.rs`: 267 lines.
- `crates/chimera-mesh/src/tests_multipath_schedule/rebuild_control.rs`: 254
  lines.
- `crates/chimera-mesh/src/runtime.rs`: 409 lines.
- `bash scripts/anti_monolith_guard.sh` PASS.

## FINAL_AUDIT

Status: partial

Completed:

- Architect final audit: PASS for source-level architecture; blocked only
  Real-World/runtime claims without side_b/SIDE_A proof.
- Senior Rust final audit: initially BLOCK due to rebuild reason injection risk;
  fix applied and full source recheck passed.

Pending at document creation:

- Architect final audit: PASS for source-level architecture; blocked only
  Real-World/runtime claims without side_b/SIDE_A proof.
- Senior Rust final audit: PASS after whitespace-injection fix.
- Tester final audit: PASS after whitespace-injection fix.
- Security final audit: PASS after allowlist, private-field/compile-fail, and
  runtime Debug redaction fixes.

## REPORT

Status: source-level pass

Closed:

- Source-level runtime-owned multipath rebuild control state.
- Duplicate soft rebuild suppression.
- Rebuild after debounce window.
- Rebuild when reason/generation/fingerprint/telemetry epoch changes.
- Stale telemetry fail-closed behavior.
- Future telemetry fail-closed behavior.
- Hard-safety fail-closed behavior.
- Urgent failover debounce bypass.
- Aggregate-only diagnostics for the new rebuild control path.
- Source gates listed above.

Not closed:

- GitHub Release/Latest for this slice.
- Side B/SIDE_A one-command update proof for this slice.
- Live carrier traffic between side_b and SIDE_A.
- Real sealed transit forwarding of third-party traffic.
- Transparent TUN/OS routing.
- DNS-to-route runtime binding.
- Crash/forced-stop rollback on stand.
- Browser/IDE normal workflow without proxy/manual workaround.
- Real multipath throughput, fairness and long-run behavior.

Truth-first status:

- Source-level rebuild control: PASS by local source gates and role re-audit.
- Installed release/update proof: not done for this slice.
- Real-World datapath PASS: not verified.
