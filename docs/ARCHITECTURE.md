# Architecture

MVP data path:

```text
Application / OS
 -> Capture Manager
 -> DNS Context Manager
 -> Flow Classifier
 -> Policy Engine
 -> RouteDecision
 -> PathPlan
 -> Secure Session
 -> Carrier
 -> Gateway
 -> Destination
```

Current implementation status (fact-based):

- M0-M6 lab/verification contour is implemented and validated by project gates
  (`just mvp-check`, `just ship-readiness`, release/readiness artifacts).
- Default validation path remains network-safe (`network_state: not_modified`).
- Explicit runtime apply path exists for controlled smoke
  (`--apply-dns`, `--apply-route`, optional TUN apply path) and is validated
  with rollback artifacts.
- Full always-on OS-wide selective datapath orchestration for arbitrary apps is
  not declared as completed here; this documentation keeps that scope explicit.
