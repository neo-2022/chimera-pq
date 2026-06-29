# Workflow Attestation: Real-World Release/Update and Relay-Fronted Peer Fallback Proof v0.1.135

Status: real_world_release_update_pass_for_v0_1_135
Date: 2026-06-26

## Objective

Verify that GitHub Release/Latest `v0.1.135` can be installed on external
stand side A through the documented one-command path, and that an already
installed CHIMERA node can update from another CHIMERA peer when GitHub is
unreachable. The peer mirror is fronted by a relay and the node auto-selects
its listen port.

This is a real remote stand proof only. The local controller runtime was not
used as the product runtime.

## Council Notes

Real sub-agent roles were used for architecture, development, testing,
security, DevOps/release, and critic review.

Agreed:

- GitHub Latest remains the primary first-install/update source.
- Peer fallback is valid only for already installed CHIMERA.
- Relay-fronted peer update must keep same-origin metadata/archive/checksum.
- Private listen state may differ from the public update origin.

Rejected:

- claiming the proof from source-only tests;
- using local controller runtime as product runtime proof;
- exposing stand IPs, ports, logins, or secrets in public proof text.

## GitHub Latest Evidence

GitHub Latest was verified at the time of proof and the canonical bootstrap URL
was used:

```text
https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh
```

Latest release under proof:

```text
v0.1.135
```

## Remote Stand Side A Proof

Trusted stand role:

- stand_side=a
- ssh_ok=true
- remote_stand_used=true
- product runtime not local

Canonical one-command install/update path on remote stand side A:

```bash
bash -o pipefail -c 'curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60 https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh | bash -s -- -install'
```

Evidence:

```text
before_version=0.1.135
before_sha=28b9a9ce257f8f6eb3c629b7adcb22bd34f41e936d5610fcfabcb74a2e9042eb
chimera_bootstrap=download archive=https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera-pq-release.tar.gz
install_rc=0
after_version=0.1.135
after_sha=28b9a9ce257f8f6eb3c629b7adcb22bd34f41e936d5610fcfabcb74a2e9042eb
```

Auto-selected peer-update state on remote stand side A:

```text
peer_update_state=present
listen=0.0.0.0:<auto>
public_origin=http://<control-relay>
update_bootstrap_url=present
version_ok=true
checksum_ok=true
```

## Remote Stand Side B Proof

Trusted stand role:

- stand_side=b
- ssh_ok=true
- remote_stand_used=true
- product runtime not local

Peer-fallback update command on remote stand side B with GitHub intentionally
unreachable:

```bash
env \
  CHIMERA_UPDATE_BOOTSTRAP_URL=http://127.0.0.1:1/chimera.sh \
  CHIMERA_UPDATE_PEER_BOOTSTRAP_URLS=http://<control-relay>/chimera.sh \
  CHIMERA_UPDATE_PEER_BOOTSTRAP_URLS_FILE=<empty-proof-file> \
  "$HOME/.local/bin/chimera.sh" -start
```

Evidence:

```text
before_version=0.1.131
before_sha=6f83b42b50464378ac48bfa297baa4afaaa3164b7e7516691fbc3274f13f2bf2
chimera_update=available source=peer current_version=0.1.131 latest_version=0.1.135 current_sha=6f83b42b50464378ac48bfa297baa4afaaa3164b7e7516691fbc3274f13f2bf2 latest_sha=28b9a9ce257f8f6eb3c629b7adcb22bd34f41e936d5610fcfabcb74a2e9042eb action=install
CHIMERA self-contained install
start_status=ok mode=systemd_user node_runtime=running node=started transparent_runtime=stopped endpoint=unconfigured
start_rc=0
after_version=0.1.135
after_sha=28b9a9ce257f8f6eb3c629b7adcb22bd34f41e936d5610fcfabcb74a2e9042eb
stop_status=ok mode=systemd_user
stop_rc=0
```

## Result

Pass for the narrow release/update live contour:

- GitHub Latest install/update path succeeded on remote stand side A.
- Peer fallback succeeded on remote stand side B when GitHub was intentionally
  blocked.
- The peer mirror used an auto-selected listen port and a separate public
  update origin.
- Installed version and checksum matched `v0.1.135`.

## Limits

- This is not a full WEAVE datapath proof.
- This is not a throughput or long-run performance proof.
- This is not a product release signature proof.
- The proof uses redacted stand markers only.
