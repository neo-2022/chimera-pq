# Stabilization Gate v0.1.206

**workline:** WEAVE mesh-node MVP stabilization gate  
**baseline version:** 0.1.206  
**target release after gate:** 0.1.207+  
**status:** in_progress

## Purpose

This document is the acceptance contract for the stabilization work ordered in
`CHIMERA-PQ_MVP_SPEC.md` §9/§11. It lists the gates that must be evidenced before
the MVP can be considered stable and before any post-MVP discovery/back-end
mechanism (DHT, invite, economy) is implemented.

## Scope

Only the remote stand may execute runtime proofs:

- `<nl-stand-node>` — node_id `amai`, public seed / publisher
- `<ru-stand-node>` — node_id `vdsina`, public seed / publisher
- `<laptop-stand-node>` — node_id `laptop`, NAT-based non-publishing member

PC remains control-only. Stand addresses are never hardcoded in product files.

## Gate Maps to MVP Spec

| MVP Spec section | Gate | Acceptance |
|---|---|---|
| §4 M3 | Carrier reconnect | after simulated carrier drop, mesh datapath recovers within 5 s |
| §4 M4 | Rollback safety | `chimera.sh -stop` and forced `SIGKILL` of the node leave OS routes/firewall/DNS in original state |
| §4 M5 | Split-mode failover | unreachable direct resource is switched to CHIMERA path; unrelated traffic stays direct |
| §4 M5 | Clean setup | one-command install/start on clean Ubuntu 24.04/26.04 without manual tunnel/proxy deps |
| §4 M6 | Negative-path parsers | fuzz/negative tests: no panic on malformed config, frame, handshake, route input |
| §7 | DNS binding | domain→IP binding updates after TTL; stale binding does not persist beyond TTL+grace |
| §9 | Throughput | mesh throughput ≥ 50 % of direct baseline on the same stand link |
| §9 | Reconnect | < 5 s reconnect after carrier drop |
| §9 | Memory | node typical memory < 300 MB during soak |
| §9 | CPU | < 1 core at 100 Mbps mesh throughput |
| §11 | Release gate | clean clone builds; encrypted tunnel carries real traffic; policy routing works; shutdown restores network state; security tests pass; fuzz smoke passes; no raw secrets in logs; benchmark report exists; operations guide exists |

## Additional Stability Gates (Beyond MVP Spec)

- **Dead-peer repath**: if the peer selected for a flow is dead, the local ingress path retries other admitted lanes before failing.
- **NAT flap**: when a peer's public endpoint changes, discovery update rebinds without operator intervention.
- **Cross-version update**: upgrading any stand node to a newer signed release does not partition the mesh; rolling back that node also works.
- **Sealed multi-hop transit**: a transit node forwards sealed opaque bytes and does not log or inspect the payload.

## Evidence Format

Every gate must be evidenced by one of:

1. Automated shell harness output saved to `docs/STABILIZATION_EVIDENCE_<gate>_<date>.json`
2. Commit hash and test command
3. Manual operator log with timestamps (only if automation is not yet available)

Required evidence fields:

```text
status=pass|partial|fail
version=<tested-release>
remote_stand_used=true
ssh_ok=true
nodes=<comma-separated-node-ids>
test_command=<exact command>
result_summary=<counts or boolean>
blockers=<empty or list>
```

## Blockers / Open Questions

- `chimera-lab` `current_workline_attestation_guard` unit tests are currently
  failing (pre-existing). These are unrelated to mesh-node runtime and must not
  block stabilization, but they must be tracked.
- Long soak tests require background sessions; evidence must be accumulated over
  multiple agent turns.
