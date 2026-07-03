# Report Pack

Status: **FAIL**

Included reports:
- MVP spec coverage: `true` (`docs/MVP_SPEC_COVERAGE.md`)
- M5 artifacts: `true` (`docs/M5_ARTIFACTS_REPORT.md`)
- M6 artifacts: `true` (`docs/M6_ARTIFACTS_REPORT.md`)
- Release readiness: `false` (`docs/RELEASE_READINESS_REPORT.md`)
- GitHub release -> SSH runtime slice: `false` (`docs/WORKFLOW_ATTESTATION_GITHUB_RELEASE_RUNTIME_GATE_*.md`)
- CEF phase1 smoke: `true` (`docs/CEF_PHASE1_SMOKE.json`)
- Mesh route explain: `true` (`docs/MESH_ROUTE_EXPLAIN.json`)
- Mesh auto adaptive trace: `true` (`docs/MESH_AUTO_ADAPTIVE_TRACE.json`)

Truth boundary:
- Lab/proof/report contour only: `true`
- Real OS-level datapath closure (strict M4/M5): `false`

Network safety: no OS route/DNS/firewall/proxy changes in this report path.
