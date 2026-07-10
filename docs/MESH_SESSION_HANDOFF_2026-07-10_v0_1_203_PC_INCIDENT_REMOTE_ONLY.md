# MESH_SESSION_HANDOFF_2026-07-10_v0_1_203_PC_INCIDENT_REMOTE_ONLY

**session_id:** handoff-2026-07-10-203-pc-incident
**version:** 0.1.203
**status:** partial

## Incident Summary

- The user authorized using the current PC as a temporary third stand node, with the requirement to work safely.
- An attempt to start CHIMERA v0.1.203 on the PC with `APPLY_TUN=false APPLY_ROUTE=false APPLY_DNS=false` still caused the PC's internet connection to fail.
- The user rebooted the PC and removed CHIMERA entirely.
- **Root cause hypothesis:** CHIMERA `chimera-control.sh start` either ignored or partially applied the override flags, or the existing v0.1.178 policy-routing/nftables state interacted with the new start, resulting in a broken default path. A full post-mortem was not performed because the priority was restoring connectivity.

## Recovery Actions

- Verified PC internet/DNS after reboot: ping to `8.8.8.8` and `curl https://example.com` succeed.
- Removed all residual CHIMERA artifacts from PC:
  - disabled and stopped `chimera-{runtime,node,datapath,site-watch}.service` user units;
  - moved systemd unit files, `~/.local/bin/chimera*` symlinks, `~/.config/chimera`, `~/.local/share/chimera`, and `~/.cache/chimera` to `/home/art/.chimera-removed-20260710-150039`.
- `systemctl --user daemon-reload` performed.

## Working Model Going Forward

- **Current PC is control-only again.** All practical CHIMERA runtime/datapath checks must be performed over SSH on the authorized remote stand (NL VPS, RU VPS) only.
- No future CHIMERA runtime start/network apply is allowed on the current PC unless the user gives an explicit, risk-aware command.

## Open CHIMERA Work Items

- End-to-end NL ↔ RU datapath stability after v0.1.203.
- Root cause of `site_auto_watch_loop=fail` on stand nodes.
- `CURRENT_WORKLINE_ATTESTATION.json` remains `status=partial`.

## Attestation

- `CURRENT_WORKLINE_ATTESTATION.json` points to this handoff.
- PC network confirmed functional after cleanup.
