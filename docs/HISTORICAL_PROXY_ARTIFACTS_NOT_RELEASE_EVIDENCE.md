# Historical Proxy Artifacts Are Not Release Evidence

Status: active

These files are historical lab artifacts only. They must not be used as
CHIMERA/WEAVE datapath proof, release readiness evidence, MVP closure evidence,
or real-world application workflow evidence.

Reason:

- CHIMERA/WEAVE is not a VPN product and not a proxy product.
- SOCKS/proxy listener evidence does not prove transparent WEAVE datapath.
- `network_state=not_modified` evidence cannot close OS-level capture/routing
  acceptance criteria by itself.

Historical artifacts quarantined by this rule:

- `docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md`
- `docs/CHIMERA_PATH_PROOF_PROD.json`
- `docs/CHIMERA_PATH_PROOF_PROD_AFTER_FIX.json`
- `docs/CHIMERA_PATH_PROOF_WITH_AISTUDIO_RUN.json`
- `docs/PROBE_ACCESS_FOREIGN_FOCUS_PROD.json`
- `docs/PROBE_ACCESS_FOREIGN_FOCUS_PROD_AFTER_FIX.json`
- `docs/PROBE_ACCESS_FOREIGN_FOCUS_WITH_AISTUDIO.json`
- `docs/PROBE_ACCESS_FOREIGN_FOCUS_WITH_AISTUDIO_RERUN.json`
- `docs/CHIMERA_BROWSER_PARALLEL_SOAK_5M.json`
- `docs/CHIMERA_BROWSER_PARALLEL_SOAK_5M_STABILIZED2.json`
- `docs/CHIMERA_PARALLEL_SOAK_5M.json`
- `docs/CHIMERA_PARALLEL_SOAK_5M_STABILIZED.json`
- `docs/REALITY_AUDIT_2026-05-18.md`
- `docs/CHIMERA_BIDIRECTIONAL_E2E_REPORT_2026-05-22_15-48-38.md`
- `docs/CHIMERA_FRESH_GATE_REPORT.md`
- `docs/CHIMERA_MESH_LOAD_REPORT_2026-05-22.md`
- `docs/load/CHIMERA_LOAD_300S_SIDE_B_20260522_103042.json`
- `docs/load/CHIMERA_LOAD_30S_SIDE_B_20260522_023957.json`
- `docs/load/CHIMERA_LOAD_30S_SIDE_B_20260522_024613.json`
- `docs/load/CHIMERA_LOAD_30S_SIDE_B_20260522_024950.json`
- `docs/load/CHIMERA_LOAD_5M_SIDE_B_20260522_023725.json`
- `docs/load/CHIMERA_LOAD_60S_SIDE_B_20260522_144925.json`
- `docs/side_b_sync/20260522_104003/CHIMERA_CHANNEL_AUDIT.json`
- `docs/side_b_sync/20260522_104003/CHIMERA_E2E_CHANNEL_GATE.json`
- `docs/side_b_sync/20260522_104003/CHIMERA_FRESH_GATE_REPORT.json`
- `docs/side_b_sync/20260522_104003/CHIMERA_LOAD_30S_SIDE_B_20260522_103852.json`
- `docs/side_b_sync/20260522_104003/CHIMERA_PATH_PROOF.json`
- `docs/side_b_sync/20260522_104217/CHIMERA_CHANNEL_AUDIT.json`
- `docs/side_b_sync/20260522_104217/CHIMERA_E2E_CHANNEL_GATE.json`
- `docs/side_b_sync/20260522_104217/CHIMERA_FRESH_GATE_REPORT.json`
- `docs/side_b_sync/20260522_104217/CHIMERA_LOAD_30S_SIDE_B_20260522_104408.json`
- `docs/side_b_sync/20260522_104217/CHIMERA_PATH_PROOF.json`

Allowed use:

- historical comparison;
- debugging old reports;
- proof that earlier proxy/SOCKS evidence is not sufficient for release.

Forbidden use:

- claiming M4/M5/M6 closure;
- claiming release readiness;
- claiming production readiness;
- claiming normal application workflow through transparent WEAVE datapath.
