# Mesh Nodes JSON Contract

Status: active (P1 partial implementation)
Contract version: `1`
Network state marker: `not_modified`

This document defines stable JSON envelopes for:
- `chimera mesh nodes probe --all --json`
- `chimera mesh nodes state --json`
- error responses for `probe/state` in JSON mode

## 1. Common Fields

All success and error envelopes include:
- `kind` (string)
- `status` (`ok` or `error`)
- `contract_version` (number, currently `1`)
- `network_state` (string, currently `not_modified`)

Error envelopes additionally include parity metadata:
- `contract_family` (`mesh_nodes_contract`)
- `error_signature` (`<stage>:<action>`)
- `error_route_key` (`<kind>:<action>`)

## 2. Success: Probe All

`kind = "mesh_nodes_probe_all"`

Fields:
- `success` (bool)
- `selected` (number)
- `attempts_count` (number)
- `connected_peer` (string; `none` if empty)
- `connected_endpoint` (string; `none` if empty)
- `attempts` (array of objects):
  - `peer_id` (string)
  - `endpoint` (string)
  - `success` (bool)
  - `error` (string)

Example:

```json
{
  "kind": "mesh_nodes_probe_all",
  "status": "ok",
  "contract_version": 1,
  "network_state": "not_modified",
  "success": true,
  "selected": 1,
  "attempts_count": 1,
  "connected_peer": "de",
  "connected_endpoint": "127.0.0.1:443",
  "attempts": [
    {
      "peer_id": "de",
      "endpoint": "127.0.0.1:443",
      "success": true,
      "error": ""
    }
  ]
}
```

## 3. Success: Runtime State View

`kind = "mesh_nodes_runtime_state_view"`

Fields:
- `current_node_id` (string)
- `pinned_node_id` (string)
- `autoconnect` (bool or null)
- `restricted_mode` (bool)
- `restricted_reason` (string)

Example:

```json
{
  "kind": "mesh_nodes_runtime_state_view",
  "status": "ok",
  "contract_version": 1,
  "network_state": "not_modified",
  "current_node_id": "de",
  "pinned_node_id": "de",
  "autoconnect": true,
  "restricted_mode": false,
  "restricted_reason": ""
}
```

## 4. Error Envelope

Used by `probe/state` in JSON mode.

Fields:
- `kind` (string; operation domain)
- `status` (`error`)
- `contract_version` (number)
- `network_state` (`not_modified`)
- `stage` (string)
- `action` (string)
- `message` (string)

Example:

```json
{
  "kind": "mesh_nodes_probe_all",
  "status": "error",
  "contract_version": 1,
  "network_state": "not_modified",
  "stage": "probe_input",
  "action": "inspect_inventory",
  "message": "no nodes available for probe"
}
```

## 5. Evidence

Implementation:
- `crates/chimera-cli/src/mesh_cli/nodes_cmd.rs`

Tests:
- `crates/chimera-cli/src/mesh_cli/tests_nodes_runtime_state.rs`

Snapshot regression locks (exact JSON string equality):
- `nodes_probe_all_json_snapshot_stable`
- `nodes_state_view_json_snapshot_stable`
- `nodes_json_error_snapshot_stable`

## 6. Quantum-Safety Track (Mandatory Next Hardening)

Current guard handshake (`CHIMERA_HELLO <token> -> CHIMERA_OK`) is an access
gate, not full cryptographic node identity.

Required upgrade path:
- signed challenge-response with nonce + ttl + anti-replay cache;
- hybrid signature policy for control-plane (`classical + PQ`);
- key id + revocation + rotation binding to discovery trust set;
- reject unsigned or stale responses even if endpoint is reachable.

Current enforced behavior in CLI guard flow:
- challenge includes `key_id` and `pq_key_id`;
- verifier must match both ids against expected values;
- mismatch is rejected with `unexpected_key_id` / `unexpected_pq_key_id`.
- invalid challenge timing is rejected:
  - `invalid_ttl_window`
  - `issued_at_too_far_in_future`
  - `challenge_expired`
- replayed nonce is rejected with `guard_replay_nonce`.
- probe JSON error contract includes `stage=proof_verify` and
  `action=verify_chimera_proof` for proof-gate failures.
