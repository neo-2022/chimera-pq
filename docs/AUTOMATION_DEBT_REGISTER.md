# CHIMERA Automation Debt Register

Status: active (blocking guard)
Updated: 2026-05-22

Purpose:
- record every operation that still required manual intervention;
- fail automation guard while any debt item remains unresolved;
- prevent future "works only by hand" regressions.

## Active Debt Items

1. `DEBT-001`  
Area: `egress/autonomy`  
Problem: geo-bypass on VPS has no autonomous multi-region egress path without operator-provided upstream credentials/endpoints.  
Current impact: blocked resources on VPS stay blocked in `full` and `split` modes.  
Expected autonomous behavior: CHIMERA must auto-select working egress path (no manual upstream provisioning), then keep it healthy via watchdog/failover.  
Status: `open`

2. `DEBT-002`  
Area: `nat acceptance`  
Problem: end-to-end nat verification is not one-command deterministic for both laptop+VPS in clean environments.  
Current impact: operator has to chain several scripts and environment preparations manually.  
Expected autonomous behavior: single command runs uninstall/install/start/full/split verification and emits machine-readable PASS/FAIL with blockers.  
Status: `closed`  
Evidence:
- `scripts/chimera_autonomous_nat_guard.sh`
- `just chimera-autonomous-nat-guard`

## Close Rule

Debt item can be moved to `closed` only when all are true:
- autonomous command exists in repository scripts/justfile;
- proof artifact is reproducible on laptop and VPS;
- no manual parameter editing was required during proof cycle.
