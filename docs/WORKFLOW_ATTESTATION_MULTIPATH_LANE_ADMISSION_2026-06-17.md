# CHIMERA Workflow Attestation: Multipath Lane Admission Observability

Timestamp: 2026-06-17 23:29 MSK

## Scope

This source slice adds privacy-safe observability for WEAVE multipath lane
admission. It reports how many active lane slots the planner requested,
admitted and rejected under the transit capacity budget.

This is control-plane diagnostics only. It is not runtime throughput evidence,
not payload accounting and not Real-World datapath proof.

## Implemented

- Added `crates/chimera-mesh/src/runtime/multipath_lane_admission.rs`.
- Added `MeshMultipathSchedule` aggregate fields:
  - `lane_admission_requested_active_lane_count`;
  - `lane_admission_admitted_active_lane_count`;
  - `lane_admission_rejected_active_lane_count`;
  - `lane_admission_capacity_status`.
- Added public explain lines under the explicit
  `multipath_schedule_lane_admission_*` prefix.
- Kept transit payload policy at `sealed_opaque_only`.
- Kept route binding IDs opaque and did not add peer endpoint or route ID output.
- Split multipath redaction tests into
  `crates/chimera-mesh/src/tests_multipath_schedule/redaction.rs`.

## Team Review Summary

Agreed:

- The change fits MVP diagnostics/route explanation scope.
- Public output must remain aggregate-only.
- `rejected_active_lanes` means rejected planner lane slots, not dropped packets.
- The output must not be treated as carrier/runtime throughput evidence.
- Release/stand proof must use GitHub Release/Latest one-command update only.

Rejected:

- Any classification, scheduling or logging based on transit payload contents.
- Any public raw peer ID, endpoint, route binding ID, token, password or payload.
- Any claim that route-explain proves real TUN/datapath/transit/rollback.
- Any laptop/VPS proof through `scp`, `rsync`, local tarball, `cargo run`,
  `git clone` or local PC runtime.

## Source Evidence

PASS:

```text
cargo fmt --all -- --check
cargo test -q -p chimera-mesh tests_multipath_schedule
cargo test -q -p chimera-mesh runtime_failover_plan_from_dps_payload_applies_multipath_schedule
cargo test -q -p chimera-mesh runtime_reselection_plan_with_health_from_dps_payload_applies_multipath_schedule
cargo check -q --workspace
cargo test -q --workspace
cargo clippy -q --workspace --all-targets -- -D warnings
bash scripts/anti_monolith_guard.sh
just rust-no-hardcode-guard
bash scripts/chimera_installer_gate.sh
bash scripts/chimera_update_contract_smoke.sh
bash scripts/chimera_start_contract_smoke.sh
bash scripts/chimera_stop_contract_smoke.sh
```

Anti-monolith status:

- `crates/chimera-mesh/src/runtime/multipath_schedule.rs`: 396 lines, under
  400 runtime leaf limit.
- `crates/chimera-mesh/src/tests_multipath_schedule/mod.rs`: 391 lines, under
  450 mesh test leaf limit.
- `crates/chimera-mesh/src/tests_multipath_schedule/redaction.rs`: 141 lines.

## Remote Stand Status

Not executed yet for this source slice.

Required next evidence:

- commit and tag next release;
- publish GitHub Release/Latest with required assets;
- update laptop and VPS using only the bounded GitHub one-command bootstrap;
- verify installed version/checksum;
- verify installed `route-explain` includes
  `multipath_schedule_lane_admission_*` fields and redaction markers.

## Status Boundary

Status: partial.

Closed:

- Source-level lane admission observability.
- Source-level route-explain lines.
- Source-level redaction and non-regression checks.

Not closed:

- Installed-binary proof for this exact source slice.
- Real node-to-node transit.
- Transparent TUN/OS routing.
- DNS-to-route runtime binding.
- Forced-stop rollback on the stand.
- Browser/IDE normal workflow.
- Real multipath carrier traffic.

Do not report Real-World PASS from this artifact.
