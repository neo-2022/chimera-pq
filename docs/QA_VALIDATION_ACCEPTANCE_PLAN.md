# CHIMERA-PQ QA Validation & Acceptance Plan (MVP)

Status: draft plan (not execution report)
Owner role: QA/Validation Lead
Scope source: `CHIMERA-PQ_MVP_SPEC.md` + project gates from `AGENTS.md`
Execution mode: readiness-based, no calendar deadlines (`docs/EXECUTION_MODE_NO_TIMELINES.md`)

## 1. PASS Model: Lab vs Real-World

`Lab PASS` and `Real-World PASS` are separate and both are required for ship.

- Lab PASS: correctness, security invariants, determinism, parser/fuzz stability, rollback behavior in controlled environment.
- Real-World PASS: behavior on real Linux hosts and real network conditions with operator workflow, without breaking host networking.

Hard rule:
- `Lab PASS` cannot be promoted to `Real-World PASS` automatically.
- Any stage that changes system behavior (TUN, routing, DNS, rollback) is `Real-World required`.

## 2. Acceptance Gates

Gate IDs and release dependency:

1. `G0` Build & Static Quality Gate (Lab)
2. `G1` Crypto/Session Correctness Gate (Lab)
3. `G2` Carrier Resilience Gate (Lab)
4. `G3` Routing/DNS Decision Correctness Gate (Lab)
5. `G4` Safety Rollback Gate (Lab + Real-World)
6. `G5` Operator Diagnostics Gate (Lab + Real-World)
7. `G6` Performance Baseline Gate (Lab + Real-World)
8. `G7` Security Hardening Gate (Lab)
9. `G8` End-to-End Usability Gate (Real-World)
10. `G9` Release Readiness Gate (Evidence completeness)

Ship requires `G0..G9 = PASS`.

## 3. Exact Test Matrix

## 3.1 Lab PASS Matrix

| Gate | Test ID | Objective | Method/Command Bundle | PASS Criteria |
|---|---|---|---|---|
| G0 | L-G0-01 | Workspace health | `just check && just test && just fmt && just lint` | all commands exit 0 |
| G0 | L-G0-02 | No panic on malformed config | negative config corpus + unit/integration tests | no panic, structured error |
| G1 | L-G1-01 | Encrypted frame exchange | client+gateway integration over fake carrier | encrypted exchange succeeds |
| G1 | L-G1-02 | Replay rejection | replay same packet number | replay rejected |
| G1 | L-G1-03 | Tamper rejection | mutate ciphertext/tag/header | packet rejected |
| G1 | L-G1-04 | Downgrade rejection | suite downgrade simulation | downgrade rejected |
| G1 | L-G1-05 | Rekey trigger correctness | packet/time threshold tests | rekey occurs at configured threshold |
| G2 | L-G2-01 | Carrier connect/reconnect | disconnect simulation in TLS/TCP + QUIC tests | reconnect succeeds per policy |
| G2 | L-G2-02 | Probe handling privacy | invalid probe before session | no CHIMERA identity leak |
| G3 | L-G3-01 | Policy precedence | exact domain vs suffix, CIDR, port/proto matrix | precedence matches spec |
| G3 | L-G3-02 | DNS-to-route binding | resolve domain then route by bound IP context | bound decision preserved |
| G3 | L-G3-03 | Route explain trace | `chimera route explain ...` contract tests | matched rule + reason shown |
| G4 | L-G4-01 | Rollback on normal down | `chimera up` then `chimera down` lab harness | state restored |
| G4 | L-G4-02 | Rollback on forced stop/crash | terminate client/service mid-session | rollback completes |
| G5 | L-G5-01 | Diagnostics redaction | doctor/export log snapshot tests | no secrets/raw sensitive fields |
| G5 | L-G5-02 | Error contract stability | JSON envelope contract tests | schema + stage/action stable |
| G6 | L-G6-01 | Throughput baseline | benchmark script vs direct baseline | regression within budget |
| G6 | L-G6-02 | Latency overhead baseline | benchmark p50/p95 delta capture | within agreed threshold |
| G7 | L-G7-01 | Parser fuzz smoke | fuzz config/frame/handshake parsers | no panic/crash |
| G7 | L-G7-02 | Route determinism property | property tests same input => same decision | deterministic |
| G7 | L-G7-03 | Packet number properties | monotonicity/reuse rejection properties | invariant holds |
| G9 | L-G9-01 | Artifact completeness | manifest check over required artifacts list | no missing artifact |

## 3.2 Real-World PASS Matrix

| Gate | Test ID | Objective | Environment | PASS Criteria |
|---|---|---|---|---|
| G4 | R-G4-01 | Real host rollback safety | clean Linux client host | original route/DNS state restored after down |
| G4 | R-G4-02 | Crash rollback safety | forced stop on real host | auto-recovery/rollback successful |
| G5 | R-G5-01 | Operator diagnosis usefulness | real misconfig scenarios | failure reason actionable, no secret leakage |
| G6 | R-G6-01 | Real network performance | client<->gateway over real network | meets min throughput/max latency budget |
| G6 | R-G6-02 | Packet loss/delay resilience | netem or realistic unstable path | session survives/reconnects per policy |
| G8 | R-G8-01 | Clean machine first-run | fresh Ubuntu-like machine | user reaches gateway using docs only |
| G8 | R-G8-02 | Policy split behavior | mixed direct + tunneled routes | direct remains direct, tunneled goes tunnel |
| G8 | R-G8-03 | Browser usability | common sites via configured policy | pages open successfully |
| G8 | R-G8-04 | IDE/proxy workflow sanity | VS Code/proxy capture notes path | workflow operates as documented |
| G8 | R-G8-05 | Multi-run stability | repeated up/down cycles (>=10) | no residual route/DNS corruption |
| G9 | R-G9-01 | Release candidate installability | package/binary install on target | no cargo dependency required |

## 4. Required Evidence Artifacts Per Gate

Minimum artifact set (all required for PASS claim):

- `E-G0`: build/test/lint/fmt logs with commit SHA and UTC timestamp.
- `E-G1`: session/crypto test report including replay/tamper/downgrade/rekey cases.
- `E-G2`: carrier resilience report with disconnect/reconnect traces.
- `E-G3`: routing policy matrix output + route-explain JSON snapshots.
- `E-G4-L`: lab rollback artifacts (`runtime_state`, rollback status before/after).
- `E-G4-R`: real-host rollback snapshots (before/after route table + DNS resolver state).
- `E-G5`: doctor/diag export samples proving redaction and actionable failure stage/action.
- `E-G6-L`: lab benchmark raw JSON + baseline comparison report.
- `E-G6-R`: real-world benchmark runbook + raw metrics + host/network profile.
- `E-G7`: fuzz smoke summary, failing seeds (if any), property-test log.
- `E-G8`: clean-machine run transcript (install, up, route behavior, down), operator notes.
- `E-G9`: release readiness manifest mapping each gate to artifact file paths.

Evidence quality constraints:

- Every artifact includes: commit SHA, test ID, date/time, environment descriptor.
- Redaction mandatory for secrets and sensitive identifiers.
- PASS without artifact link is invalid.

## 5. Gate Promotion Rules (Lab -> Real-World)

A gate can be promoted only when both conditions hold:

1. Lab gate PASS evidence exists and is complete.
2. Matching real-world gate PASS evidence exists where required.

Required real-world gates: `G4`, `G5`, `G6`, `G8`, `G9`.

## 6. Stop-Ship Criteria (Hard Fail)

Any single condition below is `STOP-SHIP`:

1. Any gate `G0..G9` is FAIL or missing evidence.
2. Any unhandled panic in parser/session/carrier/runtime during acceptance bundle.
3. Replay, tamper, or downgrade rejection fails at least once.
4. Rollback fails to restore host networking in any real-world rollback test.
5. Diagnostics/log export leaks secrets/raw destination details by default.
6. Route determinism fails for identical inputs.
7. Performance regression exceeds approved budget and no signed exception exists.
8. Clean-machine install/run requires undeclared local dependencies (including cargo on target).
9. Security checklist/threat-model acceptance is incomplete.
10. Release readiness manifest does not map all claimed PASS gates to concrete artifacts.

## 7. Pass/Fail Report Template (Mandatory)

For each gate report:

- Status: `pass | fail | partial`
- Evidence: exact artifact paths + command/log references
- Unclosed items: what remains open
- Risks/limits: residual risk and production impact

Final release verdict must include:

- `Lab PASS`: pass/fail with gate list
- `Real-World PASS`: pass/fail with gate list
- `Stop-Ship`: clear `triggered/not triggered` with criteria ID(s)

## 8. Execution Notes

- This document is a validation plan, not evidence of completion.
- Any statement `done/closed/pass` is forbidden until artifacts exist.
- Post-MVP scopes (mesh/DHT/relay economy/etc.) are excluded from MVP ship gates.
