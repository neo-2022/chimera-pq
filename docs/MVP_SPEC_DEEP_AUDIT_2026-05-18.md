# CHIMERA-PQ MVP Deep Audit (2026-05-18)

## Scope
- Target spec: `CHIMERA-PQ_MVP_SPEC.md`
- Repo: `chimera-pq`
- Audit type: code + tests + artifact consistency + runtime-behavior claims check

## Executed Validation Commands
- `just check`
- `just test`
- `just lint`
- `just fuzz-smoke`
- `just deny`
- `just net-sim`
- `just perf-smoke`
- `cargo run -p chimera-lab -- mvp-spec-check --json --out docs/mvp_spec_check_audit.json`
- `cargo run -p chimera-lab -- release-readiness-report --json --out docs/release_readiness_audit.json`
- `cargo run -p chimera-lab -- mvp-verify --json --out docs/MVP_VERIFY.json`
- `cargo run -p chimera-gateway -- run --config configs/gateway.example.conf`

## High-Severity Findings

### 1) M4/M5 real datapath gap (system-level capture/routing is not implemented)
Spec requires local capture via TUN or local proxy mode and practical routing behavior:
- `CHIMERA-PQ_MVP_SPEC.md:23`
- `CHIMERA-PQ_MVP_SPEC.md:170-190`
- `CHIMERA-PQ_MVP_SPEC.md:208-212`

Observed implementation state:
- Capture crate still contains planning-level mode selection (`crates/chimera-capture/src/lib.rs:17-29`),
  but runtime apply paths now exist in CLI state lifecycle:
  - explicit TUN apply/rollback path (`--apply-tun`) with `TUNSETIFF` attempt and rollback on failure;
  - explicit route apply/rollback path (`--apply-route`);
  - explicit DNS apply/rollback path (`--apply-dns`).
- Gateway listener gap from earlier snapshots is fixed and not a blocker now.
- Automatic always-on policy-driven system datapath for arbitrary applications is still not closed.

Impact:
- MVP checks now include explicit modified-state runtime apply smokes (DNS/route),
  but fully automatic OS-level datapath behavior for arbitrary applications remains unfulfilled.

### 2) Reporting layer over-asserts readiness for real-world behavior
Observed artifacts repeatedly return `network_state:"not_modified"`, including release-style reports:
- `docs/release_readiness_audit.json`
- `docs/MVP_VERIFY.json`
- `docs/mvp_spec_check_audit.json`
- `docs/REPORT_PACK.json`
- `docs/MVP_SNAPSHOT.json`

This is consistent with safety/simulation mode, but it conflicts with interpreting M4/M5 as fully closed for real system datapath.

## Medium-Severity Findings

### 3) Operations documentation codifies no-network-change runbook
- `docs/OPERATIONS.md:3-5, 42`

This is safe, but it means current official flow validates simulation-oriented checks rather than full system routing behavior expected by strict interpretation of M4/M5 practical usability.

## Passing Areas (Evidence)

### CI/Quality
- `just check`: PASS
- `just test`: PASS (full workspace tests green)
- `just lint`: PASS

### Security/Hardening checks
- `just fuzz-smoke`: PASS
- `just deny`: PASS
- `just net-sim`: PASS
- `just perf-smoke`: PASS

### Lab outputs
- `docs/mvp_spec_check_audit.json`: status `ok`
- `docs/release_readiness_audit.json`: status `ok`
- `docs/MVP_VERIFY.json`: status `ok`

## Truth-First Status (as of this audit)
- M0: mostly satisfied by tooling/tests.
- M1-M3: satisfied in current lab/test sense.
- M4: **partially satisfied** (routing logic/determinism yes; real OS datapath no).
- M5: **partially satisfied** (docs/diagnostics exist; practical app-agnostic datapath not fully satisfied).
- M6: mostly satisfied for current smoke/hardening gates.

## Conclusion
Current project state is strong on simulation/test harness and report generation, but not yet equivalent to fully realized OS-level capture/routing behavior implied by strict practical reading of M4/M5.
