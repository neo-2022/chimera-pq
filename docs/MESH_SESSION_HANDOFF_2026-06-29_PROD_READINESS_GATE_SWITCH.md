# CHIMERA Mesh Session Handoff: Prod-Readiness Gate Switch

## Saved At

- Timestamp: 2026-06-29T22:05:00Z

## Active Objective

- Stop continuing local metadata/perf micro-slices by inertia.
- Switch CHIMERA-PQ / WEAVE mesh-node MVP work to the remote
  release/runtime gate.
- Keep transit payload opaque/sealed and untouched.
- Keep local PC network state untouched; real runtime checks happen only on the
  SSH stand.

## User Directive Captured

- User rejected slow micro-step execution toward CHIMERA production readiness.
- User rejected shallow critic output as a decision basis.
- Rule recorded in `AGENTS.md` section `24.8. Prod-Readiness First После
  Perf-Slices`.

## Council Consensus

Roles consulted through real sub-agents:

- Architect: current active objective must be remote real-runtime/release gate,
  not another metadata/perf slice.
- Senior developer: replace metadata micro-optimizations with release/runtime,
  mesh convergence, datapath transit, diagnostics and SLO packages.
- QA: prod-readiness proof bundle needs real install/update, start/stop/restart,
  join, ingress/egress/transit, reconnect/failover, DNS/route/TUN, rollback,
  diagnostics, perf, secret/hardcode and redaction checks.
- Security: no prod-ready claim without remote runtime proof, secret/hardcode
  scan, negative-path parser/session/config checks, bounded buffers and opaque
  transit evidence.
- DevOps: closest goal is release artifact install without `cargo`, checksum and
  version verification, user-service lifecycle, reconnect, rollback and
  redacted diagnostics evidence.

## Next Gates

1. Remote release/runtime gate:
   - build release artifact;
   - verify checksum/version;
   - install/update on remote stand without `cargo`;
   - start/stop/restart user-service;
   - collect redacted diagnostics evidence.
2. Mesh convergence gate:
   - seed/invite discovery;
   - peer-table lifecycle;
   - lane/binding/route choice;
   - route rebuild after peer/carrier loss;
   - state publish.
3. Datapath transit gate:
   - local ingress;
   - peer ingress;
   - local egress;
   - peer transit of sealed payload;
   - no payload logging/inspection.
4. Rollback/failover gate:
   - forced carrier failure;
   - reconnect/rebind;
   - crash/forced-stop recovery;
   - rollback to previous binary/config.
5. Hardening proof bundle:
   - workspace tests/clippy;
   - fuzz/parser smoke;
   - no-hardcode and secret/redaction guards;
   - perf budget for CPU/RAM/latency/convergence;
   - anti-monolith check.

## Evidence Fields Allowed

- `remote_stand_used`
- `ssh_ok`
- `artifact_checksum_ok`
- `version_ok`
- `install_without_cargo_ok`
- `config_validate_ok`
- `start_ok`
- `status_ok`
- `restart_ok`
- `stop_ok`
- `join_ok`
- `peer_count_ok`
- `route_binding_ok`
- `transit_opaque_ok`
- `reconnect_ok`
- `rollback_ok`
- `diagnostics_redacted_ok`
- `perf_budget_ok`

## Not Allowed In Public Evidence

- stand IPs, hostnames, usernames, home paths or private ports;
- passwords, tokens, private keys, invite secrets or session keys;
- packet payloads, packet dumps, plaintext DNS or full configs;
- unredacted local route/firewall/DNS state.

## Not Closed

- Prod-ready MVP is not claimed.
- Practical VPN usability is not claimed.
- Transparent app access without proxy is not claimed.
- One-command install/update is not claimed.
- Carrier reconnect/retry and rollback are not yet verified in this switch
  record.

## Next Step

- Start the remote release/runtime gate: inspect existing release/install scripts
  and proof commands, then run the allowed SSH-only stand checks with redacted
  evidence.
