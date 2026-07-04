# CHIMERA Mesh Session Handoff: New-Machine Proof v0.1.168

## Saved At

- Timestamp: 2026-07-04

## Active Objective

- Prove the scenario `new machine -> public install -> plain start -> bind -> full work without manual follow-up`.

## What Was Proven

- `v0.1.168` is the current GitHub `Latest`.
- Signed releases published in this session:
  - `v0.1.165` - superseded, known bad for seeded bind.
  - `v0.1.166` - superseded, known bad for auto-publish proof host flow.
  - `v0.1.167` - superseded, known bad for auto-publish trust-anchor forwarding.
  - `v0.1.168` - current latest.
- On the new VPS, public GitHub one-command install of `v0.1.167` succeeded from a clean state with authoritative seed env.
- On the new VPS, public uninstall cleaned all checked traces:
  - release tree absent
  - config absent
  - cache absent
  - launcher absent
  - systemd user units absent
- On the new VPS, plain `chimera.sh -start` in a clean shell reached:
  - `start_status=ok`
  - `bound_transit_authority_state=present`
  - `node_config_ready=true`
  - `transparent_runtime_service_state=active`
  - persisted bootstrap included non-empty:
    - `CHIMERA_MESH_LOCAL_NODE`
    - `CHIMERA_MESH_REMOTE_PEER_SPEC`

## Product Defects Found And Fixed

- Fixed seeded bind failure caused by persisted bootstrap context not being used when no preflight env existed.
  - release: `v0.1.165`
- Fixed strict-shell failure in seeded peer-spec fallback caused by uninitialized `peer_spec`.
  - release: `v0.1.166`
- Fixed seeded bind choosing incomplete live shell env over complete persisted bootstrap context.
  - release: `v0.1.167`
- Fixed auto-publish missing discovery trust-anchor forwarding from persisted bootstrap context.
  - release: `v0.1.168`

## Current External Blocker

- Final clean re-proof on `v0.1.168` is blocked by live network reachability between the new VPS and the old VPS source.
- Evidence:
  - from the control host, old VPS discovery URL is reachable and returns valid JSON.
  - on old VPS, peer-update listener is active and listening on the advertised port.
  - from the new VPS, connections to the old VPS time out on all tested ports:
    - `22`
    - `80`
    - `443`
    - advertised discovery port
    - advertised node port
- Because of that, the final `v0.1.168` clean install on the new VPS cannot currently fetch the old VPS discovery snapshot, so the end-to-end clean re-proof is `partial`, not closed.

## Truth Boundary

- `v0.1.168` release publication is real and verified.
- Public install/uninstall proof on the new VPS is real.
- Plain start with full bind on the new VPS was physically proven on `v0.1.167`.
- Auto-publish fix for trust-anchor forwarding is covered by local contract tests and released in `v0.1.168`.
- Full clean-room `v0.1.168` re-proof on the new VPS is not closed due the external inter-VPS reachability blocker above.

## Next Step

- Re-check live reachability from the new VPS to the old VPS source.
- If reachability is restored, rerun:
  1. public uninstall on new VPS,
  2. public GitHub latest install with authoritative seed env,
  3. plain `chimera.sh -start` in clean shell,
  4. `chimera.sh -status`,
  5. verify `mesh_nodes.discovery.json` exists automatically and contains invite token plus update bootstrap URL.
