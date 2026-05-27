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
- in-memory carrier for tests;
- TLS carrier crate skeleton;
- QUIC carrier crate skeleton;
- capture planning crate with TUN/local-proxy fallback model;
- fake client/gateway handshake over in-memory carrier;
- HKDF-SHA256 key schedule skeleton wired into established sessions;
- CLI and gateway skeletons;
- CLI status diagnostics include capture-mode plan and carrier profile output;
- typed client/gateway config parsing with example config files and config smoke checks;
- lab smoke command;
- fuzz smoke command for parser/decoder robustness checks;
- net-sim command for local loss/delay/reconnect/mtu simulation (no OS network changes);
- perf smoke and benchmark report commands for M6-style performance checks.

Operational note:

- default proof path is safe and non-invasive for host networking;
- runtime apply path is explicit and controlled (requires `--apply-*` flags);
- release/ship gates require runtime apply DNS/route evidence plus rollback
  artifacts before reporting PASS.
