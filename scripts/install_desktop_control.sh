#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_NODE_ROLE="${CHIMERA_INSTALL_NODE_ROLE:-client}"
INSTALL_NODE_ROLE_FILE="$ROOT_DIR/.chimera_install_role"
SYSTEMD_USER_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
APPLICATIONS_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
LOCAL_BIN_DIR="${HOME}/.local/bin"
UPSTREAM_ENV_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/chimera/upstream_proxy.env"
PEER_EGRESS_ENV_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/chimera/peer-egress.env"
PEER_EGRESS_STATE_FILE="${XDG_CACHE_HOME:-$HOME/.cache}/chimera/peer-egress.state"
TRANSPARENT_RUNTIME_ENV_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/chimera/transparent-runtime.env"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: missing required command: $1" >&2
    exit 1
  }
}

need_cmd bash

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

generate_runtime_token() {
  if command -v openssl >/dev/null 2>&1; then
    openssl rand -hex 24
    return 0
  fi
  if command -v od >/dev/null 2>&1; then
    od -An -tx1 -N24 /dev/urandom | tr -d ' \n'
    return 0
  fi
  head -c 24 /dev/urandom | base64 | tr -d '=+/\n'
}

installer_gate_prepare_upstream_env() {
  mkdir -p "$(dirname "$UPSTREAM_ENV_FILE")"
  if [[ ! -f "$UPSTREAM_ENV_FILE" && -f "$ROOT_DIR/configs/upstream_proxy.env.example" ]]; then
    cp "$ROOT_DIR/configs/upstream_proxy.env.example" "$UPSTREAM_ENV_FILE"
    return 0
  fi
  if [[ -f "$ROOT_DIR/configs/upstream_proxy.env.example" ]]; then
    local discovery_url discovery_pubkey discovery_probe_timeout
    discovery_url="$(awk -F= '/^CHIMERA_MESH_NODES_DISCOVERY_URL=/{print $2; exit}' "$ROOT_DIR/configs/upstream_proxy.env.example" 2>/dev/null || true)"
    discovery_pubkey="$(awk -F= '/^CHIMERA_MESH_NODES_DISCOVERY_PUBKEY=/{print $2; exit}' "$ROOT_DIR/configs/upstream_proxy.env.example" 2>/dev/null || true)"
    discovery_probe_timeout="$(awk -F= '/^CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS=/{print $2; exit}' "$ROOT_DIR/configs/upstream_proxy.env.example" 2>/dev/null || true)"
    if [[ -n "$discovery_url" ]] && ! grep -q '^CHIMERA_MESH_NODES_DISCOVERY_URL=' "$UPSTREAM_ENV_FILE"; then
      printf '\nCHIMERA_MESH_NODES_DISCOVERY_URL=%s\n' "$discovery_url" >> "$UPSTREAM_ENV_FILE"
    fi
    if [[ -n "$discovery_pubkey" ]] && ! grep -q '^CHIMERA_MESH_NODES_DISCOVERY_PUBKEY=' "$UPSTREAM_ENV_FILE"; then
      printf 'CHIMERA_MESH_NODES_DISCOVERY_PUBKEY=%s\n' "$discovery_pubkey" >> "$UPSTREAM_ENV_FILE"
    fi
    if [[ -n "$discovery_probe_timeout" ]] && ! grep -q '^CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS=' "$UPSTREAM_ENV_FILE"; then
      printf 'CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS=%s\n' "$discovery_probe_timeout" >> "$UPSTREAM_ENV_FILE"
    fi
  fi
}

run_chimera_cli() {
  local bin="$ROOT_DIR/bin/chimera-cli"
  if [[ -x "$bin" ]]; then
    "$bin" "$@"
    return $?
  fi
  if [[ -x "$ROOT_DIR/scripts/chimera-runner.sh" ]]; then
    "$ROOT_DIR/scripts/chimera-runner.sh" cli "$@"
    return $?
  fi
  echo "error: missing chimera-cli runtime binary" >&2
  return 1
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
  if [[ "$INSTALL_NODE_ROLE" == "server" ]]; then
    CONFIGURED_CLIENT_ENDPOINT=""
    echo "peer_config_carrier_addr=none mode=peer_only"
    return 0
  fi
  local client_conf="$ROOT_DIR/configs/client.conf"
  local candidate="${CHIMERA_VPS_ENDPOINT:-${CHIMERA_CARRIER_ADDR:-${CHIMERA_MESH_REMOTE_ENDPOINT:-}}}"
  local -a mesh_nodes_args=()
  if [[ -z "$candidate" ]]; then
    if [[ -f "$UPSTREAM_ENV_FILE" ]]; then
      # shellcheck disable=SC1090
      source "$UPSTREAM_ENV_FILE"
    fi
    if [[ -n "${CHIMERA_MESH_NODES_DISCOVERY_URL:-}" ]]; then
      mesh_nodes_args+=(--discovery-url "$CHIMERA_MESH_NODES_DISCOVERY_URL")
    fi
    if [[ -n "${CHIMERA_MESH_NODES_DISCOVERY_PUBKEY:-}" ]]; then
      mesh_nodes_args+=(--discovery-pubkey "$CHIMERA_MESH_NODES_DISCOVERY_PUBKEY")
    fi
    mesh_nodes_args+=(--probe-timeout-ms "${CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS:-4000}")
    if run_chimera_cli mesh nodes select "${mesh_nodes_args[@]}"; then
      candidate="$(run_chimera_cli mesh nodes selected-endpoint "${mesh_nodes_args[@]}" 2>/dev/null | head -n1 | tr -d '[:space:]' || true)"
    fi
    if [[ -z "$candidate" ]]; then
      local best_node_id=""
      best_node_id="$(run_chimera_cli mesh nodes best "${mesh_nodes_args[@]}" 2>/dev/null | sed -n 's/^node_id=\([^[:space:]]*\).*/\1/p' | head -n1 | tr -d '[:space:]' || true)"
      if [[ -n "$best_node_id" ]]; then
        run_chimera_cli mesh nodes select --id "$best_node_id" "${mesh_nodes_args[@]}" >/dev/null 2>&1 || true
        candidate="$(run_chimera_cli mesh nodes selected-endpoint "${mesh_nodes_args[@]}" 2>/dev/null | head -n1 | tr -d '[:space:]' || true)"
      fi
    fi
  fi
  if [[ -z "$candidate" ]]; then
    CONFIGURED_CLIENT_ENDPOINT=""
    echo "peer_config_carrier_addr=none mode=peer_only"
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
  printf '%s\n' "$candidate" > "$ROOT_DIR/configs/chimera_runtime_endpoint.txt"
  CONFIGURED_CLIENT_ENDPOINT="$candidate"
  echo "peer_config_carrier_addr=$candidate"
}

configure_peer_egress_env() {
  local mode="${1:?mode_required}"
  local server="${2:-}"
  local invite_token="${3:-}"
  local peer_listen="${4:-0.0.0.0:0}"
  local local_listen="${5:-127.0.0.1:18135}"
  local pool="${CHIMERA_PEER_EGRESS_POOL:-8}"
  local connections="${CHIMERA_PEER_EGRESS_CONNECTIONS:-8}"
  local aead="${CHIMERA_PEER_EGRESS_AEAD:-aes256gcm}"
  if [[ ! "$local_listen" == *:* ]]; then
    local_listen="127.0.0.1:${local_listen}"
  fi
  if [[ ! "$peer_listen" == *:* ]]; then
    peer_listen="0.0.0.0:${peer_listen}"
  fi
  if [[ -z "$invite_token" ]]; then
    invite_token="${CHIMERA_PEER_EGRESS_TOKEN:-}"
  fi
  if [[ -z "$invite_token" ]]; then
    invite_token="$(generate_runtime_token)"
  fi
  if [[ "$mode" == "vps" && -z "${CHIMERA_PEER_EGRESS_PEER_LISTEN:-}" ]]; then
    peer_listen="0.0.0.0:0"
  fi
  mkdir -p "$(dirname "$PEER_EGRESS_ENV_FILE")"
  mkdir -p "$(dirname "$PEER_EGRESS_STATE_FILE")"
  {
    printf 'CHIMERA_PEER_EGRESS_MODE=%s\n' "$mode"
    printf 'CHIMERA_PEER_EGRESS_LOCAL_LISTEN=%s\n' "$local_listen"
    printf 'CHIMERA_PEER_EGRESS_PEER_LISTEN=%s\n' "$peer_listen"
    printf 'CHIMERA_PEER_EGRESS_STATE_FILE=%s\n' "$PEER_EGRESS_STATE_FILE"
    printf 'CHIMERA_MESH_PEER_EGRESS_STATE_PATH=%s\n' "$PEER_EGRESS_STATE_FILE"
    if [[ -n "$server" ]]; then
      printf 'CHIMERA_PEER_EGRESS_SERVER=%s\n' "$server"
    fi
    printf 'CHIMERA_PEER_EGRESS_TOKEN=%s\n' "$invite_token"
    printf 'CHIMERA_PEER_EGRESS_POOL=%s\n' "$pool"
    printf 'CHIMERA_PEER_EGRESS_CONNECTIONS=%s\n' "$connections"
    printf 'CHIMERA_PEER_EGRESS_AEAD=%s\n' "$aead"
  } >"$PEER_EGRESS_ENV_FILE"
  chmod 600 "$PEER_EGRESS_ENV_FILE"
  echo "peer_egress_mode=$mode"
  echo "peer_egress_local_listen=$local_listen"
  echo "peer_egress_peer_listen=$peer_listen"
  echo "peer_egress_state_file=$PEER_EGRESS_STATE_FILE"
  if [[ -n "$server" ]]; then
    echo "peer_egress_server=$server"
  fi
  echo "peer_egress_token_set=true"
}

configure_transparent_runtime_env() {
  local exempt_uid
  exempt_uid="$(id -u)"
  local listen="${CHIMERA_TRANSPARENT_TCP_LISTEN:-127.0.0.1:18134}"
  local gateway_local="${CHIMERA_TRANSPARENT_TCP_GATEWAY_LOCAL:-127.0.0.1:18135}"
  mkdir -p "$(dirname "$TRANSPARENT_RUNTIME_ENV_FILE")"
  {
    printf 'CHIMERA_TRANSPARENT_BIN=%s\n' "${CHIMERA_TRANSPARENT_BIN:-$ROOT_DIR/bin/chimera-transparent-tcp}"
    printf 'CHIMERA_TRANSPARENT_TCP_LISTEN=%s\n' "$listen"
    printf 'CHIMERA_TRANSPARENT_TCP_GATEWAY_LOCAL=%s\n' "$gateway_local"
    printf 'CHIMERA_TRANSPARENT_TCP_DIRECT_MODE=%s\n' "${CHIMERA_TRANSPARENT_TCP_DIRECT_MODE:-auto}"
    printf 'CHIMERA_TRANSPARENT_TCP_DIRECT_TIMEOUT_MS=%s\n' "${CHIMERA_TRANSPARENT_TCP_DIRECT_TIMEOUT_MS:-1200}"
    printf 'CHIMERA_TRANSPARENT_TCP_FIRST_RESPONSE_TIMEOUT_MS=%s\n' "${CHIMERA_TRANSPARENT_TCP_FIRST_RESPONSE_TIMEOUT_MS:-1800}"
    printf 'CHIMERA_TRANSPARENT_TCP_INITIAL_READ_TIMEOUT_MS=%s\n' "${CHIMERA_TRANSPARENT_TCP_INITIAL_READ_TIMEOUT_MS:-500}"
    printf 'CHIMERA_REDIRECT_TABLE=%s\n' "${CHIMERA_REDIRECT_TABLE:-chimera_redirect}"
    printf 'CHIMERA_REDIRECT_CHAIN=%s\n' "${CHIMERA_REDIRECT_CHAIN:-output}"
    printf 'CHIMERA_REDIRECT_EXEMPT_UID=%s\n' "${CHIMERA_REDIRECT_EXEMPT_UID:-$exempt_uid}"
    printf 'CHIMERA_TRANSPARENT_RUNTIME_UID=%s\n' "${CHIMERA_TRANSPARENT_RUNTIME_UID:-0}"
    printf 'CHIMERA_TRANSPARENT_RUNTIME_GID=%s\n' "${CHIMERA_TRANSPARENT_RUNTIME_GID:-0}"
    printf 'CHIMERA_RUNNER_USE_SUDO=%s\n' "${CHIMERA_RUNNER_USE_SUDO:-1}"
  } >"$TRANSPARENT_RUNTIME_ENV_FILE"
  chmod 600 "$TRANSPARENT_RUNTIME_ENV_FILE"
  echo "transparent_runtime_listen=$listen"
  echo "transparent_runtime_gateway_local=$gateway_local"
}

SYSTEMD_USER_READY=0
if command -v systemctl >/dev/null 2>&1 && systemctl --user show-environment >/dev/null 2>&1; then
  SYSTEMD_USER_READY=1
fi

mkdir -p "$SYSTEMD_USER_DIR" "$APPLICATIONS_DIR"
installer_gate_prepare_upstream_env
auto_fix_runtime_permissions
run_install_permissions_preflight
configure_client_target
if [[ "$INSTALL_NODE_ROLE" == "server" ]]; then
  selected_invite_token="$(run_chimera_cli mesh nodes selected-invite-token 2>/dev/null | head -n1 | tr -d '[:space:]' || true)"
  configure_peer_egress_env "vps" "" "${selected_invite_token:-${CHIMERA_PEER_EGRESS_TOKEN:-}}" "${CHIMERA_GATEWAY_LISTEN_ADDR:-${CHIMERA_GATEWAY_LISTEN_PORT:-8443}}" "127.0.0.1:18135"
elif [[ -n "${CONFIGURED_CLIENT_ENDPOINT:-}" ]]; then
  selected_invite_token="$(run_chimera_cli mesh nodes selected-invite-token 2>/dev/null | head -n1 | tr -d '[:space:]' || true)"
  configure_peer_egress_env "laptop" "$CONFIGURED_CLIENT_ENDPOINT" "$selected_invite_token" "127.0.0.1:8443"
else
  selected_invite_token="$(run_chimera_cli mesh nodes selected-invite-token 2>/dev/null | head -n1 | tr -d '[:space:]' || true)"
  configure_peer_egress_env "vps" "" "${selected_invite_token:-${CHIMERA_PEER_EGRESS_TOKEN:-}}" "${CHIMERA_GATEWAY_LISTEN_ADDR:-${CHIMERA_GATEWAY_LISTEN_PORT:-8443}}" "127.0.0.1:18135"
fi
configure_transparent_runtime_env
if [[ -f "$PEER_EGRESS_ENV_FILE" ]]; then
  # shellcheck disable=SC1090
  source "$PEER_EGRESS_ENV_FILE"
  upsert_env_kv "$UPSTREAM_ENV_FILE" "CHIMERA_PEER_EGRESS_TOKEN" "${CHIMERA_PEER_EGRESS_TOKEN:-}"
fi
if [[ "$SYSTEMD_USER_READY" == "1" ]]; then
  sed "s|__CHIMERA_ROOT__|$ROOT_DIR|g" \
    "$ROOT_DIR/deploy/systemd-user/chimera-gateway.service" >"$SYSTEMD_USER_DIR/chimera-gateway.service"
  sed "s|__CHIMERA_ROOT__|$ROOT_DIR|g" \
    "$ROOT_DIR/deploy/systemd-user/chimera-client.service" >"$SYSTEMD_USER_DIR/chimera-client.service"
  chmod 0644 "$SYSTEMD_USER_DIR/chimera-gateway.service" "$SYSTEMD_USER_DIR/chimera-client.service"
fi
install -m 0644 "$ROOT_DIR/deploy/desktop/chimera-control-gui.desktop" "$APPLICATIONS_DIR/chimera-control-gui.desktop"
sed -i "s|__CHIMERA_ROOT__|$ROOT_DIR|g" "$APPLICATIONS_DIR/chimera-control-gui.desktop"
rm -f "$APPLICATIONS_DIR/chimera-control.desktop"

chmod +x \
  "$ROOT_DIR/scripts/chimera-control.sh" \
  "$ROOT_DIR/scripts/chimera-sh" \
  "$ROOT_DIR/scripts/chimera.sh" \
  "$ROOT_DIR/scripts/chimera_runtime_bootstrap.sh" \
  "$ROOT_DIR/scripts/chimera-control-tray.sh" \
  "$ROOT_DIR/scripts/chimera-control-launcher.sh"

if [[ -n "${CHIMERA_RELEASE_VERSION:-}" ]]; then
  printf '%s\n' "$CHIMERA_RELEASE_VERSION" > "$ROOT_DIR/.chimera_release_version"
fi

if [[ -n "${CHIMERA_RELEASE_BUNDLE_SHA256:-}" ]]; then
  printf '%s\n' "$CHIMERA_RELEASE_BUNDLE_SHA256" > "$ROOT_DIR/.chimera_release_bundle.sha256"
fi

printf '%s\n' "$INSTALL_NODE_ROLE" > "$INSTALL_NODE_ROLE_FILE"

mkdir -p "$LOCAL_BIN_DIR"
ln -sfn "$ROOT_DIR/scripts/chimera-sh" "$LOCAL_BIN_DIR/chimera-sh"
ln -sfn "$ROOT_DIR/scripts/chimera.sh" "$LOCAL_BIN_DIR/chimera.sh"

if [[ "$SYSTEMD_USER_READY" == "1" ]]; then
  systemctl --user daemon-reload
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
