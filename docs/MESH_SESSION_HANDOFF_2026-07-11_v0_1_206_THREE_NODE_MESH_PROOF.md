# MESH_SESSION_HANDOFF_2026-07-11_v0_1_206_THREE_NODE_MESH_PROOF

**session_id:** handoff-2026-07-11-206-three-node-mesh-proof
**version:** 0.1.206
**status:** partial-to-pass

## Objective

Finish propagating the CHIMERA-PQ v0.1.204/v0.1.205/v0.1.206 release train to all three
stand nodes (NL `amai`, RU `vdsina`, laptop `laptop`) and prove a working three-node
WEAVE mesh with multi-site discovery and stable bidirectional datapath.

## Stand State

- **NL:** `<nl-stand-ip>`, node_id `amai`, Ubuntu 24.04
- **RU:** `<ru-stand-ip>`, node_id `vdsina`, Ubuntu 26.04
- **Laptop:** `<laptop-stand-ip>`, node_id `laptop`, Ubuntu 26.04, user `art`
- **PC:** control-only, no CHIMERA runtime.

All three nodes now run `chimera-runtime 0.1.206` via local-release install
(`CHIMERA_ALLOW_LOCAL_RELEASE_SOURCE=1`).

### Service Status

| node | start_status | runtime_publication | peer_egress_mode | site-watch |
|------|--------------|--------------------:|------------------|------------|
| NL   | ok           | ready               | node             | active     |
| RU   | ok           | ready               | node             | active     |
| laptop | partial    | degraded            | node             | active     |

Laptop publication is intentionally degraded: it is a non-publishing third node
(behind NAT, no `peer-update.env`), but it still discovers both VPS nodes and
carries traffic over the mesh.

### Code Changes in This Session

Three signed commits on `main`:

1. `8138fe2` — `fix(carrier): merge mesh discovery snapshots from all configured URLs`
2. `2917c1d` — `cli: split CHIMERA_MESH_NODES_DISCOVERY_URL by comma for multi-source discovery`
3. `e603494` — `carrier: add deadline/retry loop to lane-document local ingress path`

### Config Changes on Each Node

- `mesh_bootstrap.env`: `CHIMERA_MESH_NODES_DISCOVERY_URL` now contains both
  discovery URLs comma-separated, so both the carrier (which already split the
  value) and the CLI (now also splits it) see both sources.
- `peer-egress.env`: carrier discovery URL kept as a comma-separated list (with
  systemd `\,` escaping where loaded via `EnvironmentFile`).
- Laptop `CHIMERA_MESH_REMOTE_PEER_SPEC` left as fallback to current RU endpoint.

### Discovery Merge Proof

From the laptop, `chimera-cli mesh nodes list` now reports **2 nodes**: `amai`
and `vdsina`, each from the respective discovery snapshot, confirming that the
CLI merges snapshots from all configured sources.

The laptop lane document shows two active carrier bindings:

```text
# chimera_plan_selected_peer  0  amai   <nl-stand-ip>:<port>  NL  20  90  250
# chimera_plan_selected_peer  1  vdsina <ru-stand-ip>:<port>  RU  20  90  250
# chimera_plan_carrier_binding 1  0  amai   <nl-stand-ip>:<port>  active  50  45
# chimera_plan_carrier_binding 1  1  vdsina <ru-stand-ip>:<port>  active  50  45
```

Flow-shard multipath is balancing across NL and RU from the laptop.

## Datapath Proof

All tests below run the target through the local transparent TCP proxy; root/art
UID is exempt (direct), `nobody` is captured and sent through the mesh.

### NL → RU (`http://ifconfig.me`)

20 sequential `runuser -u nobody` curls from NL:

- **pass:** 19/20 returned `<ru-stand-ip>`
- **fail:** 1/20 DNS resolution timeout after 10 s (not an empty reply)

### RU → NL (`http://ifconfig.me`)

20 sequential `runuser -u nobody` curls from RU:

- **pass:** 20/20 returned `<nl-stand-ip>`
- **fail:** 0

### NL ↔ RU (`http://ipinfo.io/ip`)

10 sequential curls per direction:

- NL → RU: 10/10 returned `<ru-stand-ip>`
- RU → NL: 10/10 returned `<nl-stand-ip>`

The prior sporadic `Empty reply from server` on `ipinfo.io` is no longer
observed after adding the lane-document retry loop.

### Laptop Mesh

20 sequential `sudo -n -u nobody` curls from laptop:

- `http://ifconfig.me`: 20/20 passed (exited through NL or RU)
- `http://icanhazip.com`: 20/20 passed, alternating between `<nl-stand-ip>`
  and `<ru-stand-ip>`, confirming flow-shard distribution across both peers.

### Direct Traffic Bypass

- NL root direct `ifconfig.me`: returned `<nl-stand-ip>`
- RU root direct `ifconfig.me`: returned `<ru-stand-ip>`
- Laptop art direct `ifconfig.me`: returned laptop ISP IP `<laptop-wan-ip>`

Split-tunnel bypass remains correctly active for exempt UIDs.

## What Was Fixed

1. **Release propagation blocker:** the v0.1.204 tarball did not ship a
   versioned `.sha256`; used `chimera-pq-release.tar.gz` +
   `chimera-pq-release.tar.gz.sha256` for local-source install.
2. **Three-node discovery:** the CLI did not split the singular
   `CHIMERA_MESH_NODES_DISCOVERY_URL` and stopped at the first successful
   snapshot. Now it splits by comma and merges all successful snapshots,
   matching the carrier behaviour.
3. **Lane-document dead-peer flakiness:** the production local-ingress path
   picked one peer from the planned lane and gave up on handshake failure.
   Added a deadline/retry loop with 50 ms backoff, discarding dead peers and
   waiting for fresh ones, like the peer-pool path.

## Residual / Not Closed

- **Laptop publication:** remains `degraded`; the laptop does not publish a
  discovery snapshot/peer-update state (no public endpoint). It is used as an
  outbound/transiting mesh member only.
- **GitHub release:** v0.1.203 is still the latest published GitHub release.
  The v0.1.206 build exists locally and is installed on the stand, but not
  uploaded to GitHub.
- **Long-run soak:** 20-run proof is sufficient for a milestone smoke test but
  not a production soak.
- **`chimera-lab` attestation-guard unit tests:** 10 pre-existing failures in
  `current_workline_attestation_guard` remain unrelated to these changes.

## Attestation Evidence

```text
remote_stand_used=true
ssh_ok=true
version=0.1.206
nodes=amai,vdsina,laptop
laptop_discovery_nodes=2
nl_ru_ifconfig_me=19/20 and 20/20
nl_ru_ipinfo_io=10/10 and 10/10
laptop_mesh_ifconfig_me=20/20
laptop_mesh_icanhazip_me=20/20
```
