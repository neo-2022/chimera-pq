#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SYSTEMD_USER_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
APPLICATIONS_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
LOCAL_BIN_DIR="${HOME}/.local/bin"
UPSTREAM_ENV_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/chimera/upstream_proxy.env"
SOCKS_PORT_MIN="${SOCKS_PORT_MIN:-12080}"
SOCKS_PORT_MAX="${SOCKS_PORT_MAX:-12180}"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: missing required command: $1" >&2
    exit 1
  }
}

need_cmd bash

is_port_busy() {
  local port="${1:-}"
  [[ -z "$port" ]] && return 1
  ss -ltn "( sport = :$port )" 2>/dev/null | grep -q LISTEN
}

choose_free_port() {
  local p
  for ((p=SOCKS_PORT_MIN; p<=SOCKS_PORT_MAX; p++)); do
    if ! is_port_busy "$p"; then
      echo "$p"
      return 0
    fi
  done
  return 1
}

upsert_env_kv() {
  local file="${1:?file_required}"
  local key="${2:?key_required}"
  local value="${3:-}"
  mkdir -p "$(dirname "$file")"
  touch "$file"
  if grep -qE "^${key}=" "$file"; then
    sed -i "s|^${key}=.*|${key}=${value}|" "$file"
  else
    printf '%s=%s\n' "$key" "$value" >> "$file"
  fi
}

installer_gate_unify_socks_unit() {
  mkdir -p "$(dirname "$UPSTREAM_ENV_FILE")"
  if [[ ! -f "$UPSTREAM_ENV_FILE" && -f "$ROOT_DIR/configs/upstream_proxy.env.example" ]]; then
    cp "$ROOT_DIR/configs/upstream_proxy.env.example" "$UPSTREAM_ENV_FILE"
  fi

  # shellcheck disable=SC1090
  source "$UPSTREAM_ENV_FILE" 2>/dev/null || true
  local selected_port="${CHIMERA_SOCKS_PORT:-12080}"
  if [[ ! "$selected_port" =~ ^[0-9]+$ ]]; then
    selected_port="12080"
  fi
  if is_port_busy "$selected_port"; then
    local auto_port
    auto_port="$(choose_free_port || true)"
    if [[ -n "$auto_port" ]]; then
      selected_port="$auto_port"
    fi
  fi
  upsert_env_kv "$UPSTREAM_ENV_FILE" "CHIMERA_SOCKS_PORT" "$selected_port"

  # Unify legacy CHIMERA SOCKS service to config-driven port selection.
  local legacy_unit="$SYSTEMD_USER_DIR/chimera-socks-tunnel.service"
  if [[ -f "$legacy_unit" ]]; then
    cat > "$legacy_unit" <<'EOF'
[Unit]
Description=CHIMERA SOCKS tunnel to VPS
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
EnvironmentFile=%h/.config/chimera/upstream_proxy.env
ExecStart=/usr/bin/env bash -lc 'exec sshpass -p "$CHIMERA_UPSTREAM_PASS" ssh -o StrictHostKeyChecking=no -o ExitOnForwardFailure=yes -o ServerAliveInterval=30 -o ServerAliveCountMax=3 -N -D 127.0.0.1:${CHIMERA_SOCKS_PORT:-12080} -p ${CHIMERA_UPSTREAM_PORT:-22} ${CHIMERA_UPSTREAM_USER}@${CHIMERA_UPSTREAM_HOST}'
Restart=always
RestartSec=2

[Install]
WantedBy=default.target
EOF
    chmod 0644 "$legacy_unit"
  fi
}

run_install_permissions_preflight() {
  local preflight_out=""
  echo "CHIMERA install gate: permissions preflight (before provision)"
  preflight_out="$("$ROOT_DIR/scripts/chimera-control.sh" preflight-perms --warn-only 2>&1 || true)"
  echo "$preflight_out"

  if echo "$preflight_out" | grep -q "preflight_status=ok"; then
    return 0
  fi

  echo
  echo "CHIMERA install gate: auto-provisioning required runtime permissions (sudo may prompt)"
  "$ROOT_DIR/scripts/chimera-control.sh" grant-perms
  echo
  echo "CHIMERA install gate: permissions preflight (after provision)"
  preflight_out="$("$ROOT_DIR/scripts/chimera-control.sh" preflight-perms --warn-only 2>&1 || true)"
  echo "$preflight_out"
  if ! echo "$preflight_out" | grep -q "preflight_status=ok"; then
    echo "error: CHIMERA install aborted: required permissions are still not satisfied." >&2
    echo "hint: run '$ROOT_DIR/scripts/chimera-control.sh preflight-perms' and fix failed checks, then retry install." >&2
    exit 2
  fi
}

install_pkg_if_missing() {
  local bin_name="$1"
  local pkg_name="$2"
  if command -v "$bin_name" >/dev/null 2>&1 || [[ -x "/usr/sbin/$bin_name" ]]; then
    return 0
  fi
  if command -v apt-get >/dev/null 2>&1; then
    sudo -n apt-get update -y >/dev/null 2>&1 || true
    sudo -n apt-get install -y "$pkg_name" >/dev/null 2>&1 || true
    return 0
  fi
  if command -v dnf >/dev/null 2>&1; then
    sudo -n dnf install -y "$pkg_name" >/dev/null 2>&1 || true
    return 0
  fi
  if command -v yum >/dev/null 2>&1; then
    sudo -n yum install -y "$pkg_name" >/dev/null 2>&1 || true
    return 0
  fi
  if command -v pacman >/dev/null 2>&1; then
    sudo -n pacman -Sy --noconfirm "$pkg_name" >/dev/null 2>&1 || true
    return 0
  fi
}

auto_fix_runtime_permissions() {
  # Ensure nft is present for transparent split auto-redirect paths.
  install_pkg_if_missing nft nftables
}

configure_client_target() {
  local client_conf="$ROOT_DIR/configs/client.conf"
  local candidate="${CHIMERA_VPS_ENDPOINT:-${CHIMERA_CARRIER_ADDR:-${CHIMERA_MESH_REMOTE_ENDPOINT:-}}}"
  local current_addr=""
  if [[ -f "$client_conf" ]]; then
    current_addr="$(awk -F'= ' '$1=="carrier.addr" {print $2; exit}' "$client_conf" 2>/dev/null || true)"
  fi
  if [[ -z "$candidate" && -t 0 ]]; then
    printf 'CHIMERA VPS endpoint (host:port): ' >&2
    IFS= read -r candidate || true
  fi
  if [[ -z "$candidate" ]]; then
    if [[ -z "$current_addr" || "$current_addr" == 203.0.113.10:443 || "$current_addr" == 127.0.0.1:443 ]]; then
      echo "error: CHIMERA_VPS_ENDPOINT is required for the client config. Re-run install with CHIMERA_VPS_ENDPOINT=host:port or provide it at the prompt." >&2
      exit 2
    fi
    return 0
  fi
  if [[ "$candidate" != *:* ]]; then
    echo "error: invalid CHIMERA VPS endpoint: $candidate" >&2
    exit 2
  fi
  local host_part="${candidate%:*}"
  local port_part="${candidate##*:}"
  if [[ -z "$host_part" || ! "$port_part" =~ ^[0-9]+$ || "$port_part" -lt 1 || "$port_part" -gt 65535 ]]; then
    echo "error: invalid CHIMERA VPS endpoint: $candidate" >&2
    exit 2
  fi
  if [[ ! -f "$client_conf" && -f "$ROOT_DIR/configs/client.example.conf" ]]; then
    cp "$ROOT_DIR/configs/client.example.conf" "$client_conf"
  fi
  if [[ -f "$client_conf" ]]; then
    if grep -q '^carrier.addr =' "$client_conf"; then
      sed -i "s#^carrier.addr = .*#carrier.addr = ${candidate}#" "$client_conf"
    else
      printf '\ncarrier.addr = %s\n' "$candidate" >> "$client_conf"
    fi
  fi
  echo "client_config_carrier_addr=$candidate"
}

SYSTEMD_USER_READY=0
if command -v systemctl >/dev/null 2>&1 && systemctl --user show-environment >/dev/null 2>&1; then
  SYSTEMD_USER_READY=1
fi

mkdir -p "$SYSTEMD_USER_DIR" "$APPLICATIONS_DIR"
installer_gate_unify_socks_unit
auto_fix_runtime_permissions
run_install_permissions_preflight
if [[ ! -f "$ROOT_DIR/configs/client.conf" && -f "$ROOT_DIR/configs/client.example.conf" ]]; then
  cp "$ROOT_DIR/configs/client.example.conf" "$ROOT_DIR/configs/client.conf"
fi
configure_client_target
if [[ "$SYSTEMD_USER_READY" == "1" ]]; then
  sed "s|__CHIMERA_ROOT__|$ROOT_DIR|g" \
    "$ROOT_DIR/deploy/systemd-user/chimera-gateway.service" >"$SYSTEMD_USER_DIR/chimera-gateway.service"
  sed "s|__CHIMERA_ROOT__|$ROOT_DIR|g" \
    "$ROOT_DIR/deploy/systemd-user/chimera-client.service" >"$SYSTEMD_USER_DIR/chimera-client.service"
  chmod 0644 "$SYSTEMD_USER_DIR/chimera-gateway.service" "$SYSTEMD_USER_DIR/chimera-client.service"
fi
install -m 0644 "$ROOT_DIR/deploy/desktop/chimera-control-gui.desktop" "$APPLICATIONS_DIR/chimera-control-gui.desktop"
rm -f "$APPLICATIONS_DIR/chimera-control.desktop"

chmod +x \
  "$ROOT_DIR/scripts/chimera-control.sh" \
  "$ROOT_DIR/scripts/chimera-sh" \
  "$ROOT_DIR/scripts/chimera.sh" \
  "$ROOT_DIR/scripts/chimera_runtime_bootstrap.sh" \
  "$ROOT_DIR/scripts/chimera-control-tray.sh" \
  "$ROOT_DIR/scripts/chimera-control-launcher.sh" \
  "$ROOT_DIR/scripts/chimera-socks-watchdog.sh"

if [[ -n "${CHIMERA_RELEASE_VERSION:-}" ]]; then
  printf '%s\n' "$CHIMERA_RELEASE_VERSION" > "$ROOT_DIR/.chimera_release_version"
fi

mkdir -p "$LOCAL_BIN_DIR"
ln -sfn "$ROOT_DIR/scripts/chimera-sh" "$LOCAL_BIN_DIR/chimera-sh"
ln -sfn "$ROOT_DIR/scripts/chimera.sh" "$LOCAL_BIN_DIR/chimera.sh"

if [[ "$SYSTEMD_USER_READY" == "1" ]]; then
  systemctl --user daemon-reload
  if [[ -f "$SYSTEMD_USER_DIR/chimera-socks-tunnel.service" ]]; then
    systemctl --user try-restart chimera-socks-tunnel.service >/dev/null 2>&1 || true
  fi
fi

if [[ -x "$ROOT_DIR/scripts/chimera_runtime_bootstrap.sh" ]]; then
  "$ROOT_DIR/scripts/chimera_runtime_bootstrap.sh" ensure-singbox >/dev/null 2>&1 || true
fi

echo
echo "CHIMERA desktop control installed."
echo "Desktop entry: $APPLICATIONS_DIR/chimera-control-gui.desktop"
if [[ "$SYSTEMD_USER_READY" == "1" ]]; then
  echo "User units: $SYSTEMD_USER_DIR/chimera-gateway.service, $SYSTEMD_USER_DIR/chimera-client.service"
else
  echo "User units: skipped (systemd --user is unavailable in this session)"
fi
echo "Shortcut command: $LOCAL_BIN_DIR/chimera-sh"
echo "Shortcut command: $LOCAL_BIN_DIR/chimera.sh"
echo

echo "UI compatibility:"
echo "  - Wayland: launcher window mode (zenity/kdialog/yad fallback)"
echo "  - X11: tray mode when yad is available, otherwise launcher window mode"
echo "  - Headless/SSH: CLI fallback (status output)"
if ! command -v zenity >/dev/null 2>&1 && ! command -v kdialog >/dev/null 2>&1 && ! command -v yad >/dev/null 2>&1; then
  echo "No GUI dialog backend found; install one of: zenity, kdialog, yad"
fi
echo
echo "Quick start:"
echo "  chimera.sh -start"
echo "  chimera.sh -status"
echo "  chimera.sh -stop"
echo "  chimera.sh -uninstall"
