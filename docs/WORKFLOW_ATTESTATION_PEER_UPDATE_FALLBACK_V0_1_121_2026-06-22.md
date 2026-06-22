# CHIMERA Peer Update Fallback Attestation

Status: peer_update_fallback_pass_for_v0_1_121

Date: 2026-06-22

## Scope

Verify the update-only peer fallback path for an already installed CHIMERA node.

This is not a first-install proof and not a full WEAVE datapath proof.

## Stand

- Source peer: SIDE_A `<stand-admin>@<stand-host-b>`
- Update target: side_b `<stand-user>@<stand-host-a>`
- Local PC role: SSH control point only
- Local CHIMERA runtime: not used
- Release under test: `v0.1.121`
- Release checksum:
  `0900388810d2b77e2ba2981e6bd478eba9d48c025afb297ddc20a686897ed50b`

## Commands And Evidence

1. Prepared source peer from installed CHIMERA release:

```text
chimera-bootstrap serve-release --root "$HOME/.local/share/chimera" --listen 0.0.0.0:18179 --base-url http://<stand-host-b-ip>:18179
```

Evidence:

```text
chimera_peer_update_serve=ready listen=0.0.0.0:18179 version=0.1.121 sha256=0900388810d2b77e2ba2981e6bd478eba9d48c025afb297ddc20a686897ed50b
```

Side B could read the peer metadata:

```text
status=ok version=0.1.121
{"status":"ok","kind":"chimera_peer_update_metadata","version":"0.1.121","archive":"http://<stand-host-b-ip>:18179/chimera-pq-release.tar.gz","checksum":"http://<stand-host-b-ip>:18179/chimera-pq-release.tar.gz.sha256","sha256":"0900388810d2b77e2ba...
```

2. Prepared the target with older installed release using GitHub one-command
   release install for `v0.1.120`:

```text
before_version=chimera-runtime 0.1.121
before_sha=0900388810d2b77e2ba2981e6bd478eba9d48c025afb297ddc20a686897ed50b
chimera_install=ok version=0.1.120 home=<home>/.local/share/chimera
downgrade_rc=0
after_version=chimera-runtime 0.1.120
after_sha=98edec8b3ea407f8192943d7f2b6ea5fe6a751451947b3cb796559a5220acf55
```

3. Positive fallback update with GitHub bootstrap URL intentionally unreachable
   and SIDE_A peer mirror configured:

```text
CHIMERA_UPDATE_BOOTSTRAP_URL=http://127.0.0.1:9/chimera.sh
CHIMERA_UPDATE_PEER_BOOTSTRAP_URLS=http://<stand-host-b-ip>:18179
```

Evidence:

```text
before_version=chimera-runtime 0.1.120
before_sha=98edec8b3ea407f8192943d7f2b6ea5fe6a751451947b3cb796559a5220acf55
chimera_update=available source=peer current_version=0.1.120 latest_version=0.1.121 current_sha=98edec8b3ea407f8192943d7f2b6ea5fe6a751451947b3cb796559a5220acf55 latest_sha=0900388810d2b77e2ba2981e6bd478eba9d48c025afb297ddc20a686897ed50b action=install
CHIMERA self-contained install
  source: http://<stand-host-b-ip>:18179/chimera-pq-release.tar.gz
start_status=ok mode=systemd_user node_runtime=running node=started transparent_runtime=stopped endpoint=unconfigured
fallback_start_rc=0
after_version=chimera-runtime 0.1.121
after_sha=0900388810d2b77e2ba2981e6bd478eba9d48c025afb297ddc20a686897ed50b
stop_status=ok mode=systemd_user
```

4. Negative path with GitHub and peer sources unreachable, using an isolated
   empty peer URL file to avoid persistent stand configuration:

```text
CHIMERA_UPDATE_BOOTSTRAP_URL=http://127.0.0.1:9/chimera.sh
CHIMERA_UPDATE_PEER_BOOTSTRAP_URLS=http://127.0.0.1:9
CHIMERA_UPDATE_PEER_BOOTSTRAP_URLS_FILE=<empty proof file>
```

Evidence:

```text
before_version=chimera-runtime 0.1.121
before_sha=0900388810d2b77e2ba2981e6bd478eba9d48c025afb297ddc20a686897ed50b
chimera_update=unavailable current_version=0.1.121 action=continue reason=update_sources_unreachable
start_status=ok mode=systemd_user node_runtime=running node=started transparent_runtime=stopped endpoint=unconfigured
negative_isolated_start_rc=0
after_version=chimera-runtime 0.1.121
after_sha=0900388810d2b77e2ba2981e6bd478eba9d48c025afb297ddc20a686897ed50b
stop_status=ok mode=systemd_user
```

5. Final stand state:

```text
side_b: chimera-runtime 0.1.121
side_b_sha=0900388810d2b77e2ba2981e6bd478eba9d48c025afb297ddc20a686897ed50b
side_b_node_runtime=stopped
side_b_transparent_runtime=stopped

side_a: chimera-runtime 0.1.121
side_a_sha=0900388810d2b77e2ba2981e6bd478eba9d48c025afb297ddc20a686897ed50b
side_a_node_runtime=stopped
side_a_transparent_runtime=stopped
```

The temporary SIDE_A `serve-release` process was stopped after the proof.

## Result

Pass for the narrow update-only fallback contract:

- an already installed older node updated from peer source when GitHub bootstrap
  URL was unreachable;
- peer metadata, archive and checksum came from the configured SIDE_A peer mirror;
- installed version and checksum matched `v0.1.121`;
- when both update sources were unreachable, the installed version and checksum
  remained unchanged and CHIMERA continued with the installed release.

## Limits

- This is not GitHub first-install proof.
- This is not full release-readiness proof.
- This is not full WEAVE datapath proof.
- The peer mirror used HTTP inside the trusted stand. Checksum verified
  integrity, but signed peer release manifests remain a hardening item.
- The runtime still reported `peer_egress_transit_lane_bindings_publish=skipped
  reason=missing_authoritative_mesh_context`, so live multipath lane datapath
  remains unclosed.
