# Workflow Attestation: Connect Attempt Endpoint Update Consumption

## Scope

- Date: 2026-06-29
- Component: `chimera-mesh`
- Proof scope: lab/control-plane only
- Runtime started: false
- Local network state changed: false
- Remote stand used: false

## What Changed

- `connect_probe` now builds connection targets through one normalized
  connect-attempt plan before trying endpoints.
- The connect-attempt plan is used by the real `connect_probe` path and by
  focused lab tests.
- The lab proof now covers the causal chain:

```text
existing peer endpoint
 -> fresh published endpoint_generation update
 -> runtime peer endpoint update
 -> pending rebuild reason published_endpoint_changed
 -> connect attempt plan uses the fresh endpoint
```

## Positive Proof

- Fresh published endpoint update with a newer generation changes the selected
  connect attempt targets.
- The old endpoint is not kept in the connect attempt plan after the fresh
  update.
- The rebuild signal is `published_endpoint_changed` with
  `affected_peer_count=1`.

## Negative Proof

- Stale published endpoint generation does not roll the connect attempt plan
  back.
- Same generation with the same endpoint is no-op and does not create a new
  dirty signal.
- Invalid published endpoint update is rejected atomically and preserves the
  previous connect attempt plan.

## Validation

PASS:

- `cargo fmt --all -- --check`
- `cargo check -q --workspace --all-targets`
- `cargo test -q --workspace --all-targets`
- `cargo clippy -q --workspace --all-targets -- -D warnings`
- `cargo test -q -p chimera-mesh connect_attempt_plan_ -- --nocapture`
- `just rust-no-hardcode-guard-selfcheck`
- `just release-pack-schema-guard-selfcheck`
- `just ship-structure-guard-selfcheck`
- `./scripts/release_pack_schema_guard.sh`
- `./scripts/ship_structure_guard.sh`
- `git diff --cached --check`

## Limits

- This is not Real-World PASS.
- This does not prove real carrier reconnect on the SSH stand.
- This does not prove automatic bind/rebind end-to-end in a running service.
- Transit payload opacity remains governed by existing WEAVE sealed-frame
  tests; this slice does not inspect payload.
