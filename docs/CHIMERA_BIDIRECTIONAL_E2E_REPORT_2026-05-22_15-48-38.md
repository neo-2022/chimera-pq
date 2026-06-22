# CHIMERA Bidirectional E2E Report

## Scope
- Target: side_b + SIDE_A runtime/e2e channel validation
- Date: 2026-05-22 (Europe/Moscow)

## Side B
- Host: `<stand-user>@<stand-host-a>`
- Precondition: upstream configured to `<stand-host-b-ip>` (ports 22/443/8443)
- Result:
  - `CHIMERA_PATH_PROOF.json`: `status=pass`, `reason=distinct_path_ip`
  - `CHIMERA_E2E_CHANNEL_GATE_SIDE_B.json`: `status=pass`, `reason=channel_audit_and_selected_routes_ok`

## SIDE_A
- Host: `<stand-admin>@<stand-host-b>`
- Precondition: upstream configured to local ssh endpoint (`127.0.0.1:22`), `sshpass` installed
- Single-host mode flags:
  - `CHIMERA_PATH_PROOF_ALLOW_SAME_IP=1`
  - `CHIMERA_E2E_ALLOW_WARN_AUDIT=1`
- Result:
  - `CHIMERA_PATH_PROOF.json`: `status=pass`, `reason=same_public_ip_allowed`
  - `CHIMERA_E2E_CHANNEL_GATE_SIDE_A.json`: `status=pass`, `reason=channel_audit_and_selected_routes_ok`

## Notes
- SIDE_A path-proof in single-host topology cannot provide distinct public IP by design; this is now explicitly gated via flag and reflected in reason codes.
- For normal two-host topology, flags are not required and `distinct_path_ip` remains the target outcome.
