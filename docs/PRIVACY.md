# Privacy

Diagnostics must be redacted by default. Do not log raw credentials, private keys,
WEAVE configs, passwords, raw packet payloads or exact destinations in exported logs.

Transit traffic is not local node data. For peer transit/forwarding, diagnostics
may report safe envelope metadata such as frame count, packet number or byte
count, but must not expose sealed frame bytes or decrypted payload contents.

WEAVE transit forwarding code may carry sealed bytes to the next hop, but it must
not provide payload interpretation APIs. Debug output for transit frame wrappers
must redact sealed bytes.

The node runtime's sealed-transit branch follows the same rule: it validates the
outer frame envelope, forwards sealed bytes unchanged, and does not decrypt or
inspect the payload.
