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

## Phase 1A Evidence (dead-peer selection, churn, reconnect)

Implemented in GPG-signed commit(s) and evidenced by unit tests only:

```text
commit=b18d1e755c5a4d331df3fcb14ffca63721cce565
status=pass
version=0.1.0 (workspace)
remote_stand_used=false
nodes=none
blockers=none
```

Phase 1A focus:

- `crates/chimera-carrier/src/peer_egress/modes_local_ingress.rs`
  - Extended the lane-document local-ingress retry loop: after the flow-key
    selected lane fails handshake, the same lane is retried up to 3 attempts
    (fresh peers may arrive), then the code falls back to another admitted
    active lane from `plan.multipath_schedule`.  Redacted logging and the
    existing deadline are preserved; no panics on malformed input.
  - Added `active_fallback_bindings` helper to enumerate deterministic active
    lane fallbacks.

- `crates/chimera-carrier/src/peer_egress/modes_local_ingress_tests.rs`
  - `lane_document_retries_same_binding_when_fresh_peer_arrives`
  - `lane_document_fallbacks_to_other_active_lane_when_first_peer_is_dead`
  - `peer_pool_discards_dead_peer_and_does_not_retry_same_stream`

- `crates/chimera-carrier-tls/src/lib.rs`
  - Added TCP/TLS carrier reconnect/backoff (≤5 s deadline), redacted
    `event=tls_carrier_reconnect` and `event=tls_carrier_stream_dropped` log
    lines, `set_connect_addr` for listener endpoint changes, and
    `with_reconnect_max_wait_ms` for tests.
  - Added regression tests:
    - `tls_carrier_reconnects_within_deadline_after_server_late_bind`
    - `tls_carrier_reconnects_after_disconnect_and_endpoint_change`

Commands run (this is process-only evidence; no remote stand hosts touched):

```text
cargo test -p chimera-carrier --all-targets      # 239 passed
cargo test -p chimera-carrier-tls --all-targets  # 7 passed
cargo clippy --workspace --all-targets -- -D warnings  # clean
```

Status: Phase 1A process-level gates pass.  Remote-stand runtime evidence for
§4 M3/M6 still pending (SSH-only, will be accumulated in next turns).

## Blockers / Open Questions

- `chimera-lab` `current_workline_attestation_guard` unit tests are currently
  failing (pre-existing). These are unrelated to mesh-node runtime and must not
  block stabilization, but they must be tracked.
- Long soak tests require background sessions; evidence must be accumulated over
  multiple agent turns.
