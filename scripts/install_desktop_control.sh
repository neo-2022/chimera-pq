#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_NODE_ROLE_FILE="$ROOT_DIR/.chimera_install_role"
SYSTEMD_USER_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
SYSTEMD_USER_DEFAULT_TARGET_WANTS_DIR="$SYSTEMD_USER_DIR/default.target.wants"
APPLICATIONS_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
LOCAL_BIN_DIR="${HOME}/.local/bin"
RUNTIME_SERVICE_UNIT="${CHIMERA_RUNTIME_SERVICE_UNIT:-chimera-runtime.service}"
NODE_SERVICE_UNIT="${CHIMERA_NODE_SERVICE_UNIT:-chimera-node.service}"
DATAPATH_SERVICE_UNIT="${CHIMERA_DATAPATH_SERVICE_UNIT:-chimera-datapath.service}"
SITE_AUTOWATCH_SERVICE_UNIT="${CHIMERA_SITE_AUTOWATCH_SERVICE_UNIT:-chimera-site-watch.service}"
LEGACY_NODE_COMPAT_SERVICE_UNIT="${LEGACY_NODE_COMPAT_SERVICE_UNIT:-${CHIMERA_LEGACY_NODE_SERVICE_UNIT:-chimera-gateway.service}}"
LEGACY_DATAPATH_COMPAT_SERVICE_UNIT="${LEGACY_DATAPATH_COMPAT_SERVICE_UNIT:-${CHIMERA_LEGACY_DATAPATH_SERVICE_UNIT:-chimera-client.service}}"
BOOTSTRAP_ENV_FILE="${CHIMERA_BOOTSTRAP_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/mesh_bootstrap.env}"
GITVERS_BOOTSTRAP_URLS_FILE="${CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URLS_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/update_gitvers_bootstrap_urls.list}"
MESH_DISCOVERY_URLS_FILE="${CHIMERA_MESH_NODES_DISCOVERY_URLS_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/mesh_nodes_discovery_urls.list}"
PEER_EGRESS_ENV_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/chimera/peer-egress.env"
PEER_EGRESS_STATE_FILE="${XDG_CACHE_HOME:-$HOME/.cache}/chimera/peer-egress.state"
PEER_UPDATE_STATE_FILE="${XDG_CACHE_HOME:-$HOME/.cache}/chimera/peer-update.state.json"
TRANSPARENT_RUNTIME_ENV_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/chimera/transparent-runtime.env"
CHIMERA_CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/chimera"

normalize_install_node_role() {
  case "${1:-node}" in
    node|weave-node|client|server) printf '%s\n' "node" ;;
    *) printf '%s\n' "node" ;;
  esac
}

INSTALL_NODE_ROLE="$(normalize_install_node_role "${CHIMERA_INSTALL_NODE_ROLE:-node}")"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: missing required command: $1" >&2
    exit 1
  }
}

need_cmd bash

trim_ascii() {
  local value="${1:-}"
  value="${value#"${value%%[![:space:]]*}"}"
  value="${value%"${value##*[![:space:]]}"}"
  printf '%s' "$value"
}

upsert_env_kv() {
  local file="${1:?file_required}"
  local key="${2:?key_required}"
  local value="${3:-}"
  local quoted_value tmp_file replaced=0 line
  [[ "$key" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || {
    echo "error: invalid env key: $key" >&2
    exit 2
  }
  quoted_value="$(shell_quote_env_value "$key" "$value")"
  mkdir -p "$(dirname "$file")"
  touch "$file"
  tmp_file="$(mktemp)"
  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ "$line" == "$key="* ]]; then
      if [[ "$replaced" -eq 0 ]]; then
        printf '%s=%s\n' "$key" "$quoted_value"
        replaced=1
      fi
      continue
    fi
    printf '%s\n' "$line"
  done <"$file" >"$tmp_file"
  if [[ "$replaced" -eq 0 ]]; then
    printf '%s=%s\n' "$key" "$quoted_value" >>"$tmp_file"
  fi
  cat "$tmp_file" >"$file"
  rm -f "$tmp_file"
}

remove_env_kv() {
  local file="${1:?file_required}"
  local key="${2:?key_required}"
  local tmp_file line
  [[ "$key" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || {
    echo "error: invalid env key: $key" >&2
    exit 2
  }
  [[ -f "$file" ]] || return 0
  tmp_file="$(mktemp)"
  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ "$line" == "$key="* ]]; then
      continue
    fi
    printf '%s\n' "$line"
  done <"$file" >"$tmp_file"
  cat "$tmp_file" >"$file"
  rm -f "$tmp_file"
}

shell_quote_env_value() {
  local key="${1:?key_required}"
  local value="${2:-}"
  if printf '%s' "$value" | LC_ALL=C grep -q '[[:cntrl:]]'; then
    echo "error: invalid control character in env value: $key" >&2
    exit 2
  fi
  printf '%q' "$value"
}

write_env_kv() {
  local key="${1:?key_required}"
  local value="${2:-}"
  printf '%s=%s\n' "$key" "$(shell_quote_env_value "$key" "$value")"
}

read_existing_env_kv_from_file() {
  local file="${1:?file_required}"
  local key="${2:?key_required}"
  [[ -f "$file" ]] || return 0
  local raw
  raw="$(awk -v key="$key" 'index($0, key "=") == 1 { print substr($0, length(key) + 2); exit }' \
    "$file" 2>/dev/null || true)"
  decode_existing_env_rhs "$key" "$raw"
}

read_existing_peer_env_kv() {
  local key="${1:?key_required}"
  read_existing_env_kv_from_file "$PEER_EGRESS_ENV_FILE" "$key"
}

read_existing_transparent_env_kv() {
  local key="${1:?key_required}"
  read_existing_env_kv_from_file "$TRANSPARENT_RUNTIME_ENV_FILE" "$key"
}

decode_existing_env_rhs() {
  local key="${1:?key_required}"
  local raw="${2:-}"
  [[ -n "$raw" ]] || return 0
  if printf '%s' "$raw" | LC_ALL=C grep -q '[[:cntrl:]]'; then
    echo "error: invalid control character in existing peer env value: $key" >&2
    exit 2
  fi
  local out="" char rest
  while [[ -n "$raw" ]]; do
    char="${raw:0:1}"
    raw="${raw:1}"
    if [[ "$char" == "\\" ]]; then
      [[ -n "$raw" ]] || {
        echo "error: dangling escape in existing peer env value: $key" >&2
        exit 2
      }
      rest="${raw:0:1}"
      raw="${raw:1}"
      out+="$rest"
    else
      case "$char" in
        '$'|'`'|'|'|'&'|'('|')'|'<'|'>'|'{'|'}')
          echo "error: unsupported shell syntax in existing peer env value: $key" >&2
          exit 2
          ;;
      esac
      out+="$char"
    fi
  done
  printf '%s' "$out"
}

normalize_peer_env_bool() {
  local value="${1:-false}"
  case "$value" in
    true|1|yes)
      printf '%s\n' true
      ;;
    false|0|no|"")
      printf '%s\n' false
      ;;
    *)
      echo "error: invalid boolean value for peer egress config" >&2
      exit 2
      ;;
  esac
}

require_numeric_preserved_id() {
  local key="${1:?key_required}"
  local value="${2:-}"
  [[ "$value" =~ ^[0-9]+$ ]] || {
    echo "error: invalid preserved numeric env value: $key" >&2
    exit 2
  }
}

prefer_existing_env_value() {
  local file="${1:?file_required}"
  local key="${2:?key_required}"
  local fallback="${3:-}"
  local existing
  existing="$(read_existing_env_kv_from_file "$file" "$key")"
  if [[ -n "$existing" ]]; then
    printf '%s\n' "$existing"
    return 0
  fi
  printf '%s\n' "$fallback"
}

bootstrap_env_key_allowed() {
  case "${1:-}" in
    CHIMERA_MESH_NODES_DISCOVERY_URL|CHIMERA_MESH_NODES_DISCOVERY_URLS|CHIMERA_MESH_NODES_DISCOVERY_PUBKEY|CHIMERA_MESH_NODES_DISCOVERY_KEYRING|CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS|CHIMERA_MESH_NAMESPACE|CHIMERA_MESH_LOCAL_NODE|CHIMERA_MESH_POLICY_PAYLOAD|CHIMERA_MESH_TRAFFIC_PROFILE|CHIMERA_MESH_REMOTE_PEER_SPEC|CHIMERA_MESH_EXTRA_PEERS|CHIMERA_MESH_REMOTE_NODE|CHIMERA_MESH_REMOTE_ENDPOINT|CHIMERA_MESH_REMOTE_REGION|CHIMERA_MESH_REMOTE_LOAD_SCORE|CHIMERA_MESH_REMOTE_RELIABILITY_SCORE|CHIMERA_PEER_UPDATE_BASE_URL|CHIMERA_PEER_UPDATE_LISTEN)
      return 0
      ;;
  esac
  return 1
}

bootstrap_env_rhs_is_safe_data() {
  local raw="${1:-}" char
  if printf '%s' "$raw" | LC_ALL=C grep -q '[[:cntrl:]]'; then
    return 1
  fi
  while [[ -n "$raw" ]]; do
    char="${raw:0:1}"
    raw="${raw:1}"
    if [[ "$char" == "\\" ]]; then
      [[ -n "$raw" ]] || return 1
      raw="${raw:1}"
      continue
    fi
    case "$char" in
      '$'|'`'|'|'|'&'|'('|')'|'<'|'>'|'{'|'}'|';')
        return 1
        ;;
    esac
  done
  return 0
}

validate_bootstrap_env_file_for_load() {
  local file="${1:?file_required}"
  local line key rhs
  declare -A seen=()
  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ -z "$(trim_ascii "$line")" || "$line" == \#* ]] && continue
    [[ "$line" == *=* ]] || return 1
    key="${line%%=*}"
    rhs="${line#*=}"
    bootstrap_env_key_allowed "$key" || return 1
    [[ -z "${seen[$key]+x}" ]] || return 1
    seen["$key"]=1
    bootstrap_env_rhs_is_safe_data "$rhs" || return 1
  done <"$file"
}

load_bootstrap_env_if_present() {
  [[ -f "$BOOTSTRAP_ENV_FILE" ]] || return 0
  if ! validate_bootstrap_env_file_for_load "$BOOTSTRAP_ENV_FILE"; then
    echo "error: invalid bootstrap env: $BOOTSTRAP_ENV_FILE" >&2
    return 2
  fi
  local key value
  while IFS= read -r key || [[ -n "$key" ]]; do
    value="$(read_existing_env_kv_from_file "$BOOTSTRAP_ENV_FILE" "$key")"
    if grep -q "^${key}=" "$BOOTSTRAP_ENV_FILE" 2>/dev/null; then
      printf -v "$key" '%s' "$value"
    fi
  done <<'EOF'
CHIMERA_MESH_NODES_DISCOVERY_URL
CHIMERA_MESH_NODES_DISCOVERY_URLS
CHIMERA_MESH_NODES_DISCOVERY_PUBKEY
CHIMERA_MESH_NODES_DISCOVERY_KEYRING
CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS
CHIMERA_MESH_NAMESPACE
CHIMERA_MESH_LOCAL_NODE
CHIMERA_MESH_POLICY_PAYLOAD
CHIMERA_MESH_TRAFFIC_PROFILE
CHIMERA_MESH_REMOTE_PEER_SPEC
CHIMERA_MESH_EXTRA_PEERS
CHIMERA_MESH_REMOTE_NODE
CHIMERA_MESH_REMOTE_ENDPOINT
CHIMERA_MESH_REMOTE_REGION
CHIMERA_MESH_REMOTE_LOAD_SCORE
CHIMERA_MESH_REMOTE_RELIABILITY_SCORE
CHIMERA_PEER_UPDATE_BASE_URL
CHIMERA_PEER_UPDATE_LISTEN
EOF
}

disable_systemd_user_unit_link() {
  local unit="${1:?unit_required}"
  rm -f "$SYSTEMD_USER_DEFAULT_TARGET_WANTS_DIR/$unit"
}

enable_systemd_user_unit_link() {
  local unit="${1:?unit_required}"
  mkdir -p "$SYSTEMD_USER_DEFAULT_TARGET_WANTS_DIR"
  ln -sfn "../$unit" "$SYSTEMD_USER_DEFAULT_TARGET_WANTS_DIR/$unit"
}

systemd_user_unit_link_enabled() {
  local unit="${1:?unit_required}"
  [[ -L "$SYSTEMD_USER_DEFAULT_TARGET_WANTS_DIR/$unit" ]]
}

chimera_install_has_existing_state() {
  [[ -f "$INSTALL_NODE_ROLE_FILE" ]] && return 0
  [[ -f "$SYSTEMD_USER_DIR/$RUNTIME_SERVICE_UNIT" ]] && return 0
  systemd_user_unit_link_enabled "$RUNTIME_SERVICE_UNIT" && return 0
  return 1
}

runtime_boot_recovery_requested() {
  if [[ "${CHIMERA_INSTALL_ENABLE_BOOT_RECOVERY+x}" == "x" ]]; then
    [[ "$(normalize_peer_env_bool "$CHIMERA_INSTALL_ENABLE_BOOT_RECOVERY")" == "true" ]]
    return $?
  fi
  if ! chimera_install_has_existing_state; then
    return 0
  fi
  systemd_user_unit_link_enabled "$RUNTIME_SERVICE_UNIT"
}

install_systemd_user_unit() {
  local unit="${1:?unit_required}"
  local source_file="$ROOT_DIR/deploy/systemd-user/$unit"
  [[ -f "$source_file" ]] || {
    echo "error: missing systemd user unit: $source_file" >&2
    exit 1
  }
  sed "s|__CHIMERA_ROOT__|$ROOT_DIR|g" "$source_file" >"$SYSTEMD_USER_DIR/$unit"
  chmod 0644 "$SYSTEMD_USER_DIR/$unit"
}

best_effort_enable_user_linger() {
  local user_name
  user_name="$(id -un 2>/dev/null || true)"
  [[ -n "$user_name" ]] || return 0
  command -v loginctl >/dev/null 2>&1 || return 0
  if loginctl enable-linger "$user_name" >/dev/null 2>&1; then
    echo "user_linger=enabled"
    return 0
  fi
  local linger_state=""
  linger_state="$(loginctl show-user "$user_name" -p Linger 2>/dev/null | sed -n 's/^Linger=//p' | tr '[:upper:]' '[:lower:]' || true)"
  case "$linger_state" in
    yes|true|1)
      echo "user_linger=present"
      ;;
    *)
      echo "user_linger=unverified"
      ;;
  esac
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

run_control_plane_step() {
  local step="${1:?step_required}"
  shift
  local output="" rc=0
  output="$("$ROOT_DIR/scripts/chimera-control.sh" "$step" "$@" 2>&1)" || rc=$?
  if [[ "$rc" -ne 0 && -n "$output" ]]; then
    printf '%s\n' "$output" >&2
  fi
  return "$rc"
}

persist_bootstrap_env_override_if_present() {
  local key="${1:?key_required}"
  local value="${!key:-}"
  [[ -n "$value" ]] || return 0
  upsert_env_kv "$BOOTSTRAP_ENV_FILE" "$key" "$value"
}

seed_bootstrap_env_value_if_absent() {
  local key="${1:?key_required}"
  local value="${2:-}"
  [[ -n "$value" ]] || return 0
  if grep -q "^${key}=" "$BOOTSTRAP_ENV_FILE" 2>/dev/null; then
    return 0
  fi
  upsert_env_kv "$BOOTSTRAP_ENV_FILE" "$key" "$value"
}

installer_bootstrap_authoritative_peer_source_present() {
  if [[ -f "$BOOTSTRAP_ENV_FILE" ]]; then
    load_bootstrap_env_if_present || return $?
  fi
  [[ -n "${CHIMERA_MESH_REMOTE_PEER_SPEC:-}" ]] && return 0
  [[ -n "${CHIMERA_MESH_EXTRA_PEERS:-}" ]] && return 0
  [[ -n "${CHIMERA_MESH_NODES_DISCOVERY_URL:-}" ]] && return 0
  [[ -n "${CHIMERA_MESH_NODES_DISCOVERY_URLS:-}" ]] && return 0
  [[ -s "$MESH_DISCOVERY_URLS_FILE" ]] && return 0
  if [[ -n "${CHIMERA_MESH_REMOTE_NODE:-}" && -n "${CHIMERA_MESH_REMOTE_ENDPOINT:-}" && -n "${CHIMERA_MESH_REMOTE_REGION:-}" && -n "${CHIMERA_MESH_REMOTE_LOAD_SCORE:-}" && -n "${CHIMERA_MESH_REMOTE_RELIABILITY_SCORE:-}" ]]; then
    return 0
  fi
  return 1
}

validate_bootstrap_seed_contract() {
  if [[ ( -n "${CHIMERA_MESH_NODES_DISCOVERY_URL:-}" || -n "${CHIMERA_MESH_NODES_DISCOVERY_URLS:-}" ) && -z "${CHIMERA_MESH_NODES_DISCOVERY_PUBKEY:-}" && -z "${CHIMERA_MESH_NODES_DISCOVERY_KEYRING:-}" ]]; then
    echo "error: mesh discovery source requires CHIMERA_MESH_NODES_DISCOVERY_PUBKEY or CHIMERA_MESH_NODES_DISCOVERY_KEYRING" >&2
    exit 2
  fi
  if [[ -n "${CHIMERA_MESH_POLICY_PAYLOAD:-}" && -n "${CHIMERA_MESH_TRAFFIC_PROFILE:-}" ]]; then
    echo "error: bootstrap seed must provide exactly one of CHIMERA_MESH_POLICY_PAYLOAD or CHIMERA_MESH_TRAFFIC_PROFILE" >&2
    exit 2
  fi
}

installer_gate_prepare_bootstrap_env() {
  mkdir -p "$(dirname "$BOOTSTRAP_ENV_FILE")"
  touch "$BOOTSTRAP_ENV_FILE"
  remove_env_kv "$BOOTSTRAP_ENV_FILE" "CHIMERA_PEER_EGRESS_TOKEN"
  validate_bootstrap_seed_contract
  if [[ -f "$ROOT_DIR/configs/mesh_bootstrap.env.example" ]]; then
    local discovery_url discovery_urls discovery_pubkey discovery_keyring discovery_probe_timeout
    discovery_url="$(awk -F= '/^CHIMERA_MESH_NODES_DISCOVERY_URL=/{print $2; exit}' "$ROOT_DIR/configs/mesh_bootstrap.env.example" 2>/dev/null || true)"
    discovery_urls="$(awk -F= '/^CHIMERA_MESH_NODES_DISCOVERY_URLS=/{print $2; exit}' "$ROOT_DIR/configs/mesh_bootstrap.env.example" 2>/dev/null || true)"
    discovery_pubkey="$(awk -F= '/^CHIMERA_MESH_NODES_DISCOVERY_PUBKEY=/{print $2; exit}' "$ROOT_DIR/configs/mesh_bootstrap.env.example" 2>/dev/null || true)"
    discovery_keyring="$(awk -F= '/^CHIMERA_MESH_NODES_DISCOVERY_KEYRING=/{print $2; exit}' "$ROOT_DIR/configs/mesh_bootstrap.env.example" 2>/dev/null || true)"
    discovery_probe_timeout="$(awk -F= '/^CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS=/{print $2; exit}' "$ROOT_DIR/configs/mesh_bootstrap.env.example" 2>/dev/null || true)"
    seed_bootstrap_env_value_if_absent "CHIMERA_MESH_NODES_DISCOVERY_URL" "$discovery_url"
    seed_bootstrap_env_value_if_absent "CHIMERA_MESH_NODES_DISCOVERY_URLS" "$discovery_urls"
    seed_bootstrap_env_value_if_absent "CHIMERA_MESH_NODES_DISCOVERY_PUBKEY" "$discovery_pubkey"
    seed_bootstrap_env_value_if_absent "CHIMERA_MESH_NODES_DISCOVERY_KEYRING" "$discovery_keyring"
    seed_bootstrap_env_value_if_absent "CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS" "$discovery_probe_timeout"
  fi
  local bootstrap_override_keys=(
    CHIMERA_MESH_NODES_DISCOVERY_URL
    CHIMERA_MESH_NODES_DISCOVERY_URLS
    CHIMERA_MESH_NODES_DISCOVERY_PUBKEY
    CHIMERA_MESH_NODES_DISCOVERY_KEYRING
    CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS
    CHIMERA_MESH_NAMESPACE
    CHIMERA_MESH_LOCAL_NODE
    CHIMERA_MESH_POLICY_PAYLOAD
    CHIMERA_MESH_TRAFFIC_PROFILE
    CHIMERA_MESH_REMOTE_PEER_SPEC
    CHIMERA_MESH_EXTRA_PEERS
    CHIMERA_MESH_REMOTE_NODE
    CHIMERA_MESH_REMOTE_ENDPOINT
    CHIMERA_MESH_REMOTE_REGION
    CHIMERA_MESH_REMOTE_LOAD_SCORE
    CHIMERA_MESH_REMOTE_RELIABILITY_SCORE
    CHIMERA_PEER_UPDATE_BASE_URL
    CHIMERA_PEER_UPDATE_LISTEN
  )
  local key
  for key in "${bootstrap_override_keys[@]}"; do
    persist_bootstrap_env_override_if_present "$key"
  done
  chmod 600 "$BOOTSTRAP_ENV_FILE"
}

installer_gate_prepare_gitvers_bootstrap_sources() {
  mkdir -p "$(dirname "$GITVERS_BOOTSTRAP_URLS_FILE")"
  if [[ -s "$GITVERS_BOOTSTRAP_URLS_FILE" ]]; then
    chmod 600 "$GITVERS_BOOTSTRAP_URLS_FILE"
    echo "gitvers_bootstrap_sources_seeded=false"
    return 0
  fi
  if [[ ! -f "$ROOT_DIR/configs/update_gitvers_bootstrap_urls.example.list" ]]; then
    echo "error: missing GitVers bootstrap sources template" >&2
    exit 2
  fi
  cp "$ROOT_DIR/configs/update_gitvers_bootstrap_urls.example.list" "$GITVERS_BOOTSTRAP_URLS_FILE"
  chmod 600 "$GITVERS_BOOTSTRAP_URLS_FILE"
  echo "gitvers_bootstrap_sources_seeded=true"
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

upsert_node_config_kv() {
  local file="${1:?file_required}"
  local key="${2:?key_required}"
  local value="${3:-}"
  local key_re tmp_file replaced=0 line
  if printf '%s' "$value" | LC_ALL=C grep -q '[[:cntrl:]]'; then
    echo "error: invalid control character in node config value: $key" >&2
    exit 2
  fi
  key_re="${key//./\\.}"
  mkdir -p "$(dirname "$file")"
  touch "$file"
  tmp_file="$(mktemp)"
  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ "$line" =~ ^[[:space:]]*$key_re[[:space:]]*= ]]; then
      if [[ "$replaced" -eq 0 ]]; then
        printf '%s = %s\n' "$key" "$value"
        replaced=1
      fi
      continue
    fi
    printf '%s\n' "$line"
  done <"$file" >"$tmp_file"
  if [[ "$replaced" -eq 0 ]]; then
    printf '%s = %s\n' "$key" "$value" >>"$tmp_file"
  fi
  cat "$tmp_file" >"$file"
  rm -f "$tmp_file"
}

read_node_config_kv() {
  local file="${1:?file_required}"
  local key="${2:?key_required}"
  awk -F= -v target="$key" '
    {
      raw_key = $1
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", raw_key)
      if (raw_key != target) {
        next
      }
      value = substr($0, index($0, "=") + 1)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
      print value
      exit
    }
  ' "$file" 2>/dev/null || true
}

node_config_value_is_placeholder() {
  local value="${1:-}"
  value="$(trim_ascii "$value")"
  [[ -z "$value" ]] && return 0
  [[ "$value" == "\${"*"}" ]]
}

node_listen_addr_is_auto_like() {
  local value="${1:-}"
  value="$(trim_ascii "$value")"
  case "$value" in
    ""|auto|*:0)
      return 0
      ;;
  esac
  return 1
}

normalize_node_connect_addr() {
  local endpoint="${1:?endpoint_required}"
  case "$endpoint" in
    tcp://*)
      printf '%s\n' "$endpoint"
      ;;
    tcp@*)
      printf 'tcp://%s\n' "${endpoint#tcp@}"
      ;;
    *@*)
      echo "error: unsupported CHIMERA node endpoint transport" >&2
      exit 2
      ;;
    *)
      printf 'tcp://%s\n' "$endpoint"
      ;;
  esac
}

raw_node_endpoint_host_port() {
  local endpoint="${1:?endpoint_required}"
  case "$endpoint" in
    tcp://*)
      printf '%s\n' "${endpoint#tcp://}"
      ;;
    tcp@*)
      printf '%s\n' "${endpoint#tcp@}"
      ;;
    *)
      printf '%s\n' "$endpoint"
      ;;
  esac
}

mesh_peer_spec_endpoint() {
  local peer_spec="${1:-}"
  local node_id="" endpoint="" region="" load_score="" reliability_score="" extra=""
  [[ -n "$peer_spec" ]] || return 1
  IFS='@' read -r node_id endpoint region load_score reliability_score extra <<<"$peer_spec"
  [[ -n "$node_id" && -n "$endpoint" && -n "$region" && -n "$load_score" && -n "$reliability_score" && -z "$extra" ]] || return 1
  printf '%s\n' "$endpoint"
}

derive_node_server_name() {
  local host_part="${1:?host_part_required}"
  local server_name="${CHIMERA_NODE_SERVER_NAME:-${CHIMERA_CARRIER_SERVER_NAME:-${CHIMERA_MESH_REMOTE_SERVER_NAME:-$host_part}}}"
  server_name="${server_name#[}"
  server_name="${server_name%]}"
  if [[ -z "$server_name" ]]; then
    echo "error: CHIMERA node server name is empty" >&2
    exit 2
  fi
  printf '%s\n' "$server_name"
}

legacy_node_listen_addr() {
  if [[ -n "${CHIMERA_GATEWAY_LISTEN_ADDR:-}" ]]; then
    printf '%s\n' "${CHIMERA_GATEWAY_LISTEN_ADDR}"
    return 0
  fi
  if [[ -n "${CHIMERA_GATEWAY_LISTEN_PORT:-}" ]]; then
    printf '%s\n' "${CHIMERA_GATEWAY_LISTEN_PORT}"
    return 0
  fi
  printf '%s\n' ""
}

desired_node_listen_addr() {
  local legacy_listen listen_addr
  legacy_listen="$(legacy_node_listen_addr)"
  listen_addr="${CHIMERA_NODE_LISTEN_ADDR:-${CHIMERA_NODE_PEER_LISTEN_ADDR:-${legacy_listen:-auto}}}"
  if [[ -z "$listen_addr" ]]; then
    listen_addr="auto"
  fi
  printf '%s\n' "$listen_addr"
}

desired_node_peer_egress_listen() {
  local listen_addr
  listen_addr="$(desired_node_listen_addr)"
  case "$listen_addr" in
    ""|auto)
      printf '%s\n' "0.0.0.0:0"
      ;;
    *:*)
      printf '%s\n' "$listen_addr"
      ;;
    *)
      printf '0.0.0.0:%s\n' "$listen_addr"
      ;;
  esac
}

materialize_node_runtime_config() {
  local node_conf="${1:?node_conf_required}"
  local connect_addr="${2:?connect_addr_required}"
  local host_part="${3:?host_part_required}"
  local server_name listen_addr existing_server_name existing_listen_addr final_server_name final_listen_addr
  server_name="$(derive_node_server_name "$host_part")"
  listen_addr="$(desired_node_listen_addr)"
  existing_server_name="$(read_node_config_kv "$node_conf" "carrier.server_name")"
  existing_listen_addr="$(read_node_config_kv "$node_conf" "peer.listen_addr")"
  final_server_name="$server_name"
  final_listen_addr="$listen_addr"
  if [[ -z "${CHIMERA_NODE_SERVER_NAME:-${CHIMERA_CARRIER_SERVER_NAME:-${CHIMERA_MESH_REMOTE_SERVER_NAME:-}}}" ]] \
    && ! node_config_value_is_placeholder "$existing_server_name"; then
    final_server_name="$existing_server_name"
  fi
  if [[ -z "${CHIMERA_NODE_LISTEN_ADDR:-${CHIMERA_NODE_PEER_LISTEN_ADDR:-${CHIMERA_GATEWAY_LISTEN_ADDR:-${CHIMERA_GATEWAY_LISTEN_PORT:-}}}}" ]] \
    && ! node_config_value_is_placeholder "$existing_listen_addr"; then
    final_listen_addr="$existing_listen_addr"
  fi
  upsert_node_config_kv "$node_conf" "node.mode" "mesh-node"
  upsert_node_config_kv "$node_conf" "carrier.addr" "$connect_addr"
  upsert_node_config_kv "$node_conf" "carrier.server_name" "$final_server_name"
  upsert_node_config_kv "$node_conf" "peer.listen_addr" "$final_listen_addr"
}

configure_node_peer_target() {
  local node_conf="$ROOT_DIR/configs/mesh-node.conf"
  local candidate="${CHIMERA_NODE_ENDPOINT:-${CHIMERA_PEER_ENDPOINT:-${CHIMERA_CARRIER_ADDR:-${CHIMERA_MESH_REMOTE_ENDPOINT:-}}}}"
  local -a mesh_nodes_args=()
  if [[ -z "$candidate" ]]; then
    if [[ -f "$BOOTSTRAP_ENV_FILE" ]]; then
      load_bootstrap_env_if_present || exit 2
    fi
    if [[ -z "$candidate" && -n "${CHIMERA_MESH_REMOTE_PEER_SPEC:-}" ]]; then
      candidate="$(mesh_peer_spec_endpoint "${CHIMERA_MESH_REMOTE_PEER_SPEC:-}" 2>/dev/null || true)"
    fi
    if [[ -z "$candidate" && ( -n "${CHIMERA_MESH_NODES_DISCOVERY_URL:-}" || -n "${CHIMERA_MESH_NODES_DISCOVERY_URLS:-}" || -s "$MESH_DISCOVERY_URLS_FILE" ) ]]; then
      if [[ -n "${CHIMERA_MESH_NODES_DISCOVERY_URL:-}" ]]; then
        mesh_nodes_args+=(--discovery-url "$CHIMERA_MESH_NODES_DISCOVERY_URL")
      fi
      if [[ -n "${CHIMERA_MESH_NODES_DISCOVERY_PUBKEY:-}" ]]; then
        mesh_nodes_args+=(--discovery-pubkey "$CHIMERA_MESH_NODES_DISCOVERY_PUBKEY")
      fi
      if [[ -n "${CHIMERA_MESH_NODES_DISCOVERY_KEYRING:-}" ]]; then
        mesh_nodes_args+=(--discovery-keyring "$CHIMERA_MESH_NODES_DISCOVERY_KEYRING")
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
  fi
  if [[ -z "$candidate" ]]; then
    if [[ ! -f "$node_conf" && -f "$ROOT_DIR/configs/mesh-node.example.conf" ]]; then
      cp "$ROOT_DIR/configs/mesh-node.example.conf" "$node_conf"
    fi
    CONFIGURED_PEER_ENDPOINT=""
    echo "peer_config_node_endpoint=none"
    echo "peer_config_carrier_addr=none mode=peer_only"
    return 0
  fi
  local raw_candidate connect_addr
  raw_candidate="$(raw_node_endpoint_host_port "$candidate")"
  connect_addr="$(normalize_node_connect_addr "$candidate")"
  if [[ "$raw_candidate" != *:* ]]; then
    echo "error: invalid CHIMERA node endpoint" >&2
    exit 2
  fi
  local host_part="${raw_candidate%:*}"
  local port_part="${raw_candidate##*:}"
  if [[ -z "$host_part" || ! "$port_part" =~ ^[0-9]+$ || "$port_part" -lt 1 || "$port_part" -gt 65535 ]]; then
    echo "error: invalid CHIMERA node endpoint" >&2
    exit 2
  fi
  if [[ ! -f "$node_conf" && -f "$ROOT_DIR/configs/mesh-node.example.conf" ]]; then
    cp "$ROOT_DIR/configs/mesh-node.example.conf" "$node_conf"
  fi
  if [[ -f "$node_conf" ]]; then
    materialize_node_runtime_config "$node_conf" "$connect_addr" "$host_part"
  fi
  printf '%s\n' "$raw_candidate" > "$ROOT_DIR/configs/chimera_runtime_endpoint.txt"
  CONFIGURED_PEER_ENDPOINT="$raw_candidate"
  echo "peer_config_node_endpoint_present=true"
  echo "peer_config_node_endpoint=<redacted>"
  echo "peer_config_carrier_addr_present=true"
}

configure_peer_egress_env() {
  local mode="${1:?mode_required}"
  local peer_endpoint="${2:-}"
  local invite_token="${3:-}"
  local previous_peer_endpoint previous_peer_listen previous_local_listen previous_pool previous_connections previous_aead previous_allow_pool_transit previous_allow_bound_transit previous_invite_token
  previous_peer_endpoint="$(read_existing_peer_env_kv CHIMERA_PEER_EGRESS_SERVER)"
  previous_peer_listen="$(read_existing_peer_env_kv CHIMERA_PEER_EGRESS_PEER_LISTEN)"
  previous_local_listen="$(read_existing_peer_env_kv CHIMERA_PEER_EGRESS_LOCAL_LISTEN)"
  previous_pool="$(read_existing_peer_env_kv CHIMERA_PEER_EGRESS_POOL)"
  previous_connections="$(read_existing_peer_env_kv CHIMERA_PEER_EGRESS_CONNECTIONS)"
  previous_aead="$(read_existing_peer_env_kv CHIMERA_PEER_EGRESS_AEAD)"
  previous_allow_pool_transit="$(read_existing_peer_env_kv CHIMERA_PEER_EGRESS_ALLOW_POOL_TRANSIT)"
  previous_allow_bound_transit="$(read_existing_peer_env_kv CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT)"
  previous_invite_token="$(read_existing_peer_env_kv CHIMERA_PEER_EGRESS_TOKEN)"
  local peer_listen local_listen
  local pool="${CHIMERA_PEER_EGRESS_POOL:-${previous_pool:-8}}"
  local connections="${CHIMERA_PEER_EGRESS_CONNECTIONS:-${previous_connections:-8}}"
  local aead="${CHIMERA_PEER_EGRESS_AEAD:-${previous_aead:-aes256gcm}}"
  local allow_pool_transit="${CHIMERA_PEER_EGRESS_ALLOW_POOL_TRANSIT:-${previous_allow_pool_transit:-false}}"
  local previous_transit_lane_bindings_file
  previous_transit_lane_bindings_file="$(read_existing_peer_env_kv CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE)"
  local allow_bound_transit
  if [[ -n "${CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT+x}" ]]; then
    allow_bound_transit="${CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT}"
  elif [[ -n "$previous_allow_bound_transit" ]]; then
    allow_bound_transit="$previous_allow_bound_transit"
  elif [[ "$mode" == "node" ]]; then
    allow_bound_transit="true"
  else
    allow_bound_transit="false"
  fi
  local transit_lane_bindings_file="${CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE:-${previous_transit_lane_bindings_file:-}}"
  allow_bound_transit="$(normalize_peer_env_bool "$allow_bound_transit")"
  local desired_peer_listen="${4:-0.0.0.0:0}"
  local desired_local_listen="${5:-127.0.0.1:18135}"
  local effective_node_listen=""
  local reset_legacy_auto_listens=0
  if [[ "$mode" == "node" && -f "$ROOT_DIR/configs/mesh-node.conf" ]]; then
    effective_node_listen="$(read_node_config_kv "$ROOT_DIR/configs/mesh-node.conf" "peer.listen_addr")"
  fi
  if [[ "$mode" == "node" && -z "${CHIMERA_PEER_EGRESS_PEER_LISTEN:-}" && -z "${CHIMERA_PEER_EGRESS_LOCAL_LISTEN:-}" ]] \
    && node_listen_addr_is_auto_like "$effective_node_listen"; then
    reset_legacy_auto_listens=1
  fi
  if [[ "$reset_legacy_auto_listens" -eq 1 ]]; then
    peer_listen="${CHIMERA_PEER_EGRESS_PEER_LISTEN:-$desired_peer_listen}"
    local_listen="${CHIMERA_PEER_EGRESS_LOCAL_LISTEN:-$desired_local_listen}"
  else
    peer_listen="${CHIMERA_PEER_EGRESS_PEER_LISTEN:-${previous_peer_listen:-$desired_peer_listen}}"
    local_listen="${CHIMERA_PEER_EGRESS_LOCAL_LISTEN:-${previous_local_listen:-$desired_local_listen}}"
  fi
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
    invite_token="$previous_invite_token"
  fi
  if [[ -z "$invite_token" ]]; then
    invite_token="$(generate_runtime_token)"
  fi
  if [[ -z "$peer_endpoint" ]]; then
    peer_endpoint="$previous_peer_endpoint"
  fi
  mkdir -p "$(dirname "$PEER_EGRESS_ENV_FILE")"
  mkdir -p "$(dirname "$PEER_EGRESS_STATE_FILE")"
  mkdir -p "$(dirname "$PEER_UPDATE_STATE_FILE")"
  touch "$PEER_EGRESS_ENV_FILE"
  upsert_env_kv "$PEER_EGRESS_ENV_FILE" 'CHIMERA_PEER_EGRESS_MODE' "$mode"
  upsert_env_kv "$PEER_EGRESS_ENV_FILE" 'CHIMERA_PEER_EGRESS_LOCAL_LISTEN' "$local_listen"
  upsert_env_kv "$PEER_EGRESS_ENV_FILE" 'CHIMERA_PEER_EGRESS_PEER_LISTEN' "$peer_listen"
  upsert_env_kv "$PEER_EGRESS_ENV_FILE" 'CHIMERA_PEER_EGRESS_STATE_FILE' "$PEER_EGRESS_STATE_FILE"
  upsert_env_kv "$PEER_EGRESS_ENV_FILE" 'CHIMERA_MESH_PEER_EGRESS_STATE_PATH' "$PEER_EGRESS_STATE_FILE"
  upsert_env_kv "$PEER_EGRESS_ENV_FILE" 'CHIMERA_PEER_UPDATE_STATE_FILE' "$PEER_UPDATE_STATE_FILE"
  if [[ -n "$peer_endpoint" ]]; then
    upsert_env_kv "$PEER_EGRESS_ENV_FILE" 'CHIMERA_PEER_EGRESS_SERVER' "$peer_endpoint"
  else
    remove_env_kv "$PEER_EGRESS_ENV_FILE" 'CHIMERA_PEER_EGRESS_SERVER'
  fi
  upsert_env_kv "$PEER_EGRESS_ENV_FILE" 'CHIMERA_PEER_EGRESS_TOKEN' "$invite_token"
  upsert_env_kv "$PEER_EGRESS_ENV_FILE" 'CHIMERA_PEER_EGRESS_POOL' "$pool"
  upsert_env_kv "$PEER_EGRESS_ENV_FILE" 'CHIMERA_PEER_EGRESS_CONNECTIONS' "$connections"
  upsert_env_kv "$PEER_EGRESS_ENV_FILE" 'CHIMERA_SERVICE_FWMARK' "${CHIMERA_SERVICE_FWMARK:-0x5244}"
  upsert_env_kv "$PEER_EGRESS_ENV_FILE" 'CHIMERA_PEER_EGRESS_AEAD' "$aead"
  upsert_env_kv "$PEER_EGRESS_ENV_FILE" 'CHIMERA_PEER_EGRESS_ALLOW_POOL_TRANSIT' "$allow_pool_transit"
  upsert_env_kv "$PEER_EGRESS_ENV_FILE" 'CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT' "$allow_bound_transit"
  if [[ -n "$transit_lane_bindings_file" ]]; then
    upsert_env_kv "$PEER_EGRESS_ENV_FILE" 'CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE' "$transit_lane_bindings_file"
  else
    remove_env_kv "$PEER_EGRESS_ENV_FILE" 'CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE'
  fi
  chmod 600 "$PEER_EGRESS_ENV_FILE"
  echo "peer_egress_mode=$mode"
  echo "peer_egress_local_listen=$local_listen"
  echo "peer_egress_peer_listen=$peer_listen"
  echo "peer_egress_state_file=$PEER_EGRESS_STATE_FILE"
  echo "peer_egress_allow_pool_transit=$allow_pool_transit"
  echo "peer_egress_allow_bound_transit=$allow_bound_transit"
  if [[ -n "$transit_lane_bindings_file" ]]; then
    echo "peer_egress_transit_lane_bindings_file_configured=true"
  fi
  if [[ -n "$peer_endpoint" ]]; then
    echo "peer_egress_peer_endpoint_present=true"
    echo "peer_egress_peer_endpoint=<redacted>"
  fi
  echo "peer_egress_token_set=true"
}

configure_transparent_runtime_env() {
  local default_uid default_gid exempt_uid transparent_uid transparent_gid
  default_uid="$(id -u)"
  default_gid="$(id -g)"
  exempt_uid="${CHIMERA_REDIRECT_EXEMPT_UID:-$(prefer_existing_env_value "$TRANSPARENT_RUNTIME_ENV_FILE" CHIMERA_REDIRECT_EXEMPT_UID "$default_uid")}"
  transparent_uid="${CHIMERA_TRANSPARENT_RUNTIME_UID:-$(prefer_existing_env_value "$TRANSPARENT_RUNTIME_ENV_FILE" CHIMERA_TRANSPARENT_RUNTIME_UID "$default_uid")}"
  transparent_gid="${CHIMERA_TRANSPARENT_RUNTIME_GID:-$(prefer_existing_env_value "$TRANSPARENT_RUNTIME_ENV_FILE" CHIMERA_TRANSPARENT_RUNTIME_GID "$default_gid")}"
  require_numeric_preserved_id "CHIMERA_REDIRECT_EXEMPT_UID" "$exempt_uid"
  require_numeric_preserved_id "CHIMERA_TRANSPARENT_RUNTIME_UID" "$transparent_uid"
  require_numeric_preserved_id "CHIMERA_TRANSPARENT_RUNTIME_GID" "$transparent_gid"
  local listen="${CHIMERA_TRANSPARENT_TCP_LISTEN:-$(prefer_existing_env_value "$TRANSPARENT_RUNTIME_ENV_FILE" CHIMERA_TRANSPARENT_TCP_LISTEN 127.0.0.1:18134)}"
  local transit_local="${CHIMERA_TRANSPARENT_TCP_TRANSIT_LOCAL:-${CHIMERA_TRANSPARENT_TCP_GATEWAY_LOCAL:-$(prefer_existing_env_value "$TRANSPARENT_RUNTIME_ENV_FILE" CHIMERA_TRANSPARENT_TCP_TRANSIT_LOCAL 127.0.0.1:18135)}}"
  local direct_mode="${CHIMERA_TRANSPARENT_TCP_DIRECT_MODE:-$(prefer_existing_env_value "$TRANSPARENT_RUNTIME_ENV_FILE" CHIMERA_TRANSPARENT_TCP_DIRECT_MODE disabled)}"
  local direct_timeout_ms="${CHIMERA_TRANSPARENT_TCP_DIRECT_TIMEOUT_MS:-$(prefer_existing_env_value "$TRANSPARENT_RUNTIME_ENV_FILE" CHIMERA_TRANSPARENT_TCP_DIRECT_TIMEOUT_MS 1200)}"
  local initial_read_timeout_ms="${CHIMERA_TRANSPARENT_TCP_INITIAL_READ_TIMEOUT_MS:-$(prefer_existing_env_value "$TRANSPARENT_RUNTIME_ENV_FILE" CHIMERA_TRANSPARENT_TCP_INITIAL_READ_TIMEOUT_MS 500)}"
  local redirect_table="${CHIMERA_REDIRECT_TABLE:-$(prefer_existing_env_value "$TRANSPARENT_RUNTIME_ENV_FILE" CHIMERA_REDIRECT_TABLE chimera_redirect)}"
  local redirect_chain="${CHIMERA_REDIRECT_CHAIN:-$(prefer_existing_env_value "$TRANSPARENT_RUNTIME_ENV_FILE" CHIMERA_REDIRECT_CHAIN output)}"
  local service_fwmark="${CHIMERA_REDIRECT_SERVICE_FWMARK:-$(prefer_existing_env_value "$TRANSPARENT_RUNTIME_ENV_FILE" CHIMERA_REDIRECT_SERVICE_FWMARK 0x5244)}"
  local nft_privilege_mode="${CHIMERA_NFT_PRIVILEGE_MODE:-$(prefer_existing_env_value "$TRANSPARENT_RUNTIME_ENV_FILE" CHIMERA_NFT_PRIVILEGE_MODE sudo)}"
  local runner_use_sudo="${CHIMERA_RUNNER_USE_SUDO:-$(prefer_existing_env_value "$TRANSPARENT_RUNTIME_ENV_FILE" CHIMERA_RUNNER_USE_SUDO 1)}"
  mkdir -p "$(dirname "$TRANSPARENT_RUNTIME_ENV_FILE")"
  touch "$TRANSPARENT_RUNTIME_ENV_FILE"
  upsert_env_kv "$TRANSPARENT_RUNTIME_ENV_FILE" 'CHIMERA_TRANSPARENT_BIN' "${CHIMERA_TRANSPARENT_BIN:-$ROOT_DIR/bin/chimera-transparent-tcp}"
  upsert_env_kv "$TRANSPARENT_RUNTIME_ENV_FILE" 'CHIMERA_TRANSPARENT_TCP_LISTEN' "$listen"
  upsert_env_kv "$TRANSPARENT_RUNTIME_ENV_FILE" 'CHIMERA_TRANSPARENT_TCP_TRANSIT_LOCAL' "$transit_local"
  upsert_env_kv "$TRANSPARENT_RUNTIME_ENV_FILE" 'CHIMERA_TRANSPARENT_TCP_DIRECT_MODE' "$direct_mode"
  upsert_env_kv "$TRANSPARENT_RUNTIME_ENV_FILE" 'CHIMERA_TRANSPARENT_TCP_DIRECT_TIMEOUT_MS' "$direct_timeout_ms"
  upsert_env_kv "$TRANSPARENT_RUNTIME_ENV_FILE" 'CHIMERA_TRANSPARENT_TCP_INITIAL_READ_TIMEOUT_MS' "$initial_read_timeout_ms"
  upsert_env_kv "$TRANSPARENT_RUNTIME_ENV_FILE" 'CHIMERA_REDIRECT_TABLE' "$redirect_table"
  upsert_env_kv "$TRANSPARENT_RUNTIME_ENV_FILE" 'CHIMERA_REDIRECT_CHAIN' "$redirect_chain"
  upsert_env_kv "$TRANSPARENT_RUNTIME_ENV_FILE" 'CHIMERA_REDIRECT_SERVICE_FWMARK' "$service_fwmark"
  upsert_env_kv "$TRANSPARENT_RUNTIME_ENV_FILE" 'CHIMERA_REDIRECT_EXEMPT_UID' "${CHIMERA_REDIRECT_EXEMPT_UID:-$exempt_uid}"
  upsert_env_kv "$TRANSPARENT_RUNTIME_ENV_FILE" 'CHIMERA_TRANSPARENT_RUNTIME_UID' "${CHIMERA_TRANSPARENT_RUNTIME_UID:-$transparent_uid}"
  upsert_env_kv "$TRANSPARENT_RUNTIME_ENV_FILE" 'CHIMERA_TRANSPARENT_RUNTIME_GID' "${CHIMERA_TRANSPARENT_RUNTIME_GID:-$transparent_gid}"
  upsert_env_kv "$TRANSPARENT_RUNTIME_ENV_FILE" 'CHIMERA_NFT_PRIVILEGE_MODE' "$nft_privilege_mode"
  upsert_env_kv "$TRANSPARENT_RUNTIME_ENV_FILE" 'CHIMERA_RUNNER_USE_SUDO' "$runner_use_sudo"
  chmod 600 "$TRANSPARENT_RUNTIME_ENV_FILE"
  echo "transparent_runtime_listen=$listen"
  echo "transparent_runtime_transit_local=$transit_local"
}

SYSTEMD_USER_READY=0
USER_LINGER_STATUS=""
BOOT_RECOVERY_STATUS="disk_only"
RUNTIME_SERVICE_ENABLE_STATE="unknown"
RUNTIME_BOOT_RECOVERY_REQUESTED=0
if command -v systemctl >/dev/null 2>&1 && systemctl --user show-environment >/dev/null 2>&1; then
  SYSTEMD_USER_READY=1
fi

mkdir -p "$SYSTEMD_USER_DIR" "$APPLICATIONS_DIR"
mkdir -p "$CHIMERA_CACHE_DIR"
installer_gate_prepare_bootstrap_env
installer_gate_prepare_gitvers_bootstrap_sources
auto_fix_runtime_permissions
run_install_permissions_preflight
configure_node_peer_target
node_peer_listen="$(desired_node_peer_egress_listen)"
bootstrap_authority_present=0
bootstrap_authority_rc=0
if installer_bootstrap_authoritative_peer_source_present; then
  bootstrap_authority_present=1
else
  bootstrap_authority_rc=$?
fi
[[ "$bootstrap_authority_rc" -eq 2 ]] && exit 2
if [[ "$bootstrap_authority_present" -eq 1 && -z "${CONFIGURED_PEER_ENDPOINT:-}" ]]; then
  echo "error: authoritative mesh seed did not resolve a peer endpoint during install" >&2
  exit 2
fi
if [[ -n "${CONFIGURED_PEER_ENDPOINT:-}" ]]; then
  selected_invite_token="$(run_chimera_cli mesh nodes selected-invite-token 2>/dev/null | head -n1 | tr -d '[:space:]' || true)"
  configure_peer_egress_env "node" "$CONFIGURED_PEER_ENDPOINT" "$selected_invite_token" "$node_peer_listen" "127.0.0.1:18135"
else
  selected_invite_token="$(run_chimera_cli mesh nodes selected-invite-token 2>/dev/null | head -n1 | tr -d '[:space:]' || true)"
  configure_peer_egress_env "node" "" "${selected_invite_token:-${CHIMERA_PEER_EGRESS_TOKEN:-}}" "$node_peer_listen" "127.0.0.1:18135"
fi
if [[ "$bootstrap_authority_present" -eq 1 ]]; then
  run_control_plane_step mesh-seed-control-plane --strict
  run_control_plane_step mesh-bind-control-plane --strict
else
  "$ROOT_DIR/scripts/chimera-control.sh" mesh-seed-control-plane --best-effort >/dev/null 2>&1 || true
  "$ROOT_DIR/scripts/chimera-control.sh" mesh-bind-control-plane --best-effort >/dev/null 2>&1 || true
fi
configure_transparent_runtime_env
if runtime_boot_recovery_requested; then
  RUNTIME_BOOT_RECOVERY_REQUESTED=1
fi
install_systemd_user_unit "$NODE_SERVICE_UNIT"
install_systemd_user_unit "$DATAPATH_SERVICE_UNIT"
install_systemd_user_unit "$RUNTIME_SERVICE_UNIT"
if [[ -f "$ROOT_DIR/deploy/systemd-user/$SITE_AUTOWATCH_SERVICE_UNIT" ]]; then
  install_systemd_user_unit "$SITE_AUTOWATCH_SERVICE_UNIT"
fi
rm -f "$SYSTEMD_USER_DIR/$LEGACY_NODE_COMPAT_SERVICE_UNIT" "$SYSTEMD_USER_DIR/$LEGACY_DATAPATH_COMPAT_SERVICE_UNIT"
disable_systemd_user_unit_link "$NODE_SERVICE_UNIT"
disable_systemd_user_unit_link "$DATAPATH_SERVICE_UNIT"
disable_systemd_user_unit_link "$SITE_AUTOWATCH_SERVICE_UNIT"
disable_systemd_user_unit_link "$LEGACY_NODE_COMPAT_SERVICE_UNIT"
disable_systemd_user_unit_link "$LEGACY_DATAPATH_COMPAT_SERVICE_UNIT"
if [[ "$RUNTIME_BOOT_RECOVERY_REQUESTED" -eq 1 ]]; then
  enable_systemd_user_unit_link "$RUNTIME_SERVICE_UNIT"
  USER_LINGER_STATUS="$(best_effort_enable_user_linger || true)"
  if [[ -n "$USER_LINGER_STATUS" ]]; then
    printf '%s\n' "$USER_LINGER_STATUS"
  fi
else
  disable_systemd_user_unit_link "$RUNTIME_SERVICE_UNIT"
  USER_LINGER_STATUS="user_linger=preserved_disabled"
fi
install -m 0644 "$ROOT_DIR/deploy/desktop/chimera-control-gui.desktop" "$APPLICATIONS_DIR/chimera-control-gui.desktop"
sed -i "s|__CHIMERA_ROOT__|$ROOT_DIR|g" "$APPLICATIONS_DIR/chimera-control-gui.desktop"
rm -f "$APPLICATIONS_DIR/chimera-control.desktop"

chmod +x \
  "$ROOT_DIR/scripts/chimera-control.sh" \
  "$ROOT_DIR/scripts/chimera-sh" \
  "$ROOT_DIR/scripts/chimera-update.sh" \
  "$ROOT_DIR/scripts/chimera.sh" \
  "$ROOT_DIR/scripts/chimera-control-tray.sh" \
  "$ROOT_DIR/scripts/chimera-control-launcher.sh"

if [[ "$SYSTEMD_USER_READY" == "1" ]]; then
  install -d -m 0700 "$CHIMERA_CACHE_DIR"
  touch "$CHIMERA_CACHE_DIR/chimera_node.service.log" "$CHIMERA_CACHE_DIR/chimera_datapath.service.log"
  touch "$CHIMERA_CACHE_DIR/chimera_site_watch.service.log"
  systemctl --user disable "$NODE_SERVICE_UNIT" "$DATAPATH_SERVICE_UNIT" "$SITE_AUTOWATCH_SERVICE_UNIT" "$LEGACY_NODE_COMPAT_SERVICE_UNIT" "$LEGACY_DATAPATH_COMPAT_SERVICE_UNIT" >/dev/null 2>&1 || true
  systemctl --user daemon-reload >/dev/null 2>&1 || true
  if [[ "$RUNTIME_BOOT_RECOVERY_REQUESTED" -eq 1 ]]; then
    systemctl --user enable "$RUNTIME_SERVICE_UNIT" >/dev/null 2>&1 || true
  else
    systemctl --user disable "$RUNTIME_SERVICE_UNIT" >/dev/null 2>&1 || true
  fi
  RUNTIME_SERVICE_ENABLE_STATE="$(systemctl --user is-enabled "$RUNTIME_SERVICE_UNIT" 2>/dev/null || true)"
fi

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

if [[ "$SYSTEMD_USER_READY" == "1" ]]; then
  case "$USER_LINGER_STATUS" in
    user_linger=enabled|user_linger=present)
      case "$RUNTIME_SERVICE_ENABLE_STATE" in
        enabled|enabled-runtime|linked|linked-runtime|alias)
          BOOT_RECOVERY_STATUS="armed"
          ;;
        *)
          BOOT_RECOVERY_STATUS="session_only"
          ;;
      esac
      ;;
    *)
      BOOT_RECOVERY_STATUS="session_only"
      ;;
  esac
fi

echo
echo "CHIMERA desktop control installed."
echo "Desktop entry: $APPLICATIONS_DIR/chimera-control-gui.desktop"
echo "User units: $SYSTEMD_USER_DIR/$RUNTIME_SERVICE_UNIT, $SYSTEMD_USER_DIR/$NODE_SERVICE_UNIT, $SYSTEMD_USER_DIR/$DATAPATH_SERVICE_UNIT"
echo "boot_recovery_status=$BOOT_RECOVERY_STATUS"
if [[ "$BOOT_RECOVERY_STATUS" == "armed" ]]; then
  echo "Boot recovery: armed via $RUNTIME_SERVICE_UNIT"
elif [[ "$BOOT_RECOVERY_STATUS" == "session_only" ]]; then
  echo "Boot recovery: live systemd --user is ready, but reboot persistence is unverified"
else
  echo "Boot recovery: units are installed on disk only; live systemd --user session is unavailable in this shell"
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
