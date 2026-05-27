# MESH First Launch Execution Gate

Status: closed on 2026-05-21
Owner: CHIMERA-PQ mesh workline

## Objective (strict)

Single active objective until closure:
- first real CHIMERA mesh launch between two hosts (VPS + laptop) with factual connectivity confirmation.

No parallel objective is allowed if it does not directly unblock this launch.

## Definition of Done (DoD)

The objective is closed only when all conditions are evidenced:
1. mesh runtime is started on both peers with valid config;
2. peers discover/select/connect through runtime path (not explain-only simulation);
3. at least one real end-to-end traffic probe through selected mesh path succeeds;
4. diagnostics show selected peer/path and no contradictory error stage;
5. rollback/down path remains safe after launch test.

If any condition is not proven, status is not closed.

## Hard Execution Rules

1. No “micro-progress” updates as substitute for launch progress.
2. Any change must be mapped to a concrete launch blocker.
3. Test-only or contract-only tasks are allowed only when they unblock a current launch blocker.
4. If a task is not launch-critical, move it to backlog and do not execute now.
5. Every status update must include:
   - current blocker;
   - action in progress;
   - evidence produced.

## Close Report (2026-05-21)

Status: completed

DoD evidence matrix:

1. mesh runtime started on both peers with valid config:
   - `docs/MESH_LAUNCH_PREFLIGHT_VPS.json` -> `status=ready`, `ready_for_real_launch=true`
   - `docs/MESH_LAUNCH_PREFLIGHT_LAPTOP.json` -> `status=ready`, `ready_for_real_launch=true`

2. peers discover/select/connect through runtime path:
   - `docs/MESH_LAUNCH_PREFLIGHT_VPS.json` -> `connect_probe_success=true`, non-empty selected peer
   - `docs/MESH_LAUNCH_PREFLIGHT_LAPTOP.json` -> `connect_probe_success=true`, non-empty selected peer
   - `docs/MESH_LAUNCH_PREFLIGHT_VERIFY.json` -> `status=ready`, `all_ready=true`

3. at least one real end-to-end traffic probe through selected path succeeds:
   - `docs/CHIMERA_PATH_PROOF.json` -> `status=pass`, `reason=distinct_path_ip`, `totals.passed=2`

4. diagnostics show selected peer/path and no contradictory error stage:
   - `docs/CHIMERA_CHANNEL_AUDIT.json` -> `status=pass`, `reason=channel_separation_observed`
   - `docs/CHIMERA_E2E_CHANNEL_GATE.json` -> `status=pass`, `reason=channel_audit_and_selected_routes_ok`

5. rollback/down path remains safe after launch test:
   - `scripts/chimera-control.sh stop` -> listener down (`chimera_proxy_listener=down`)
   - `scripts/chimera-control.sh start` -> listener restored (`chimera_proxy_listener=up`)
   - e2e/report artifacts remain `network_state=not_modified`

## Operator Runbook (Side A <-> Side B)

Goal of this runbook:
- produce deterministic `mesh launch-preflight` JSON artifacts on both peers;
- confirm `ready_for_real_launch=true` before first real traffic probe.

Notes:
- this preflight does not modify OS routes/DNS/firewall (`network_state=not_modified`);
- run commands from repo root: `/home/art/Archives/VPN/chimera-pq`.

### 1) Prepare peer specs

Define two peer records (example format):
- Side A peer: `node-a@<SIDE_A_IP>:<PORT>@eu@20@90`
- Side B peer: `node-b@<SIDE_B_IP>:<PORT>@eu@25@85`

Use real reachable IP/port values for your environment.

### 2) Run preflight on Side A

```bash
cargo run -p chimera-cli -- mesh launch-preflight \
  --namespace cef-public \
  --node node-a \
  --traffic-profile high_speed_anonymous \
  --peer "node-b@<SIDE_B_IP>:<PORT>@eu@25@85" \
  --timeout-ms 1200 \
  --json \
  --out docs/MESH_LAUNCH_PREFLIGHT_VPS.json
```

### 3) Run preflight on Side B

```bash
cargo run -p chimera-cli -- mesh launch-preflight \
  --namespace cef-public \
  --node node-b \
  --traffic-profile high_speed_anonymous \
  --peer "node-a@<SIDE_A_IP>:<PORT>@eu@20@90" \
  --timeout-ms 1200 \
  --json \
  --out docs/MESH_LAUNCH_PREFLIGHT_LAPTOP.json
```

### 4) Interpret result

`ready` means:
- `status="ready"`
- `ready_for_real_launch=true`
- `connect_probe_success=true`
- `blockers=[]`

`blocked` means:
- `status="blocked"`
- `ready_for_real_launch=false`
- `connect_probe_success=false`
- `blockers` contains at least `connectivity_probe_failed`

### 5) Closure condition for this gate

This gate can move to next step only when both artifacts
(`MESH_LAUNCH_PREFLIGHT_VPS.json` and `MESH_LAUNCH_PREFLIGHT_LAPTOP.json`)
show `status="ready"` with no blockers.

### 6) Machine verify (recommended)

```bash
cargo run -p chimera-cli -- mesh launch-preflight-verify \
  --vps-report docs/MESH_LAUNCH_PREFLIGHT_VPS.json \
  --laptop-report docs/MESH_LAUNCH_PREFLIGHT_LAPTOP.json \
  --json \
  --out docs/MESH_LAUNCH_PREFLIGHT_VERIFY.json
```

Expected:
- exit code `0`;
- `docs/MESH_LAUNCH_PREFLIGHT_VERIFY.json` has:
  - `status="ready"`
  - `all_ready=true`
  - `vps_ready=true` (maps to Side A report path)
  - `laptop_ready=true` (maps to Side B report path)
  - same non-empty `namespace` on both source reports (mismatch blocks readiness)

### 7) One-command wrapper (optional)

For local-side execution with env-driven parameters:

```bash
bash scripts/mesh_launch_preflight_pair.sh
```

Required env vars:
- `CHIMERA_MESH_LOCAL_ROLE` (`side_a` or `side_b`)
- `CHIMERA_MESH_NAMESPACE`
- `CHIMERA_MESH_LOCAL_NODE`
- `CHIMERA_MESH_REMOTE_NODE`
- `CHIMERA_MESH_REMOTE_ENDPOINT`
- `CHIMERA_MESH_LOCAL_OUT`
- `CHIMERA_MESH_REMOTE_OUT`

Safety note:
- for real two-host runs, `CHIMERA_MESH_REMOTE_ENDPOINT` must be a real reachable host.
- documentation placeholder ranges (`198.51.100.0/24`, `203.0.113.0/24`, `192.0.2.0/24`) are blocked by env guard unless `CHIMERA_MESH_ALLOW_REMOTE_MISSING=1` is explicitly set for local-only staged checks.

Optional env vars:
- `CHIMERA_MESH_POLICY_PAYLOAD`
- `CHIMERA_MESH_TRAFFIC_PROFILE` (`high_speed_anonymous|privacy_first|speed_first|low_latency_private`)
- `CHIMERA_MESH_TIMEOUT_MS`
- `CHIMERA_MESH_VERIFY_OUT`
- `CHIMERA_MESH_EXTRA_PEERS` (comma/newline peer list for N-node tests)

Important:
- set either `CHIMERA_MESH_POLICY_PAYLOAD` or `CHIMERA_MESH_TRAFFIC_PROFILE`, not both.
- if both are unset, wrapper defaults to `CHIMERA_MESH_TRAFFIC_PROFILE=high_speed_anonymous`.

Templates:
- `configs/mesh_launch_preflight.vps.env.example`
- `configs/mesh_launch_preflight.laptop.env.example`

Convenience commands after copying templates to `.env` files:

```bash
cp configs/mesh_launch_preflight.vps.env.example configs/mesh_launch_preflight.vps.env
cp configs/mesh_launch_preflight.laptop.env.example configs/mesh_launch_preflight.laptop.env

just mesh-launch-preflight-ready-check
just mesh-launch-preflight-side-a
just mesh-launch-preflight-side-b
just mesh-launch-preflight-evidence-guard
```

Fast endpoint update helper:

```bash
just mesh-launch-preflight-set-remote-endpoint side_a <laptop_host:port>
just mesh-launch-preflight-set-remote-endpoint side_b <vps_host:port>
```

Profile override examples (no env-file edits required):

```bash
just mesh-launch-preflight-side-a-profile high_speed_anonymous
just mesh-launch-preflight-side-b-profile high_speed_anonymous

just mesh-launch-preflight-side-a-profile privacy_first
just mesh-launch-preflight-side-b-profile privacy_first
```

Copy-paste quick block (all profiles):

```bash
just mesh-launch-preflight-side-a-profile high_speed_anonymous && just mesh-launch-preflight-side-b-profile high_speed_anonymous
just mesh-launch-preflight-side-a-profile privacy_first && just mesh-launch-preflight-side-b-profile privacy_first
just mesh-launch-preflight-side-a-profile speed_first && just mesh-launch-preflight-side-b-profile speed_first
just mesh-launch-preflight-side-a-profile low_latency_private && just mesh-launch-preflight-side-b-profile low_latency_private
```

Copy-paste staged block (all profiles, first-host sequencing):

```bash
just mesh-launch-preflight-side-a-profile-staged high_speed_anonymous && just mesh-launch-preflight-side-b-profile-staged high_speed_anonymous
just mesh-launch-preflight-side-a-profile-staged privacy_first && just mesh-launch-preflight-side-b-profile-staged privacy_first
just mesh-launch-preflight-side-a-profile-staged speed_first && just mesh-launch-preflight-side-b-profile-staged speed_first
just mesh-launch-preflight-side-a-profile-staged low_latency_private && just mesh-launch-preflight-side-b-profile-staged low_latency_private
```

Copy-paste staged two-phase block (strict first-host sequencing):

```bash
just mesh-launch-preflight-side-a-profile-staged high_speed_anonymous
just mesh-launch-preflight-side-b-profile-staged high_speed_anonymous

just mesh-launch-preflight-side-a-profile-staged privacy_first
just mesh-launch-preflight-side-b-profile-staged privacy_first

just mesh-launch-preflight-side-a-profile-staged speed_first
just mesh-launch-preflight-side-b-profile-staged speed_first

just mesh-launch-preflight-side-a-profile-staged low_latency_private
just mesh-launch-preflight-side-b-profile-staged low_latency_private
```

Profile cheat-sheet (operator quick choice):
- `high_speed_anonymous`:
  - best for: bulk downloads/uploads, CDN-heavy traffic, large artifacts;
  - behavior: prefers throughput and flow sharding;
  - expectation in restricted networks: may show `blocked` on one side, artifact is still valid for diagnostics.
- `privacy_first`:
  - best for: stable web usage with conservative path behavior;
  - behavior: single-path tendency and same-egress continuity;
  - expectation: fewer aggressive path changes, better session consistency.
- `speed_first`:
  - best for: software updates, container/model/artifact pulls;
  - behavior: throughput-priority with broader load tolerance;
  - expectation: highest transfer bias, less strict continuity than privacy-first.
- `low_latency_private`:
  - best for: calls, gaming-like latency-sensitive checks, remote interactive flows;
  - behavior: strict reliability/load thresholds, low-latency-oriented peer choice;
  - expectation: can reject peers faster when quality drops.

N-node preflight example (extra candidate peers):

```bash
CHIMERA_MESH_EXTRA_PEERS='node-c@203.0.113.30:443@eu@30@88,node-d@203.0.113.31:443@eu@35@86' \
  just mesh-launch-preflight-side-a-profile high_speed_anonymous
```

Staged mode (for first host run when remote artifact is not available yet):
```bash
just mesh-launch-preflight-side-a-staged
just mesh-launch-preflight-side-b-staged
```
These commands set `CHIMERA_MESH_ALLOW_REMOTE_MISSING=1`, produce local preflight artifact,
and skip pair verify until both artifacts exist.

Autopilot mode (single command):
```bash
just mesh-launch-preflight-autopilot
```
Defaults:
- mode: `staged`
- profile set: `core` (`high_speed_anonymous`, `privacy_first`)
- vps endpoint: `91.124.19.180:443`

Variants:
```bash
just mesh-launch-preflight-autopilot full core 91.124.19.180:443
just mesh-launch-preflight-autopilot staged all 91.124.19.180:443
```
Behavior:
- auto-binds real endpoints via `mesh-launch-preflight-auto-bind`;
- enforces `mesh-launch-preflight-ready-check`;
- runs side A then side B for selected profile set;
- in `full` mode also runs `mesh-launch-preflight-evidence-guard`;
- prints final `mesh-launch-preflight-status-summary`.

`mesh-launch-preflight-evidence-guard` validates:
- freshness of `MESH_LAUNCH_PREFLIGHT_VPS.json`, `MESH_LAUNCH_PREFLIGHT_LAPTOP.json`, `MESH_LAUNCH_PREFLIGHT_VERIFY.json`
  with max age (`CHIMERA_MESH_PREFLIGHT_MAX_AGE_SEC`, default 1800 sec);
- `docs/MESH_LAUNCH_PREFLIGHT_VPS.json` against Side A role contract;
- `docs/MESH_LAUNCH_PREFLIGHT_LAPTOP.json` against Side B role contract;
- `docs/MESH_LAUNCH_PREFLIGHT_VERIFY.json` aggregate contract.
- cross-artifact consistency: namespace and ready flags in `VERIFY` must match peer reports.

Smoke command for full evidence chain:
```bash
just mesh-launch-preflight-evidence-smoke
```

Profile smoke command (wrapper + parser contract for all presets):
```bash
just mesh-launch-preflight-profile-smoke
```

Daily quick check (without full gate):
```bash
just mesh-launch-preflight-ready-hint
just mesh-launch-preflight-status-summary
just mesh-launch-preflight-ready-check
just mesh-launch-preflight-profile-two-phase-fastcheck-selfcheck
just mesh-launch-preflight-profile-two-phase-fastcheck
```

Route-explain operator contract quick check:
```bash
just mesh-route-explain-operator-contract-selfcheck
just mesh-route-explain-operator-contract-check
```
Use this before real launch attempts when routing/diagnostics contract code was touched.

Documentation-only fast check:
```bash
just mesh-launch-preflight-doc-fast-selfcheck
```
Use it when you changed only `MESH_FIRST_LAUNCH_EXECUTION_GATE.md` and want the fastest structural verification.

Full gate note:
- `just mesh-launch-gate-selfcheck` now includes this fast fail-fast block before heavier smoke runs.

Profile smoke PASS semantics:
- command must finish with exit code `0`;
- for every profile (`high_speed_anonymous`, `privacy_first`, `speed_first`, `low_latency_private`)
  staged run is executed in strict two-phase order: first `side_a`, then `side_b`;
- side run may return `0` (`ready`) or `1` (`blocked`) because real connectivity depends on environment;
- any other side return code is failure;
- required local preflight artifacts for each side/profile must be created;
- temporary `/tmp/chimera_mesh_launch_preflight_*` artifacts must be cleaned after smoke.

Preservation check (must stay green):
```bash
just mesh-launch-preflight-evidence-smoke-docs-preserve-selfcheck
```
This confirms smoke flow does not modify real `docs/MESH_LAUNCH_PREFLIGHT_*.json` artifacts.

Preflight env gate:
- before either command runs `mesh launch-preflight`, `scripts/mesh_launch_preflight_env_guard.sh`
  validates required variables and endpoint/path invariants from the selected env file.
- if env validation fails, pair preflight is blocked immediately.

One-shot unblock + run:

```bash
just mesh-launch-preflight-unblock-and-run <laptop_host:port>
```

Fast unblock path (step-by-step):
```bash
just mesh-launch-preflight-set-remote-endpoint side_a <laptop_host:port>
just mesh-launch-preflight-ready-check
just mesh-launch-preflight-side-a && just mesh-launch-preflight-side-b && just mesh-launch-preflight-evidence-guard
```

Quick status summary:
```bash
just mesh-launch-preflight-status-summary
```

Optional local endpoint update helper:
```bash
just mesh-launch-preflight-set-local-endpoint side_b <laptop_host:port>
```

Set both real endpoints in one step:
```bash
just mesh-launch-preflight-set-real-endpoints <laptop_host:port> <vps_host:port>
```
