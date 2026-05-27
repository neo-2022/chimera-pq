# CHIMERA Bidirectional E2E Report

## Scope
- Target: laptop + VPS runtime/e2e channel validation
- Date: 2026-05-22 (Europe/Moscow)

## Laptop
- Host: `art@192.168.31.31`
- Precondition: upstream configured to `91.124.19.180` (ports 22/443/8443)
- Result:
  - `CHIMERA_PATH_PROOF.json`: `status=pass`, `reason=distinct_path_ip`
  - `CHIMERA_E2E_CHANNEL_GATE_LAPTOP.json`: `status=pass`, `reason=channel_audit_and_selected_routes_ok`

## VPS
- Host: `root@91.124.19.180`
- Precondition: upstream configured to local ssh endpoint (`127.0.0.1:22`), `sshpass` installed
- Single-host mode flags:
  - `CHIMERA_PATH_PROOF_ALLOW_SAME_IP=1`
  - `CHIMERA_E2E_ALLOW_WARN_AUDIT=1`
- Result:
  - `CHIMERA_PATH_PROOF.json`: `status=pass`, `reason=same_public_ip_allowed`
  - `CHIMERA_E2E_CHANNEL_GATE_VPS.json`: `status=pass`, `reason=channel_audit_and_selected_routes_ok`

## Notes
- VPS path-proof in single-host topology cannot provide distinct public IP by design; this is now explicitly gated via flag and reflected in reason codes.
- For normal two-host topology, flags are not required and `distinct_path_ip` remains the target outcome.
