# CHIMERA Full CEF Gap Map (2026-05-18)

## Scope

- Primary source: `/home/art/Archives/WEAVE/CHIMERA.pdf`
- Current implementation root: `/home/art/Archives/WEAVE/chimera-pq`
- Purpose: separate **MVP/Lab PASS** from **Full CEF closure**

## Truth Boundary

- MVP/Lab contour: PASS (verified by `just ship-readiness`).
- Full CEF contour from `CHIMERA.pdf`: PARTIAL / NOT CLOSED.

## Gap Matrix (High-Level)

1. Full cooperative mesh runtime
- `CHIMERA.pdf` evidence: sections around `12. Peer-assisted mesh model`, `12.2 Discovery modes`, `12.3 Joining and bootstrap paths`.
- Current fact: Phase-1 implementation is present in `crates/chimera-mesh` and runtime wiring is verified in `docs/CEF_TRACK_REPORT.json` (`mesh_runtime.runtime_wired=true`).
- Status: implemented for Phase-1 track (runtime wired), Full CEF not closed.
- Next step: extend from Phase-1 wiring to full mesh lifecycle (discovery/bootstrap integration + policy-driven path selection).

2. DHT discovery (public/private) and provider records
- `CHIMERA.pdf` evidence: sections `12.5 Public DHT global discovery`, `13. Distributed Discovery, DHT and Policy Store`, `13.2 DHT Discovery Record`.
- Current fact: Phase-1 implementation is present in `crates/chimera-dht` and runtime wiring is verified in `docs/CEF_TRACK_REPORT.json` (`dht_discovery.runtime_wired=true`).
- Status: implemented for Phase-1 track (runtime wired), Full CEF not closed.
- Next step: extend from Phase-1 wiring to provider-record lifecycle, lookup client and trust-policy validation path.

3. Distributed Policy Store (DPS)
- `CHIMERA.pdf` evidence: architecture path includes `Distributed Policy Store Client`; section `13`.
- Current fact: Phase-1 implementation is present in `crates/chimera-dps` and runtime wiring is verified in `docs/CEF_TRACK_REPORT.json` (`distributed_policy_store.runtime_wired=true`).
- Status: implemented for Phase-1 track (runtime wired), Full CEF not closed.
- Next step: extend from Phase-1 wiring to fetch/merge/update flows with signature-chain trust policy.

4. Cooperative relay participation/consent model
- `CHIMERA.pdf` evidence: `12.6 Cooperative relay consent`, `12.7 Cooperative relay traffic classes`, `12.8 Relay privacy model`.
- Current fact: Phase-1 implementation is present in `crates/chimera-relay` and runtime wiring is verified in `docs/CEF_TRACK_REPORT.json` (`cooperative_relay_model.runtime_wired=true`).
- Status: implemented for Phase-1 track (runtime wired), Full CEF not closed.
- Next step: extend from Phase-1 wiring to full relay role lifecycle in runtime/path planner with explicit policy boundaries.

5. Emergency/OOB carriers
- `CHIMERA.pdf` evidence: architecture includes `Emergency Carrier`; sections mention BLE/LoRa/audio/QR/NFC paths.
- Current fact: Phase-1 implementation is present in `crates/chimera-emergency` and runtime wiring is verified in `docs/CEF_TRACK_REPORT.json` (`emergency_oob_carriers.runtime_wired=true`).
- Status: implemented for Phase-1 track (runtime wired), Full CEF not closed.
- Next step: extend from Phase-1 wiring to feature-gated emergency bootstrap flows with policy guardrails.

6. Roaming cache / distributed bootstrap continuation
- `CHIMERA.pdf` evidence: path includes `Roaming Cache / DHT Discovery / Distributed Policy Store / Cached Policy`.
- Current fact: Phase-1 implementation is present in `crates/chimera-roaming` and runtime wiring is verified in `docs/CEF_TRACK_REPORT.json` (`roaming_cache.runtime_wired=true`).
- Status: implemented for Phase-1 track (runtime wired), Full CEF not closed.
- Next step: extend from Phase-1 wiring to signed cache artifacts and integrity-chain checks.

7. Reputation / complaint / relay credit subsystems
- `CHIMERA.pdf` evidence: path includes `Relay Credit, Complaint Evidence and Reputation Manager`; ZK/reputation concepts.
- Current fact: Phase-1 implementation is present in `crates/chimera-reputation` and runtime wiring is verified in `docs/CEF_TRACK_REPORT.json` (`reputation_complaint_credit.runtime_wired=true`).
- Status: implemented for Phase-1 track (runtime wired), Full CEF not closed.
- Next step: extend from Phase-1 wiring to complaint-evidence chain, anti-abuse gates and trust-policy lifecycle.

## Phase-1 Closure Snapshot

- `phase1_closed` in `docs/CEF_TRACK_REPORT.json`: `true`.
- This confirms Phase-1 runtime wiring for all 7 CEF blocks.
- This does **not** mean Full CEF closure; `full_cef_closed` remains `false`.

## What Is Already Strong (Use as Base)

- Secure session + handshake/test contours.
- Policy routing model and explain/diagnostics path.
- Runtime apply/rollback safety contour (explicit `--apply-*` path).
- Fuzz/perf/net-sim/hardening gates and contract checks.

## Execution Recommendation

Use two separate closure tracks and never merge their statuses:

1. `MVP/Lab Track` (already green in current gates).
2. `Full CEF Track` (new track from this gap map with separate acceptance gates).

Do not report Full CEF closure until each gap above has:
- implementation evidence in code;
- tests/artifacts;
- explicit pass criteria and non-regression checks.
