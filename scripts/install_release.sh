#!/usr/bin/env bash
set -euo pipefail

SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHIMERA_HOME="${CHIMERA_HOME:-$HOME/.local/share/chimera}"
DEFAULT_RELEASE_URL="https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera-pq-release.tar.gz"
BUNDLE_SOURCE="${1:-${CHIMERA_RELEASE_ARCHIVE_URL:-$DEFAULT_RELEASE_URL}}"
CHECKSUM_SOURCE="${2:-${CHIMERA_RELEASE_CHECKSUM_FILE:-}}"
ALLOW_LOCAL_SOURCE="${CHIMERA_ALLOW_LOCAL_RELEASE_SOURCE:-0}"
RELEASE_VERSION_FILE=".chimera_release_version"
RELEASE_BUNDLE_SHA_FILE=".chimera_release_bundle.sha256"
INSTALL_LOCAL_BIN_FILE=".chimera_install_local_bin"
RUNTIME_SERVICE_UNIT="${CHIMERA_RUNTIME_SERVICE_UNIT:-chimera-runtime.service}"
NODE_SERVICE_UNIT="${CHIMERA_NODE_SERVICE_UNIT:-chimera-node.service}"
DATAPATH_SERVICE_UNIT="${CHIMERA_DATAPATH_SERVICE_UNIT:-chimera-datapath.service}"
SITE_AUTOWATCH_SERVICE_UNIT="${CHIMERA_SITE_AUTOWATCH_SERVICE_UNIT:-chimera-site-watch.service}"
LEGACY_NODE_COMPAT_SERVICE_UNIT="${LEGACY_NODE_COMPAT_SERVICE_UNIT:-${CHIMERA_LEGACY_NODE_SERVICE_UNIT:-chimera-gateway.service}}"
LEGACY_DATAPATH_COMPAT_SERVICE_UNIT="${LEGACY_DATAPATH_COMPAT_SERVICE_UNIT:-${CHIMERA_LEGACY_DATAPATH_SERVICE_UNIT:-chimera-client.service}}"
SYSTEMD_USER_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
SYSTEMD_USER_DEFAULT_TARGET_WANTS_DIR="$SYSTEMD_USER_DIR/default.target.wants"
CHIMERA_CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/chimera"
APPLICATIONS_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"

sha256_file() {
  local file="${1:?file_required}"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
    return 0
  fi
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
    return 0
  fi
  echo "error: missing sha256 tool: sha256sum or shasum" >&2
  return 1
}

cache_buster_url() {
  local url="${1:?url_required}"
  local stamp="ts=$(date +%s%N 2>/dev/null || date +%s)"
  if [[ "$url" == *"?"* ]]; then
    printf '%s&%s\n' "$url" "$stamp"
  else
    printf '%s?%s\n' "$url" "$stamp"
  fi
}

download_url_to_file() {
  local url="${1:?url_required}"
  local dest="${2:?dest_required}"
  if command -v curl >/dev/null 2>&1; then
    if env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
      curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60 -o "$dest" "$url"
    then
      return 0
    fi
  fi
  if command -v wget >/dev/null 2>&1; then
    if env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
      wget --no-config --tries=3 --timeout=10 --dns-timeout=10 --connect-timeout=10 --read-timeout=60 --waitretry=1 -qO "$dest" "$url"
    then
      return 0
    fi
  fi
  echo "error: need curl or wget to download release" >&2
  return 1
}

resolve_checksum_source() {
  local archive="${1:?archive_required}"
  local checksum="${2:-}"
  if [[ -n "$checksum" && -f "$checksum" ]]; then
    printf '%s\n' "$checksum"
    return 0
  fi
  if [[ -f "${archive}.sha256" ]]; then
    printf '%s\n' "${archive}.sha256"
    return 0
  fi
  echo "error: release checksum is required before archive extraction: $archive" >&2
  return 1
}

verify_checksum_required() {
  local archive="${1:?archive_required}"
  local checksum="${2:?checksum_required}"
  local expected actual
  expected="$(awk '{print $1}' "$checksum" | tr -d '[:space:]')"
  [[ -n "$expected" ]] || {
    echo "error: empty checksum file: $checksum" >&2
    return 1
  }
  actual="$(sha256_file "$archive")"
  if [[ "$actual" != "$expected" ]]; then
    echo "error: release checksum mismatch expected=$expected actual=$actual" >&2
    return 1
  fi
  CHIMERA_RELEASE_BUNDLE_SHA256="$actual"
  export CHIMERA_RELEASE_BUNDLE_SHA256
}

path_exists_or_link() {
  local path="${1:?path_required}"
  [[ -e "$path" || -L "$path" ]]
}

remove_path_if_present() {
  local path="${1:?path_required}"
  path_exists_or_link "$path" || return 0
  rm -rf "$path"
}

resolve_local_bin_default() {
  local recorded_local_bin_file="${CHIMERA_HOME}/${INSTALL_LOCAL_BIN_FILE}"
  if [[ -f "$recorded_local_bin_file" ]]; then
    local recorded_local_bin=""
    recorded_local_bin="$(tr -d '\r' <"$recorded_local_bin_file" 2>/dev/null | head -n 1 | tr -d '\n' || true)"
    if [[ -n "$recorded_local_bin" ]]; then
      printf '%s\n' "$recorded_local_bin"
      return 0
    fi
  fi
  printf '%s\n' "$HOME/.local/bin"
}

LOCAL_BIN="${CHIMERA_LOCAL_BIN:-$(resolve_local_bin_default)}"

tracked_external_state_paths() {
  printf '%s\n' \
    "${XDG_CONFIG_HOME:-$HOME/.config}/chimera" \
    "$SYSTEMD_USER_DIR/$RUNTIME_SERVICE_UNIT" \
    "$SYSTEMD_USER_DIR/$NODE_SERVICE_UNIT" \
    "$SYSTEMD_USER_DIR/$DATAPATH_SERVICE_UNIT" \
    "$SYSTEMD_USER_DIR/$SITE_AUTOWATCH_SERVICE_UNIT" \
    "$SYSTEMD_USER_DIR/$LEGACY_NODE_COMPAT_SERVICE_UNIT" \
    "$SYSTEMD_USER_DIR/$LEGACY_DATAPATH_COMPAT_SERVICE_UNIT" \
    "$SYSTEMD_USER_DEFAULT_TARGET_WANTS_DIR/$RUNTIME_SERVICE_UNIT" \
    "$SYSTEMD_USER_DEFAULT_TARGET_WANTS_DIR/$NODE_SERVICE_UNIT" \
    "$SYSTEMD_USER_DEFAULT_TARGET_WANTS_DIR/$DATAPATH_SERVICE_UNIT" \
    "$SYSTEMD_USER_DEFAULT_TARGET_WANTS_DIR/$SITE_AUTOWATCH_SERVICE_UNIT" \
    "$SYSTEMD_USER_DEFAULT_TARGET_WANTS_DIR/$LEGACY_NODE_COMPAT_SERVICE_UNIT" \
    "$SYSTEMD_USER_DEFAULT_TARGET_WANTS_DIR/$LEGACY_DATAPATH_COMPAT_SERVICE_UNIT" \
    "$APPLICATIONS_DIR/chimera-control-gui.desktop" \
    "$APPLICATIONS_DIR/chimera-control.desktop" \
    "$LOCAL_BIN/chimera" \
    "$LOCAL_BIN/chimera.sh" \
    "$LOCAL_BIN/chimera-sh" \
    "${CHIMERA_HOME}/configs/mesh-node.conf" \
    "$CHIMERA_CACHE_DIR/peer-egress.state" \
    "$CHIMERA_CACHE_DIR/peer-update.state.json"
}

snapshot_external_state() {
  local snapshot_dir="${1:?snapshot_dir_required}"
  local snapshot_root="$snapshot_dir/files"
  local manifest_file="$snapshot_dir/manifest.tsv"
  local path snapshot_path
  mkdir -p "$snapshot_root"
  : >"$manifest_file"
  while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    if path_exists_or_link "$path"; then
      printf 'present\t%s\n' "$path" >>"$manifest_file"
      snapshot_path="$snapshot_root/${path#/}"
      mkdir -p "$(dirname "$snapshot_path")"
      cp -a "$path" "$snapshot_path"
    else
      printf 'missing\t%s\n' "$path" >>"$manifest_file"
    fi
  done < <(tracked_external_state_paths)
}

restore_external_state() {
  local snapshot_dir="${1:-}"
  local manifest_file="$snapshot_dir/manifest.tsv"
  local snapshot_root="$snapshot_dir/files"
  local state path snapshot_path
  [[ -n "$snapshot_dir" && -f "$manifest_file" ]] || return 0
  while IFS=$'\t' read -r state path; do
    [[ -n "$path" ]] || continue
    remove_path_if_present "$path"
    if [[ "$state" == "present" ]]; then
      snapshot_path="$snapshot_root/${path#/}"
      mkdir -p "$(dirname "$path")"
      cp -a "$snapshot_path" "$path"
    fi
  done <"$manifest_file"
}

cleanup_external_state_snapshot() {
  local snapshot_dir="${1:-}"
  [[ -n "$snapshot_dir" && -e "$snapshot_dir" ]] || return 0
  rm -rf "$snapshot_dir"
}

link_launchers() {
  mkdir -p "${LOCAL_BIN}" || return 1
  ln -sfn "${CHIMERA_HOME}/scripts/chimera.sh" "${LOCAL_BIN}/chimera" || return 1
  ln -sfn "${CHIMERA_HOME}/scripts/chimera.sh" "${LOCAL_BIN}/chimera.sh" || return 1
  ln -sfn "${CHIMERA_HOME}/scripts/chimera-sh" "${LOCAL_BIN}/chimera-sh" || return 1
}

restore_previous_release() {
  local backup_home="${1:-}"
  local had_previous="${2:-0}"
  rm -rf "$CHIMERA_HOME"
  if [[ "$had_previous" == "1" && -n "$backup_home" && -d "$backup_home" ]]; then
    mv "$backup_home" "$CHIMERA_HOME"
  fi
}

remove_previous_release_backup() {
  local backup_home="${1:-}"
  [[ -n "$backup_home" && -e "$backup_home" ]] || return 0
  if rm -rf "$backup_home" 2>/dev/null; then
    return 0
  fi
  if command -v sudo >/dev/null 2>&1; then
    if sudo -n rm -rf "$backup_home" 2>/dev/null; then
      return 0
    fi
  fi
  echo "warning: previous release backup cleanup failed; leaving redacted backup directory in place" >&2
  return 0
}

install_prepared_release_tree() {
  local prepared_release="${1:?prepared_release_required}"
  local archive="${2:-}"
  local checksum="${3:-}"
  local parent_dir backup_home="" external_state_snapshot="" had_previous=0

  [[ -d "$prepared_release" ]] || {
    echo "error: prepared release tree not found: $prepared_release" >&2
    return 1
  }
  parent_dir="$(dirname "$CHIMERA_HOME")"
  mkdir -p "$parent_dir"

  if [[ -n "$archive" && -n "$checksum" ]]; then
    mkdir -p "$prepared_release/releases"
    cp -f "$archive" "$prepared_release/releases/chimera-pq-release.tar.gz"
    cp -f "$checksum" "$prepared_release/releases/chimera-pq-release.tar.gz.sha256"
  fi

  chmod +x "$prepared_release/bin/"*
  chmod +x "$prepared_release/scripts/"*.sh
  chmod +x "$prepared_release/bin/chimera-bootstrap" 2>/dev/null || true

  if [[ -e "$CHIMERA_HOME" ]]; then
    backup_home="$(mktemp -d "${parent_dir}/.chimera-previous.XXXXXX")"
    rmdir "$backup_home"
    mv "$CHIMERA_HOME" "$backup_home"
    had_previous=1
  fi
  if ! mv "$prepared_release" "$CHIMERA_HOME"; then
    restore_previous_release "$backup_home" "$had_previous"
    return 1
  fi

  external_state_snapshot="$(mktemp -d "${parent_dir}/.chimera-external.XXXXXX")"
  if ! snapshot_external_state "$external_state_snapshot"; then
    restore_previous_release "$backup_home" "$had_previous"
    cleanup_external_state_snapshot "$external_state_snapshot"
    return 1
  fi

  echo "install: running desktop control setup"
  CHIMERA_RELEASE_VERSION="$(cat "${CHIMERA_HOME}/.chimera_release_version" 2>/dev/null || true)"
  export CHIMERA_RELEASE_VERSION
  local install_rc=0
  bash "${CHIMERA_HOME}/scripts/install_desktop_control.sh" || install_rc=$?
  if [[ "$install_rc" -ne 0 ]]; then
    restore_previous_release "$backup_home" "$had_previous"
    restore_external_state "$external_state_snapshot"
    cleanup_external_state_snapshot "$external_state_snapshot"
    return "$install_rc"
  fi

  if ! link_launchers; then
    restore_previous_release "$backup_home" "$had_previous"
    restore_external_state "$external_state_snapshot"
    cleanup_external_state_snapshot "$external_state_snapshot"
    return 1
  fi
  if [[ "$had_previous" == "1" ]]; then
    rm -f "$CHIMERA_CACHE_DIR/peer-egress.state" "$CHIMERA_CACHE_DIR/peer-update.state.json"
    echo "install: cleared stale runtime state"
  fi
  printf '%s\n' "$LOCAL_BIN" > "$CHIMERA_HOME/$INSTALL_LOCAL_BIN_FILE"
  cleanup_external_state_snapshot "$external_state_snapshot"
  if [[ -n "$backup_home" ]]; then
    remove_previous_release_backup "$backup_home"
  fi
}

install_from_archive() {
  local archive="${1:?archive_required}"
  local checksum
  local extract_tmp
  local cleanup_extract_tmp
  checksum="$(resolve_checksum_source "$archive" "${2:-}")"
  verify_checksum_required "$archive" "$checksum"
  extract_tmp="$(mktemp -d)"
  cleanup_extract_tmp="$(printf '%q' "$extract_tmp")"
  trap "rm -rf -- ${cleanup_extract_tmp}" RETURN
  tar -xzf "$archive" -C "$extract_tmp"
  if [[ ! -d "$extract_tmp/chimera-release" ]]; then
    echo "error: release archive did not contain chimera-release/" >&2
    return 1
  fi
  install_prepared_release_tree "$extract_tmp/chimera-release" "$archive" "$checksum"
}

require_local_source_opt_in() {
  if [[ "$ALLOW_LOCAL_SOURCE" != "1" ]]; then
    echo "error: local release sources are development-only." >&2
    echo "hint: use GitHub Latest one-command install, or set CHIMERA_ALLOW_LOCAL_RELEASE_SOURCE=1 for local packaging debug." >&2
    exit 1
  fi
}

echo "CHIMERA self-contained install"
echo "  source: ${BUNDLE_SOURCE}"

if [[ -d "${BUNDLE_SOURCE}" ]]; then
  require_local_source_opt_in
  echo "install: copying release directory"
  TMP_DIR="$(mktemp -d)"
  trap 'rm -rf "$TMP_DIR"' EXIT
  cp -a "${BUNDLE_SOURCE}" "${TMP_DIR}/chimera-release"
  install_prepared_release_tree "${TMP_DIR}/chimera-release"
elif [[ -f "${BUNDLE_SOURCE}" && "${BUNDLE_SOURCE}" == *.tar.gz ]]; then
  require_local_source_opt_in
  echo "install: extracting tarball"
  install_from_archive "$BUNDLE_SOURCE" "$CHECKSUM_SOURCE"
elif [[ "${BUNDLE_SOURCE}" == https://* || "${BUNDLE_SOURCE}" == http://* ]]; then
  echo "install: downloading from ${BUNDLE_SOURCE}"
  TMP_DIR="$(mktemp -d)"
  trap 'rm -rf "$TMP_DIR"' EXIT
  TMP_ARCHIVE="${TMP_DIR}/chimera-release.tar.gz"
  TMP_CHECKSUM="${TMP_DIR}/chimera-release.tar.gz.sha256"
  install_attempt=""
  install_max_attempts="${CHIMERA_INSTALL_VERIFY_ATTEMPTS:-3}"
  archive_url=""
  checksum_url=""
  for (( install_attempt=1; install_attempt<=install_max_attempts; install_attempt++ )); do
    archive_url="$(cache_buster_url "${BUNDLE_SOURCE}")"
    if [[ "$install_attempt" -gt 1 ]]; then
      echo "install: verification mismatch, retrying attempt ${install_attempt}/${install_max_attempts}" >&2
      sleep "${CHIMERA_INSTALL_VERIFY_RETRY_SEC:-5}"
      rm -f "$TMP_ARCHIVE" "$TMP_CHECKSUM"
    fi
    if ! download_url_to_file "$archive_url" "${TMP_ARCHIVE}"; then
      echo "error: release archive download unavailable" >&2
      exit 2
    fi
    if [[ -n "${CHIMERA_RELEASE_CHECKSUM_URL:-}" ]]; then
      checksum_url="$(cache_buster_url "${CHIMERA_RELEASE_CHECKSUM_URL}")"
      if ! download_url_to_file "$checksum_url" "${TMP_CHECKSUM}"; then
        echo "error: remote release checksum is unavailable" >&2
        exit 2
      fi
      CHECKSUM_SOURCE="$TMP_CHECKSUM"
    elif download_url_to_file "$(cache_buster_url "${BUNDLE_SOURCE}.sha256")" "${TMP_CHECKSUM}" 2>/dev/null; then
      CHECKSUM_SOURCE="$TMP_CHECKSUM"
    else
      echo "error: remote release checksum is unavailable" >&2
      exit 2
    fi
    if install_from_archive "$TMP_ARCHIVE" "$CHECKSUM_SOURCE"; then
      break
    fi
    if [[ "$install_attempt" -eq "$install_max_attempts" ]]; then
      echo "error: release archive verification failed" >&2
      exit 3
    fi
  done
else
  echo "error: cannot find release at ${BUNDLE_SOURCE}" >&2
  echo "usage: ${0} [<path-to-tarball> | <path-to-release-dir> | <url>]" >&2
  exit 1
fi

echo
echo "CHIMERA self-contained install complete."
echo "  version: ${CHIMERA_RELEASE_VERSION:-unknown}"
echo "  home:    ${CHIMERA_HOME}"
echo "  bin:     ${LOCAL_BIN}/chimera"
echo
echo "Quick start:"
echo "  chimera -start"
echo "  chimera -status"
echo "  chimera -stop"
