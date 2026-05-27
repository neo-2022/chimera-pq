# CHIMERA-PQ Post-MVP Product Roadmap

Status: active roadmap (not MVP execution scope)

## 0. Full Stage Map (MVP -> Post-MVP)

This roadmap now includes the full path so execution can be tracked end-to-end.

### 0.1 MVP Stages (authoritative in spec)

Source of truth:
- `../../CHIMERA-PQ_MVP_SPEC.md` (milestones M0-M6 and acceptance criteria)

MVP stages:
1. M0 Repository and test harness
2. M1 Local prototype tunnel
3. M2 Secure session and crypto policy
4. M3 First real carrier
5. M4 Capture and routing
6. M5 Practical VPN usability
7. M6 Hardening gate

### 0.2 Post-MVP Stages (this roadmap)

1. P1 Mesh registry/discovery contract as product control-plane
2. P2 Desktop UI shell over runtime API
3. P3 Android client surface
4. P4 Ads integration in UI layer only

## 0.3 Stage-to-Doc Links

- Global execution mode (no timelines): `EXECUTION_MODE_NO_TIMELINES.md`
- MVP scope and acceptance: `../../CHIMERA-PQ_MVP_SPEC.md`
- Architecture baseline: `ARCHITECTURE.md`
- Operations baseline: `OPERATIONS.md`
- Security baseline: `SECURITY.md`
- Privacy baseline: `PRIVACY.md`
- Mesh launch gate context: `MESH_FIRST_LAUNCH_EXECUTION_GATE.md`
- Session continuity/handoff evidence: `MESH_SESSION_HANDOFF_*.md`
- Full docs inventory (all docs/*.md + docs/*.txt): `ROADMAP_DOC_INDEX.md`
- MVP close/readiness artifacts:
  - `RELEASE_READINESS_REPORT.md`
  - `SHIP_READINESS_REPORT.md`
  - `FINAL_M5_M6_REPORT.md`

## 0.4 Audit Basis (Truth-First)

Roadmap structure is based on:
- root governance docs: `../../AGENTS.md`, `../../CHIMERA-PQ_MVP_SPEC.md`,
  `../../Agent.md`, `../../README.md`;
- execution rule doc: `EXECUTION_MODE_NO_TIMELINES.md`;
- full current docs inventory listed in `ROADMAP_DOC_INDEX.md`
  (generated from all `docs/*.md` and `docs/*.txt` files).

## 1. Scope Boundary

This document captures product-surface goals requested by the user and keeps
them explicitly out of current CHIMERA-PQ VPN MVP execution gates.

MVP remains defined by `CHIMERA-PQ_MVP_SPEC.md`:
- Linux-first VPN core
- secure tunnel + carrier + policy routing + DNS binding
- diagnostics/explain/tests/rollback safety

The items below are post-MVP unless the user explicitly reprioritizes.

## 2. Product Surfaces After MVP

### 2.1 Desktop UI Shell

Target:
- graphical shell over the same Rust runtime core
- node selection by country
- node status visibility (reachable/health/score)
- pin/unpin/autoconnect controls
- explain diagnostics in UI

Constraint:
- UI must consume stable runtime API contracts, not duplicate networking logic.

### 2.2 Android Client

Target:
- Android application that reuses the same protocol/policy model
- country-based node selection and diagnostics parity with desktop/CLI

Constraint:
- mobile UI is a client surface; tunnel/policy/security logic remains in core
  architecture and shared protocol contracts.

## 3. Mesh Portal / Virtual Resource Direction

Target:
- always-on mesh-visible virtual resource containing:
  - available node registry
  - operator-defined additional info

Baseline security requirements:
- signed updates
- monotonic versions/epochs
- TTL expiry
- anti-replay guard
- fallback replicas

Availability goal:
- resource stays available as long as at least one CHIMERA node with valid
  replicated state remains online.

## 4. Ads and Monetization Guardrails

Monetization is accepted as post-MVP product track with strict separation:

- ads are allowed only in UI/application layer surfaces;
- ads are forbidden in VPN tunnel, policy, routing, capture, crypto, and
  datapath logic;
- ad components must not receive raw destination logs, packet payloads, keys,
  or sensitive runtime secrets;
- privacy controls and redaction defaults remain mandatory.

This separation is a hard architecture rule for future implementation.

Operator requirements закреплены:
- mesh visibility depth target: `2 hops` (default privacy bound);
- owner node has always-on access to ads control plane (campaign/rules toggles);
- global ads kill-switch must exist and must fully disable ads rendering and ads
  control actions at runtime.

Planned config surface (post-MVP P4):
- `ad_control_owner_node_id=<node-id>`;
- `ad_control_always_on=true|false`;
- `ads_enabled=true|false` (global kill-switch);
- `ad_data_scope=redacted_only` (hard default).
- `ads_subscription_disable=true|false` (auto-disable ads when paid subscription is active).

Subscription rule (future-proof):
- when user subscription state is `active`, ads must be disabled regardless of
  campaign toggles;
- subscription-based ads disable has higher priority than campaign delivery
  rules;
- if subscription state is unknown/unavailable, runtime falls back to explicit
  `ads_enabled` policy.

## 5. Execution Order

Readiness order after MVP:
1. stable mesh node registry/discovery contract
2. desktop UI shell
3. Android client
4. ads module integration in UI only

No timeline deadlines are implied; progression is gate/evidence based.

## 6. Current Tracking View

Execution status values:
- `not started`
- `in progress`
- `partial`
- `done`

### 6.1 MVP Tracking (reference)

- M0-M6 status must be taken from evidence against `../../CHIMERA-PQ_MVP_SPEC.md`
  and current artifact reports in `docs/`.

### 6.2 Post-MVP Tracking

1. P1 Mesh registry/discovery contract: `in progress (partial implemented in CLI/runtime contracts)`
2. P2 Desktop UI shell: `not started`
3. P3 Android client: `not started`
4. P4 Ads in UI layer only: `not started`

## 7. Stage Evidence Matrix (Required)

Stage-close rule:
- no stage can be marked `done` without matching acceptance source and proof
  bundle evidence.

### 7.1 MVP (M0-M6)

1. `M0`  
Acceptance docs:
- `../../CHIMERA-PQ_MVP_SPEC.md` (M0 acceptance)
Proof docs:
- `MVP_VERIFY.txt`
- `MVP_SNAPSHOT.txt`
- `MVP_SPEC_COVERAGE.md`

2. `M1`  
Acceptance docs:
- `../../CHIMERA-PQ_MVP_SPEC.md` (M1 acceptance)
Proof docs:
- `MESH_ROUTE_EXPLAIN.json`
- `MESH_RUNTIME_TRACE.json`
- `RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json`

3. `M2`  
Acceptance docs:
- `../../CHIMERA-PQ_MVP_SPEC.md` (M2 acceptance)
Proof docs:
- `SECURITY.md`
- `THREAT_MODEL.md`
- `RUNTIME_POLICY_PRECEDENCE_SMOKE.json`

4. `M3`  
Acceptance docs:
- `../../CHIMERA-PQ_MVP_SPEC.md` (M3 acceptance)
Proof docs:
- `CHIMERA_REMOTE_CYCLE_SMOKE_REPORT_2026-05-22_15-41-21.md`
- `CHIMERA_BIDIRECTIONAL_E2E_REPORT_2026-05-22_15-48-38.md`
- `CHIMERA_BIDIRECTIONAL_E2E_SMOKE_2026-05-22_16-12-24.md`

5. `M4`  
Acceptance docs:
- `../../CHIMERA-PQ_MVP_SPEC.md` (M4 acceptance)
Proof docs:
- `ROUTING.md`
- `route_explain_latest.json`
- `rollback_status_latest.json`

6. `M5`  
Acceptance docs:
- `../../CHIMERA-PQ_MVP_SPEC.md` (M5 acceptance)
Proof docs:
- `M5_ARTIFACTS_REPORT.md`
- `PROBE_ACCESS.md`
- `CHIMERA_MESH_LOAD_REPORT_2026-05-22.md`

7. `M6`  
Acceptance docs:
- `../../CHIMERA-PQ_MVP_SPEC.md` (M6 acceptance)
Proof docs:
- `M6_ARTIFACTS_REPORT.md`
- `QA_VALIDATION_ACCEPTANCE_PLAN.md`
- `FINAL_M5_M6_REPORT.md`

### 7.2 Post-MVP (P1-P4)

1. `P1` Mesh registry/discovery contract  
Acceptance docs:
- `POST_MVP_PRODUCT_ROADMAP.md` (sections 3, 5, 6, 7)
- `SECURITY.md`
- `PRIVACY.md`
Proof docs (required when stage is implemented):
- signed registry schema/spec doc (new)
- anti-replay and TTL validation test report (new)
- replica/fallback resilience report (new)
- key rotation + revoke + re-enroll flow spec (new)
- restricted-mode and bootstrap-sync recovery test report (new)

Current implementation evidence (2026-05-22, partial):
- `crates/chimera-cli/src/mesh_cli/nodes_inventory.rs`
  - signed discovery envelope checks (`contract_version`, TTL, nonce anti-replay, `key_id`, signature)
  - revoked key and revoked node filters
  - restricted mode + identity/activation/runtime-state overlays
- `crates/chimera-cli/src/mesh_cli/nodes_cmd.rs`
  - `re-enroll`, `re-enroll-prepare`, `re-enroll-submit`
  - `probe --all` bound to real `connect_probe` backend
  - runtime state persistence (`current/pinned/autoconnect`)
  - JSON success/error contracts for `probe --all` and `state`
- tests:
  - `crates/chimera-cli/src/mesh_cli/tests_nodes_inventory.rs`
  - `crates/chimera-cli/src/mesh_cli/tests_nodes_reenroll.rs`
  - `crates/chimera-cli/src/mesh_cli/tests_nodes_runtime_state.rs`
  - proof hardening coverage includes:
    - key-id mismatch/missing checks
    - ttl/future/expired challenge checks
    - nonce anti-replay check (`guard_replay_nonce`)
    - JSON error contract snapshot for `stage=proof_verify`

2. `P2` Desktop UI shell  
Acceptance docs:
- `POST_MVP_PRODUCT_ROADMAP.md` (section 2.1)
- `OPERATIONS.md`
Proof docs (required when stage is implemented):
- desktop UI functional acceptance report (new)
- API compatibility matrix with CLI/runtime (new)

3. `P3` Android client  
Acceptance docs:
- `POST_MVP_PRODUCT_ROADMAP.md` (section 2.2)
- `PRIVACY.md`
Proof docs (required when stage is implemented):
- android functional acceptance report (new)
- protocol/policy parity report vs desktop/CLI (new)

4. `P4` Ads in UI layer only  
Acceptance docs:
- `POST_MVP_PRODUCT_ROADMAP.md` (section 4)
- `PRIVACY.md`
- `SECURITY.md`
Proof docs (required when stage is implemented):
- ads isolation audit (no datapath/tunnel coupling) (new)
- privacy redaction and data-flow audit for ads module (new)
- owner-node always-on control verification report (new)
- global ads kill-switch verification report (new)
- paid-subscription ads-disable precedence verification report (new)

## 8. Key Revocation Recovery Model (P1)

Required behavior for long-offline clients and revoked keys:

1. Startup bootstrap sync:
- client first syncs trust metadata (registry, revocations, trusted key set)
  before normal mesh participation.

2. Revoked key handling:
- revoked key must not block app startup/update paths;
- revoked identity enters `restricted` mode (no full mesh routing).

3. Re-enroll flow:
- client can generate/request new keypair and re-register node identity;
- after successful re-enroll and trust acceptance, full mesh mode is restored.

4. Safety priority:
- update/sync/re-enroll paths remain available even while mesh routing is
  restricted.
