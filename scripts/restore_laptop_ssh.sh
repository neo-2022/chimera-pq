#!/usr/bin/env bash
# Temporary stand helper: ensure the laptop accepts an operator-provided SSH key.
# Run on the laptop (not on PC).
set -euo pipefail

PC_PUBKEY="${CHIMERA_RESTORE_LAPTOP_SSH_PUBKEY:-${1:-}}"
if [[ -z "$PC_PUBKEY" ]]; then
  echo "ERROR: pass public key as CHIMERA_RESTORE_LAPTOP_SSH_PUBKEY or arg1." >&2
  exit 2
fi

# Ensure OpenSSH server is installed and running.
if ! command -v sshd >/dev/null 2>&1; then
  if command -v apt-get >/dev/null 2>&1; then
    sudo apt-get update
    sudo apt-get install -y openssh-server
  else
    echo "ERROR: openssh-server not installed and apt not available; install it manually." >&2
    exit 1
  fi
fi

sudo systemctl enable --now ssh

# Open port 22 in ufw if ufw is active; ignore if not present.
if command -v ufw >/dev/null 2>&1 && sudo ufw status 2>/dev/null | grep -q 'Status: active'; then
  sudo ufw allow 22/tcp || true
fi

# Allow current-PC key for the user running this script.
mkdir -p -m 700 "$HOME/.ssh"
touch "$HOME/.ssh/authorized_keys"
chmod 600 "$HOME/.ssh/authorized_keys"
if ! grep -qxF "$PC_PUBKEY" "$HOME/.ssh/authorized_keys"; then
  echo "$PC_PUBKEY" >> "$HOME/.ssh/authorized_keys"
fi

# Quick self-check.
echo "local_ssh_check: $(timeout 5 ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new localhost 'echo ok' 2>&1 || true)"
echo "restore_laptop_ssh: done"
