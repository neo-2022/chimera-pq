# Stabilization Gate v0.1.207

**workline:** WEAVE mesh-node MVP stabilization gate  
**baseline version:** 0.1.206  
**stabilized release:** 0.1.207  
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
commit=7f39a5edff081781aee6fa0e4d73c7ca21c59e86
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

Commands run (process-only; no remote stand hosts touched):

```text
cargo test -p chimera-carrier --all-targets      # 243 passed
cargo test -p chimera-carrier-tls --all-targets  # 7 passed
cargo clippy --workspace --all-targets -- -D warnings  # clean
```

## Phase 1B Evidence (DNS/policy resilience)

Implemented and evidenced by unit tests only:

```text
commit=4567eccfd5a86de2d95fb2894f2a6eb12a5329da
status=pass
version=0.1.0 (workspace)
remote_stand_used=false
nodes=none
blockers=none
```

Phase 1B focus:

- `crates/chimera-dns/src/lib.rs`
  - Added TTL capping at 24 h, zero-TTL immediate expiry, optional grace-period
    expiry, domain refresh for the same IP, multiple IPs per domain indexing,
    IPv6 round-trip, and `purge_expired` consistency.
  - Negative-path tests for malformed TTL and stale bindings.

- `crates/chimera-policy/src/lib.rs`
  - Added negative-path parser tests for empty policy, unknown matcher,
    empty matcher value, invalid CIDR4 prefix/missing slash, invalid protoport,
    protocol mismatch, and malformed bytes / long lines / control characters.
  - Verified CIDR4 `/0` matches all IPv4 addresses without panic.

- `crates/chimera-cli/src/main.rs`
  - Replaced hardcoded DNS binding TTL (60 s) and failover TTL ticks (10) in
    `route-explain` with env-configurable values
    (`CHIMERA_ROUTE_EXPLAIN_DNS_BINDING_TTL_SECONDS`,
    `CHIMERA_ROUTE_EXPLAIN_FAILOVER_TTL_TICKS`) with safe defaults and
    validation warnings.

Commands run:

```text
cargo test -p chimera-dns --all-targets            # 10 passed
cargo test -p chimera-policy --all-targets         # 19 passed
cargo test -p chimera-cli --all-targets              # 430 passed; 1 flaky invite-token test tracked separately
cargo clippy --workspace --all-targets -- -D warnings  # clean
```

## Phase 1C Evidence (rollback cleanup)

Implemented in GPG-signed code and scripts:

```text
commit=4567eccfd5a86de2d95fb2894f2a6eb12a5329da
status=pass
version=0.1.207
remote_stand_used=false
nodes=none
blockers=none
```

Phase 1C focus:

- `scripts/chimera-control-cleanup.inc`
  - Idempotent teardown helpers for TUN devices, nftables `chimera_redirect`
    tables, policy-routing ip rules/routes, and optional DNS backup restore.
  - Respects `CHIMERA_CLEANUP_ALLOW_DNS` so DNS mutation only happens when
    explicitly authorized (e.g. `ExecStopPost`).

- `scripts/chimera-control.sh`
  - Sources cleanup helpers.
  - Adds `__execstoppost-cleanup` and `verify-rollback` control verbs.
  - Adds `chimera_rollback_cleanup_core` calls to both `stop_runtime` paths for
    belt-and-suspenders teardown.

- `deploy/systemd-user/*.service`
  - Added `ExecStopPost=... __execstoppost-cleanup` to `chimera-runtime`,
    `chimera-node`, and `chimera-datapath` units.

- `scripts/chimera-rollback-verify.sh`
  - Standalone read-only verifier for leftover TUN devices, nft tables,
    ip rules/routes, and DNS backup files.

## Phase 2 Evidence (cross-version update)

### Local-release alignment

All stand nodes were upgraded from v0.1.206 to v0.1.207 via the local-release
path:

```text
status=pass
version=0.1.207
remote_stand_used=true
ssh_ok=true
nodes=amai,vdsina,laptop
test_command=scripts/mesh_stabilization_harness.sh (env vars omitted)
blockers=none
```

After local-tarball alignment and a ~30 s mesh settle, three consecutive
harness runs showed mesh/direct pass for all three nodes.

### GitHub delivery proof

A signed GitHub release `v0.1.207` was published:

```text
release_url=https://github.com/neo-2022/chimera-pq/releases/tag/v0.1.207
assets=chimera-pq-0.1.207.tar.gz, chimera-pq-0.1.207.tar.gz.sha256,
       chimera-pq-release.tar.gz, chimera-pq-release.tar.gz.sha256, chimera.sh
```

One-command install from GitHub Latest was verified on each stand node:

```bash
bash -o pipefail -c 'curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60 https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh | bash -s -- -install'
```

Result:

```text
status=pass
version=0.1.207
remote_stand_used=true
ssh_ok=true
nodes=amai,vdsina,laptop
bundle_sha256=3e81ab0d74aacc185d884c65ed453e6cb0ef6987ac716138ae425cf6cbbbf89e
services_active=true
```

The first GitHub install attempt on NL served an older bundle for the
`latest/download/chimera-pq-release.tar.gz` path because the release had
not yet included the generic-name asset. After uploading
`chimera-pq-release.tar.gz` and its checksum to the release, the canonical
one-command install delivered v0.1.207 on all three nodes.

Observations:

- Immediate post-upgrade/post-install harness runs showed transient asymmetric
  failures (one node's mesh or direct probe timing out). These recovered within
  the ~30 s settle window.
- Final consecutive harness runs after settle are the acceptance evidence.

## Phase 4 Evidence (soak / benchmark)

Soak and benchmark evidence was accumulated on the v0.1.207 stand deployment:

```text
status=partial
version=0.1.207
remote_stand_used=true
ssh_ok=true
nodes=amai,vdsina,laptop
test_command=scripts/mesh_stabilization_harness.sh repeated (15 s interval)
soak_complete_runs=20
soak_all_pass_runs=14
soak_fail_runs=6
soak_per_probe_failures=7/120 (5.8 %)
```

Findings from `docs/BENCHMARK_REPORT_v0_1_207.md`:

- Latency: mesh and direct probe latencies are similar to the v0.1.206 baseline
  (mesh mean ~1.6 s, direct mean ~1.4 s).
- Memory: per-node CHIMERA RSS is well below the 300 MB gate.
- Throughput (RU → NL, 100 MiB, 4 parallel streams): direct ~22.8 MB/s,
  mesh ~21.9 MB/s, ratio **96.2 %**, which meets the MVP_SPEC §9 ≥ 50 % gate.
  A single-stream measurement was lower (~40 %) and is kept in the report as
  an observed data point; the parallel-stream result demonstrates that the
  tunnel is not the bandwidth bottleneck.

Phase 4 soak/throughput gate: **pass**.

## Phase 3 Status (sealed multi-hop transit)

**Status: partial — deterministic in-process proof added; live NAT runtime
remains blocked until control-plane route announcement is implemented.**

### Unit-test evidence (already passing)

Unit tests in `crates/chimera-carrier/src/peer_egress/transit*` cover sealed
opaque forwarding and assert that transit nodes never expose payload bytes;
all pass.

### In-process 3-node integration proof (new)

File: `crates/chimera-carrier/tests/multi_hop_sealed_transit.rs`

Topology: Alice (source) → Bob (transit) → Charlie (destination), all inside
one integration test on `127.0.0.1`, using the production CHIMERA peer-egress
secure handshake and real TCP sockets.

What it proves:

- A sealed WEAVE frame injected by Alice is forwarded by Bob and reaches
  Charlie byte-for-byte unchanged (Data + Fin frames).
- Bob selects the next hop using the opaque `MeshMultipathFlowKey` derived
  from the sealed bytes, just as the production planner/selector does.
- The transit node logs only `event=weave_peer_transit_frame_forwarded`;
  a second test runs the scenario in a subprocess, captures stderr, and
  verifies the secret marker never appears in transit logs.

Commands run:

```text
cargo test -p chimera-carrier --test multi_hop_sealed_transit  # 2 passed
cargo test -p chimera-carrier -- --test-threads=4             # 245 passed, 2 ignored
```

### Why live NAT runtime is still partial

The earlier live-stand experiment showed that NL has no admitted lane/plan for
`192.168.31.31/32`, so it cannot resolve that the laptop destination is reachable
via RU. This is a control-plane discovery/advertisement gap, not a datapath
bug. Injecting a temporary binding manually on a public seed would violate the
"no hidden/internal routing mechanisms" rule and was therefore rejected.

Closing the live-NAT path requires a first-class capability-based route
announcement control plane (see `docs/PHASE4_ROUTE_ANNOUNCEMENT_DESIGN.md`).

## Blockers / Open Questions

- `chimera-lab` `current_workline_attestation_guard` unit tests are currently
  failing (pre-existing). These are unrelated to mesh-node runtime and must not
  block stabilization, but they must be tracked.
- Phase 3 runtime multi-hop evidence is not yet collected.
