# MVP

The practical MVP source of truth is `../CHIMERA-PQ_MVP_SPEC.md`.

Current workspace includes implemented and test-covered contours up to M6
(lab/verification profile), with explicit runtime apply smoke checks and
rollback verification.

Implemented baseline:

- Rust workspace and tooling;
- config parser skeleton;
- route decision model;
- DNS binding model;
- frame parser and replay window;
- sealed `DATA`/`FIN` frame validation/forwarding contract for opaque peer transit;
- WEAVE symmetric node contract requiring local ingress, peer ingress, local
  egress, and peer transit in one node model;
- in-memory carrier for tests;
- TLS carrier crate skeleton;
- QUIC carrier crate skeleton;
- capture planning crate with transparent TUN mode and fail-closed behavior when TUN is unavailable;
- fake node handshake over in-memory carrier;
- HKDF-SHA256 key schedule skeleton wired into established sessions;
- CLI and transitional node/gateway wrapper skeletons with node-first runtime role;
- CLI status diagnostics include capture-mode plan and carrier profile output;
- typed legacy client/gateway config parsing with example config files and config smoke checks;
- lab smoke command;
- fuzz smoke command for parser/decoder robustness checks;
- net-sim command for local loss/delay/reconnect/mtu simulation (no OS network changes);
- perf smoke and benchmark report commands for M6-style performance checks.

Operational note:

- default proof path is safe and non-invasive for host networking;
- runtime apply path is explicit and controlled (requires `--apply-*` flags);
- release/ship gates require runtime apply DNS/route evidence plus rollback
  artifacts before reporting PASS.
