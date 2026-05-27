# Security

MVP security rules:

- no unsafe code;
- no panics on malformed input;
- no unwrap/expect on untrusted input;
- explicit frame parser errors;
- replay window tests;
- redacted diagnostics by default.

Cryptographic status must be stated in a truth-first way:

- X25519-based handshake/session path is implemented and test-covered for MVP
  lab contour.
- HKDF-SHA256 key schedule is implemented with transcript binding and split
  traffic secrets.
- Suite downgrade and replay protections are covered by tests.
- ML-KEM hybrid suite is not declared as production-complete here.

`chimera-crypto` contains the first M2 key schedule skeleton:

- transcript hashing with SHA-256;
- HKDF-SHA256 traffic secret derivation;
- separate client-to-gateway and gateway-to-client traffic secrets;
- zeroize-on-drop for traffic secrets.

Security reporting rule:

- do not claim production-grade PQ deployment without explicit ML-KEM hybrid
  implementation evidence and dedicated acceptance artifacts.
