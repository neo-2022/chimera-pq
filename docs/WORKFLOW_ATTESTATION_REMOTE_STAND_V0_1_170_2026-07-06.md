# WORKFLOW_ATTESTATION: Remote Stand Proof for CHIMERA-PQ v0.1.170

## Scope

Verify published `v0.1.170` on the authorized SSH-only stand, keeping the
proof redacted and separate from GitHub delivery metadata.

## Stand

- `remote_stand_used`: laptop + secondary VPS (primary VPS unreachable).
- `ssh_ok`: true for reachable hosts; primary VPS timed out.
- No stand addresses, logins, passwords, or temp paths appear in product files.

## GitHub Delivery

- `github_release`: v0.1.170
- `latest_download_version`: 0.1.170
- `checksum_ok`: true
- `github_one_command_install_ok`: true on reachable hosts

## Commands (redacted)

```bash
ssh <stand-host> "bash -o pipefail -c 'curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60 https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh | bash -s -- -install'"
ssh <stand-host> "<home>/.local/bin/chimera.sh -start"
ssh <stand-host> "<home>/.local/bin/chimera.sh -status"
ssh <stand-host> "<home>/.local/bin/chimera.sh -restart"
ssh <stand-host> "<home>/.local/bin/chimera.sh -stop"
ssh <stand-host> "systemctl --user disable chimera-runtime.service"
ssh <stand-host> "# reinstall, verify unit remains disabled"
ssh <stand-host> "# create fake stale state files, start service, verify cleanup"
ssh <stand-host> "# reboot, verify runtime boot recovery"
```

## Results

| Check | Status |
|-------|--------|
| install_from_github_latest | pass |
| version_checksum_match | pass |
| start_status | pass (laptop), partial-listener-only (fresh VPS) |
| restart | pass (laptop) |
| stop_status | pass |
| reboot_recovery | pass |
| disabled_boot_recovery_preserved | pass |
| stale_publication_recovery | pass |
| port_conflict_recovery | partial / observed |
| primary_vps_ssh | unreachable |

## Verdict

`v0.1.170` is installable from GitHub Latest and recovers cleanly across reboot
and stale state on the reachable stand hosts. Primary VPS remains a stand
blocker and is excluded from closure claims.
